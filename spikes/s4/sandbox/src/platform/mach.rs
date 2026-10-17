//! Mach messages between the broker and its helpers.
//!
//! The broker checks in a per-broker service name with launchd
//! (`bootstrap_check_in`), and a helper looks it up (`bootstrap_look_up`).
//! Chromium's renderer profile allows exactly this kind of look-up
//! (`MachPortRendezvousServer`). Messages are built and parsed as bytes in
//! safe code (IR1): only the `mach_msg` system call, and reading memory the
//! kernel mapped for an out-of-line region, are `unsafe` (X5, X10). Every
//! received message carries the kernel's audit trailer, which names the
//! sender's process ID (process model R3).
//!
//! Requests and replies follow the usual Mach RPC shape: a request carries a
//! send-once right to the caller's reply port, and the reply consumes it.

use core::ffi::{c_char, c_int};
use std::ffi::CString;
use std::fmt;
use std::time::Duration;

/// A Mach port name in this task's IPC space.
type PortName = u32;
/// A `kern_return_t` or `mach_msg_return_t`.
type Code = c_int;

// <mach/message.h>
const MACH_MSG_TYPE_MOVE_SEND: u32 = 17;
const MACH_MSG_TYPE_MOVE_SEND_ONCE: u32 = 18;
const MACH_MSG_TYPE_COPY_SEND: u32 = 19;
const MACH_MSG_TYPE_MAKE_SEND: u32 = 20;
const MACH_MSG_TYPE_MAKE_SEND_ONCE: u32 = 21;
const MACH_MSGH_BITS_COMPLEX: u32 = 0x8000_0000;
const MACH_MSG_VIRTUAL_COPY: u32 = 1;
const MACH_MSG_OOL_DESCRIPTOR: u32 = 1;
const MACH_SEND_MSG: c_int = 0x1;
const MACH_RCV_MSG: c_int = 0x2;
const MACH_SEND_TIMEOUT: c_int = 0x10;
const MACH_RCV_TIMEOUT: c_int = 0x100;
const MACH_MSG_TRAILER_FORMAT_0: c_int = 0;
const MACH_RCV_TRAILER_AUDIT: c_int = 3;
/// `MACH_RCV_TRAILER_TYPE(FORMAT_0) | MACH_RCV_TRAILER_ELEMENTS(AUDIT)`.
const RECEIVE_AUDIT_TRAILER: c_int =
    ((MACH_MSG_TRAILER_FORMAT_0 & 0xf) << 28) | ((MACH_RCV_TRAILER_AUDIT & 0xf) << 24);
// Return codes, <mach/message.h> and <bootstrap.h>.
const MACH_SEND_INVALID_DEST: Code = 0x1000_0003;
const MACH_SEND_TIMED_OUT: Code = 0x1000_0004;
const MACH_SEND_TOO_LARGE: Code = 0x1000_000e;
const MACH_RCV_TIMED_OUT: Code = 0x1000_4003;
const MACH_RCV_TOO_LARGE: Code = 0x1000_4004;
const MACH_RCV_INVALID_DATA: Code = 0x1000_4008;
const BOOTSTRAP_NOT_PRIVILEGED: Code = 1100;
const BOOTSTRAP_NAME_IN_USE: Code = 1101;
const BOOTSTRAP_UNKNOWN_SERVICE: Code = 1102;
// <mach/port.h>
const MACH_PORT_RIGHT_RECEIVE: u32 = 1;

/// Size of `mach_msg_header_t` in user space.
const HEADER: usize = 24;
/// `mach_msg_body_t` followed by one `mach_msg_ool_descriptor64_t`, packed
/// to 4 bytes as `<mach/message.h>` declares them.
const BODY: usize = 4;
const OOL_DESCRIPTOR: usize = 16;
/// Size of `mach_msg_audit_trailer_t`: type, size, sequence number, a
/// two-word security token and an eight-word audit token.
const AUDIT_TRAILER: usize = 52;
/// Offset of the audit token inside the trailer.
const AUDIT_TOKEN_OFFSET: usize = 20;
/// Bytes in a [`MessageBuffer`].
const BUFFER: usize = 4096;
/// Largest inline payload a [`MessageBuffer`] can receive.
pub const MAX_PAYLOAD: usize = BUFFER - HEADER - BODY - OOL_DESCRIPTOR - AUDIT_TRAILER;

/// `audit_token_t`.
#[repr(C)]
#[derive(Clone, Copy)]
struct AuditToken {
    val: [u32; 8],
}

unsafe extern "C" {
    static bootstrap_port: PortName;
    static mach_task_self_: PortName;
    fn bootstrap_check_in(bp: PortName, service_name: *const c_char, sp: *mut PortName) -> Code;
    fn bootstrap_look_up(bp: PortName, service_name: *const c_char, sp: *mut PortName) -> Code;
    fn mach_msg(
        msg: *mut u8,
        option: c_int,
        send_size: u32,
        rcv_size: u32,
        rcv_name: PortName,
        timeout: u32,
        notify: PortName,
    ) -> Code;
    fn mach_port_allocate(task: PortName, right: u32, name: *mut PortName) -> Code;
    fn mach_port_insert_right(
        task: PortName,
        name: PortName,
        poly: PortName,
        poly_poly: u32,
    ) -> Code;
    fn mach_port_deallocate(task: PortName, name: PortName) -> Code;
    fn mach_port_mod_refs(task: PortName, name: PortName, right: u32, delta: c_int) -> Code;
    fn vm_deallocate(task: PortName, address: usize, size: usize) -> Code;
}

#[link(name = "bsm")]
unsafe extern "C" {
    fn audit_token_to_pid(token: AuditToken) -> c_int;
}

/// A failed Mach or bootstrap call, with its return code.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MachError(pub Code);

impl MachError {
    /// The code's name, for codes this spike expects to see.
    pub fn name(self) -> &'static str {
        match self.0 {
            BOOTSTRAP_NOT_PRIVILEGED => "BOOTSTRAP_NOT_PRIVILEGED",
            BOOTSTRAP_NAME_IN_USE => "BOOTSTRAP_NAME_IN_USE",
            BOOTSTRAP_UNKNOWN_SERVICE => "BOOTSTRAP_UNKNOWN_SERVICE",
            MACH_SEND_INVALID_DEST => "MACH_SEND_INVALID_DEST",
            MACH_SEND_TIMED_OUT => "MACH_SEND_TIMED_OUT",
            MACH_SEND_TOO_LARGE => "MACH_SEND_TOO_LARGE",
            MACH_RCV_TIMED_OUT => "MACH_RCV_TIMED_OUT",
            MACH_RCV_TOO_LARGE => "MACH_RCV_TOO_LARGE",
            MACH_RCV_INVALID_DATA => "MACH_RCV_INVALID_DATA",
            _ => "other",
        }
    }
}

impl fmt::Debug for MachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:#x})", self.name(), self.0)
    }
}

impl fmt::Display for MachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for MachError {}

fn check(code: Code) -> Result<(), MachError> {
    if code == 0 {
        Ok(())
    } else {
        Err(MachError(code))
    }
}

fn task() -> PortName {
    // SAFETY: X8. libSystem initializes `mach_task_self_` before `main`.
    unsafe { mach_task_self_ }
}

fn service_name(service: &str) -> Result<CString, MachError> {
    // launchd names are at most 127 bytes (`name_t` in bootstrap.h).
    if service.len() >= 128 {
        return Err(MachError(BOOTSTRAP_UNKNOWN_SERVICE));
    }
    CString::new(service).map_err(|_| MachError(BOOTSTRAP_UNKNOWN_SERVICE))
}

/// The `mach_msg` option flag for `timeout` and its value in milliseconds.
fn timeout_option(timeout: Option<Duration>, flag: c_int) -> (c_int, u32) {
    match timeout {
        Some(t) => (flag, u32::try_from(t.as_millis()).unwrap_or(u32::MAX)),
        None => (0, 0),
    }
}

/// A send right, released on drop (X4).
#[derive(Debug)]
pub struct SendRight(PortName);

/// A send-once right, consumed by a reply or released on drop (X4).
#[derive(Debug)]
pub struct SendOnceRight(PortName);

/// A receive right, released on drop (X4).
#[derive(Debug)]
pub struct ReceiveRight(PortName);

impl Drop for SendRight {
    fn drop(&mut self) {
        // SAFETY: X4. This value is the right's only owner, and drops it once.
        unsafe { mach_port_deallocate(task(), self.0) };
    }
}

impl Drop for SendOnceRight {
    fn drop(&mut self) {
        // SAFETY: X4. This value is the right's only owner, and drops it once.
        // (A reply consumes the right and forgets this value instead.)
        unsafe { mach_port_deallocate(task(), self.0) };
    }
}

impl Drop for ReceiveRight {
    fn drop(&mut self) {
        // SAFETY: X4. This value is the right's only owner, and drops it once.
        unsafe { mach_port_mod_refs(task(), self.0, MACH_PORT_RIGHT_RECEIVE, -1) };
    }
}

/// Look up `service` in this task's bootstrap namespace.
pub fn look_up(service: &str) -> Result<SendRight, MachError> {
    let name = service_name(service)?;
    let mut port: PortName = 0;
    // SAFETY: X8 (`bootstrap_port`); X1 (`name`); X2 (`port`).
    check(unsafe { bootstrap_look_up(bootstrap_port, name.as_ptr(), &raw mut port) })?;
    Ok(SendRight(port))
}

/// Register `service` with launchd and receive its messages.
pub fn check_in(service: &str) -> Result<ReceiveRight, MachError> {
    let name = service_name(service)?;
    let mut port: PortName = 0;
    // SAFETY: X8 (`bootstrap_port`); X1 (`name`); X2 (`port`).
    check(unsafe { bootstrap_check_in(bootstrap_port, name.as_ptr(), &raw mut port) })?;
    Ok(ReceiveRight(port))
}

/// A message to send.
#[derive(Clone, Copy, Debug)]
pub struct Outgoing<'a> {
    pub id: i32,
    /// Inline bytes, at most [`MAX_PAYLOAD`].
    pub payload: &'a [u8],
    /// Bytes sent out of line, as a virtual copy.
    pub out_of_line: Option<&'a [u8]>,
}

impl<'a> Outgoing<'a> {
    pub fn new(id: i32, payload: &'a [u8]) -> Self {
        Self {
            id,
            payload,
            out_of_line: None,
        }
    }
}

/// Where a message goes, and which right travels with it.
#[derive(Clone, Copy)]
struct Route {
    remote: PortName,
    remote_disposition: u32,
    local: PortName,
    local_disposition: u32,
}

impl ReceiveRight {
    /// A new, unnamed port.
    pub fn new() -> Result<Self, MachError> {
        let mut port: PortName = 0;
        // SAFETY: X2 (`port`).
        check(unsafe { mach_port_allocate(task(), MACH_PORT_RIGHT_RECEIVE, &raw mut port) })?;
        Ok(Self(port))
    }

    /// A send right to this port.
    pub fn make_send(&self) -> Result<SendRight, MachError> {
        // SAFETY: X4. `self` holds the receive right that MAKE_SEND needs, and
        // the new send right is owned by the returned value.
        check(unsafe { mach_port_insert_right(task(), self.0, self.0, MACH_MSG_TYPE_MAKE_SEND) })?;
        Ok(SendRight(self.0))
    }

    /// Receive one message into `buffer`, waiting at most `timeout`.
    pub fn receive(
        &self,
        buffer: &mut MessageBuffer,
        timeout: Option<Duration>,
    ) -> Result<Received, MachError> {
        transact(buffer, None, Some(self.0), timeout)
    }

    /// Answer a request through its send-once `reply` right, then wait for
    /// the next message on this port, in one system call.
    pub fn reply_and_receive(
        &self,
        reply: SendOnceRight,
        message: Outgoing<'_>,
        buffer: &mut MessageBuffer,
        timeout: Option<Duration>,
    ) -> Result<Received, MachError> {
        let route = Route {
            remote: reply.0,
            remote_disposition: MACH_MSG_TYPE_MOVE_SEND_ONCE,
            local: 0,
            local_disposition: 0,
        };
        // Sending consumes the right, whatever the receive then does.
        core::mem::forget(reply);
        transact(buffer, Some((route, message)), Some(self.0), timeout)
    }
}

impl SendRight {
    /// Send `message`, optionally with a send right to `reply`.
    pub fn send(
        &self,
        message: Outgoing<'_>,
        reply: Option<&ReceiveRight>,
        timeout: Option<Duration>,
    ) -> Result<(), MachError> {
        let route = Route {
            remote: self.0,
            remote_disposition: MACH_MSG_TYPE_COPY_SEND,
            local: reply.map_or(0, |r| r.0),
            local_disposition: if reply.is_some() {
                MACH_MSG_TYPE_MAKE_SEND
            } else {
                0
            },
        };
        let mut buffer = MessageBuffer::new();
        transact(&mut buffer, Some((route, message)), None, timeout).map(drop)
    }

    /// Send a request with a send-once right to `reply_port`, and wait for
    /// the reply on it, in one system call.
    pub fn call(
        &self,
        message: Outgoing<'_>,
        reply_port: &ReceiveRight,
        buffer: &mut MessageBuffer,
        timeout: Option<Duration>,
    ) -> Result<Received, MachError> {
        let route = Route {
            remote: self.0,
            remote_disposition: MACH_MSG_TYPE_COPY_SEND,
            local: reply_port.0,
            local_disposition: MACH_MSG_TYPE_MAKE_SEND_ONCE,
        };
        transact(buffer, Some((route, message)), Some(reply_port.0), timeout)
    }
}

/// Send, receive, or both, in one `mach_msg` call. Returns the received
/// message, or an empty one when only sending.
fn transact(
    buffer: &mut MessageBuffer,
    send: Option<(Route, Outgoing<'_>)>,
    receive_on: Option<PortName>,
    timeout: Option<Duration>,
) -> Result<Received, MachError> {
    let mut option = 0;
    let mut send_size = 0;
    if let Some((route, message)) = send {
        send_size = encode(&mut buffer.bytes, route, message)?;
        option |= MACH_SEND_MSG | timeout_option(timeout, MACH_SEND_TIMEOUT).0;
    }
    let (receive_size, receive_name) = match receive_on {
        Some(port) => {
            option |=
                MACH_RCV_MSG | RECEIVE_AUDIT_TRAILER | timeout_option(timeout, MACH_RCV_TIMEOUT).0;
            (u32::try_from(BUFFER).expect("buffer size fits"), port)
        }
        None => (0, 0),
    };
    let milliseconds = timeout_option(timeout, 0).1;
    // SAFETY: X5. The buffer is owned, 8-byte aligned and BUFFER bytes long;
    // it holds the `send_size`-byte message built above, and the kernel
    // writes at most `receive_size` bytes into it. X4: the message names
    // ports this process holds. X10: an out-of-line region in the message
    // points into a live Rust slice for the duration of the call, during
    // which the kernel takes its virtual copy.
    let code = unsafe {
        mach_msg(
            buffer.bytes.as_mut_ptr(),
            option,
            send_size,
            receive_size,
            receive_name,
            milliseconds,
            0,
        )
    };
    check(code)?;
    if receive_on.is_some() {
        parse_received(&buffer.bytes)
    } else {
        Ok(Received::default())
    }
}

/// An owned, aligned buffer for one message (X5).
#[repr(C, align(8))]
#[derive(Debug)]
pub struct MessageBuffer {
    bytes: [u8; BUFFER],
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self { bytes: [0; BUFFER] }
    }
}

impl Default for MessageBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// A right that came with a message, for the receiver to reply on.
#[derive(Debug)]
pub enum Reply {
    Send(SendRight),
    SendOnce(SendOnceRight),
}

/// Memory the kernel mapped into this task for an out-of-line region,
/// deallocated on drop (X10).
#[derive(Debug)]
pub struct OutOfLine {
    address: usize,
    len: usize,
}

impl OutOfLine {
    pub fn bytes(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        let start: *const u8 = core::ptr::with_exposed_provenance(self.address);
        // SAFETY: X10. The kernel mapped `len` readable bytes at `address`
        // for this message; this value owns them until it is dropped, and
        // nothing writes to them meanwhile.
        unsafe { core::slice::from_raw_parts(start, self.len) }
    }
}

impl Drop for OutOfLine {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: X10. Returns the region the kernel mapped for this
            // message, once.
            unsafe { vm_deallocate(task(), self.address, self.len) };
        }
    }
}

/// A received message.
#[derive(Debug, Default)]
pub struct Received {
    pub id: i32,
    pub payload: Vec<u8>,
    /// The sender's process ID, from the kernel's audit trailer.
    pub sender_pid: i32,
    /// The right the sender attached for replies.
    pub reply: Option<Reply>,
    /// Out-of-line bytes, if the message carried a region.
    pub out_of_line: Option<OutOfLine>,
}

/// Build a message in `bytes`. Returns its size.
fn encode(bytes: &mut [u8], route: Route, message: Outgoing<'_>) -> Result<u32, MachError> {
    let padded = message.payload.len().next_multiple_of(4);
    if padded > MAX_PAYLOAD {
        return Err(MachError(MACH_SEND_TOO_LARGE));
    }
    let mut bits = route.remote_disposition | (route.local_disposition << 8);
    let mut offset = HEADER;
    if let Some(region) = message.out_of_line {
        bits |= MACH_MSGH_BITS_COMPLEX;
        let len = u32::try_from(region.len()).map_err(|_| MachError(MACH_SEND_TOO_LARGE))?;
        put_u32(bytes, HEADER, 1); // msgh_descriptor_count
        let address = region.as_ptr().expose_provenance() as u64;
        bytes[HEADER + 4..HEADER + 12].copy_from_slice(&address.to_ne_bytes());
        // Bit-fields, low to high: deallocate (false), copy, pad, type.
        put_u32(
            bytes,
            HEADER + 12,
            (MACH_MSG_VIRTUAL_COPY << 8) | (MACH_MSG_OOL_DESCRIPTOR << 24),
        );
        put_u32(bytes, HEADER + 16, len);
        offset += BODY + OOL_DESCRIPTOR;
    }
    let size = offset + padded;
    let size_u32 = u32::try_from(size).expect("message size fits");
    put_u32(bytes, 0, bits);
    put_u32(bytes, 4, size_u32);
    put_u32(bytes, 8, route.remote);
    put_u32(bytes, 12, route.local);
    put_u32(bytes, 16, 0);
    put_u32(bytes, 20, u32::from_ne_bytes(message.id.to_ne_bytes()));
    bytes[offset..offset + message.payload.len()].copy_from_slice(message.payload);
    bytes[offset + message.payload.len()..size].fill(0);
    Ok(size_u32)
}

/// Parse a received message and its audit trailer. A complex message may
/// carry one out-of-line region and nothing else.
fn parse_received(bytes: &[u8]) -> Result<Received, MachError> {
    let bits = get_u32(bytes, 0);
    let size = get_u32(bytes, 4) as usize;
    let remote = get_u32(bytes, 8);
    let id = i32::from_ne_bytes(get_u32(bytes, 20).to_ne_bytes());
    // Everything that came with the message is ours to release, even if the
    // message is then rejected: take ownership before validating.
    let reply = match bits & 0x1f {
        _ if remote == 0 => None,
        MACH_MSG_TYPE_MOVE_SEND => Some(Reply::Send(SendRight(remote))),
        MACH_MSG_TYPE_MOVE_SEND_ONCE => Some(Reply::SendOnce(SendOnceRight(remote))),
        _ => None,
    };
    let mut payload_start = HEADER;
    let mut out_of_line = None;
    if bits & MACH_MSGH_BITS_COMPLEX != 0 {
        let count = get_u32(bytes, HEADER);
        let kind = get_u32(bytes, HEADER + 12) >> 24;
        if count >= 1 && kind == MACH_MSG_OOL_DESCRIPTOR {
            let address = u64::from_ne_bytes(
                bytes[HEADER + 4..HEADER + 12]
                    .try_into()
                    .expect("eight bytes"),
            );
            out_of_line = Some(OutOfLine {
                address: usize::try_from(address).map_err(|_| MachError(MACH_RCV_INVALID_DATA))?,
                len: get_u32(bytes, HEADER + 16) as usize,
            });
        }
        if count != 1 || kind != MACH_MSG_OOL_DESCRIPTOR {
            return Err(MachError(MACH_RCV_INVALID_DATA));
        }
        payload_start += BODY + OOL_DESCRIPTOR;
    }
    let trailer = size.next_multiple_of(4);
    if size < payload_start
        || trailer + AUDIT_TRAILER > bytes.len()
        || (get_u32(bytes, trailer + 4) as usize) < AUDIT_TRAILER
    {
        return Err(MachError(MACH_RCV_INVALID_DATA));
    }
    let mut token = AuditToken { val: [0; 8] };
    for (i, word) in token.val.iter_mut().enumerate() {
        *word = get_u32(bytes, trailer + AUDIT_TOKEN_OFFSET + 4 * i);
    }
    // SAFETY: a pure function of its by-value argument.
    let sender_pid = unsafe { audit_token_to_pid(token) };
    Ok(Received {
        id,
        payload: bytes[payload_start..size].to_vec(),
        sender_pid,
        reply,
        out_of_line,
    })
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_ne_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Option<Duration> = Some(Duration::from_secs(1));

    fn own_pid() -> i32 {
        i32::try_from(std::process::id()).expect("pid fits")
    }

    #[test]
    fn a_message_round_trips_through_a_port_with_the_senders_pid() {
        let port = ReceiveRight::new().expect("allocate");
        let send = port.make_send().expect("make send");
        let reply = ReceiveRight::new().expect("allocate reply port");
        send.send(Outgoing::new(7, b"hello"), Some(&reply), SECOND)
            .expect("send");
        let mut buffer = MessageBuffer::new();
        let message = port.receive(&mut buffer, SECOND).expect("receive");
        assert_eq!(message.id, 7);
        // The payload is padded to four bytes.
        assert_eq!(&message.payload[..5], b"hello");
        assert_eq!(message.payload.len(), 8);
        assert_eq!(message.sender_pid, own_pid());
        // The attached right answers on `reply`.
        let Some(Reply::Send(answer)) = message.reply else {
            panic!("expected a send right");
        };
        answer
            .send(Outgoing::new(8, b"ok"), None, SECOND)
            .expect("reply");
        let back = reply.receive(&mut buffer, SECOND).expect("receive reply");
        assert_eq!((back.id, &back.payload[..2]), (8, &b"ok"[..]));
        assert!(back.reply.is_none());
    }

    #[test]
    fn a_call_is_answered_through_its_send_once_right_with_out_of_line_bytes() {
        let server = ReceiveRight::new().expect("allocate");
        let to_server = server.make_send().expect("make send");
        let data: Vec<u8> = (0..300_000_u32).map(|i| (i % 251) as u8).collect();
        let expected: u64 = data.iter().map(|&b| u64::from(b)).sum();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut buffer = MessageBuffer::new();
                let request = server.receive(&mut buffer, SECOND).expect("request");
                let sum: u64 = request
                    .out_of_line
                    .as_ref()
                    .expect("an out-of-line region")
                    .bytes()
                    .iter()
                    .map(|&b| u64::from(b))
                    .sum();
                let Some(Reply::SendOnce(reply)) = request.reply else {
                    panic!("expected a send-once right");
                };
                // Reply, then wait briefly for a next request that never comes.
                let error = server
                    .reply_and_receive(
                        reply,
                        Outgoing::new(request.id + 1, &sum.to_ne_bytes()),
                        &mut buffer,
                        Some(Duration::from_millis(10)),
                    )
                    .expect_err("no second request");
                assert_eq!(error.name(), "MACH_RCV_TIMED_OUT");
            });
            let reply_port = ReceiveRight::new().expect("reply port");
            let mut buffer = MessageBuffer::new();
            let message = Outgoing {
                id: 40,
                payload: b"sum",
                out_of_line: Some(&data),
            };
            let answer = to_server
                .call(message, &reply_port, &mut buffer, SECOND)
                .expect("call");
            assert_eq!(answer.id, 41);
            assert_eq!(answer.payload[..8], expected.to_ne_bytes());
            assert_eq!(answer.sender_pid, own_pid());
        });
    }

    #[test]
    fn receiving_times_out() {
        let port = ReceiveRight::new().expect("allocate");
        let mut buffer = MessageBuffer::new();
        let error = port
            .receive(&mut buffer, Some(Duration::from_millis(10)))
            .expect_err("nothing was sent");
        assert_eq!(error.name(), "MACH_RCV_TIMED_OUT");
    }

    #[test]
    fn unknown_services_are_reported_by_code() {
        let error = look_up("org.axiom.s4.no-such-service").expect_err("unknown");
        assert_eq!(error.name(), "BOOTSTRAP_UNKNOWN_SERVICE");
    }
}

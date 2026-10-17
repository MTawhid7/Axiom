//! Mach messages between the broker and its helpers.
//!
//! The broker checks in a per-broker service name with launchd
//! (`bootstrap_check_in`), and a helper looks it up (`bootstrap_look_up`).
//! Chromium's renderer profile allows exactly this kind of look-up
//! (`MachPortRendezvousServer`). Messages are built and parsed as bytes in
//! safe code (IR1): only the `mach_msg` system call itself is `unsafe` (X5).
//! Every received message carries the kernel's audit trailer, which names the
//! sender's process ID (process model R3).

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
const MACH_MSG_TYPE_COPY_SEND: u32 = 19;
const MACH_MSG_TYPE_MAKE_SEND: u32 = 20;
const MACH_MSGH_BITS_COMPLEX: u32 = 0x8000_0000;
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
/// Size of `mach_msg_audit_trailer_t`: type, size, sequence number, a
/// two-word security token and an eight-word audit token.
const AUDIT_TRAILER: usize = 52;
/// Offset of the audit token inside the trailer.
const AUDIT_TOKEN_OFFSET: usize = 20;
/// Largest payload a [`MessageBuffer`] can receive.
pub const MAX_PAYLOAD: usize = 4096 - HEADER - AUDIT_TRAILER;

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

/// A send right, released on drop (X4).
#[derive(Debug)]
pub struct SendRight(PortName);

/// A receive right, released on drop (X4).
#[derive(Debug)]
pub struct ReceiveRight(PortName);

impl Drop for SendRight {
    fn drop(&mut self) {
        // SAFETY: X4. This value is the right's only owner, and drops it once.
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
        let mut option = MACH_RCV_MSG | RECEIVE_AUDIT_TRAILER;
        let milliseconds = timeout.map_or(0, milliseconds);
        if timeout.is_some() {
            option |= MACH_RCV_TIMEOUT;
        }
        let capacity = u32::try_from(buffer.bytes.len()).expect("buffer size fits");
        // SAFETY: X5. The buffer is owned, 8-byte aligned and `capacity` bytes
        // long; the kernel writes at most that much.
        let code = unsafe {
            mach_msg(
                buffer.bytes.as_mut_ptr(),
                option,
                0,
                capacity,
                self.0,
                milliseconds,
                0,
            )
        };
        check(code)?;
        parse_received(&buffer.bytes)
    }
}

impl SendRight {
    /// Send `payload` with message ID `id`, optionally with a send right to
    /// `reply` for the receiver to answer on.
    pub fn send(
        &self,
        id: i32,
        payload: &[u8],
        reply: Option<&ReceiveRight>,
        timeout: Option<Duration>,
    ) -> Result<(), MachError> {
        let mut buffer = MessageBuffer::new();
        let size = encode_simple(&mut buffer.bytes, self.0, reply.map(|r| r.0), id, payload)?;
        let mut option = MACH_SEND_MSG;
        let milliseconds = timeout.map_or(0, milliseconds);
        if timeout.is_some() {
            option |= MACH_SEND_TIMEOUT;
        }
        // SAFETY: X5. The buffer is owned, 8-byte aligned and holds the
        // `size`-byte message built above. X4: the message names ports this
        // process holds (`self` and the optional receive right).
        check(unsafe {
            mach_msg(
                buffer.bytes.as_mut_ptr(),
                option,
                size,
                0,
                0,
                milliseconds,
                0,
            )
        })
    }
}

fn milliseconds(timeout: Duration) -> u32 {
    u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX)
}

/// An owned, aligned buffer for one message (X5).
#[repr(C, align(8))]
#[derive(Debug)]
pub struct MessageBuffer {
    bytes: [u8; 4096],
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self { bytes: [0; 4096] }
    }
}

impl Default for MessageBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// A received message.
#[derive(Debug)]
pub struct Received {
    pub id: i32,
    pub payload: Vec<u8>,
    /// The sender's process ID, from the kernel's audit trailer.
    pub sender_pid: i32,
    /// A send right the sender attached for replies.
    pub reply: Option<SendRight>,
}

/// Build a simple (non-complex) message in `bytes`. Returns its size.
fn encode_simple(
    bytes: &mut [u8],
    remote: PortName,
    local: Option<PortName>,
    id: i32,
    payload: &[u8],
) -> Result<u32, MachError> {
    let padded = payload.len().next_multiple_of(4);
    if padded > MAX_PAYLOAD {
        return Err(MachError(MACH_SEND_TOO_LARGE));
    }
    let size = HEADER + padded;
    let bits = MACH_MSG_TYPE_COPY_SEND | (local.map_or(0, |_| MACH_MSG_TYPE_MAKE_SEND) << 8);
    let size_u32 = u32::try_from(size).expect("message size fits");
    put_u32(bytes, 0, bits);
    put_u32(bytes, 4, size_u32);
    put_u32(bytes, 8, remote);
    put_u32(bytes, 12, local.unwrap_or(0));
    put_u32(bytes, 16, 0);
    put_u32(bytes, 20, u32::from_ne_bytes(id.to_ne_bytes()));
    bytes[HEADER..HEADER + payload.len()].copy_from_slice(payload);
    bytes[HEADER + payload.len()..size].fill(0);
    Ok(size_u32)
}

/// Parse a received message and its audit trailer. Complex messages are not
/// expected here, and are rejected.
fn parse_received(bytes: &[u8]) -> Result<Received, MachError> {
    let bits = get_u32(bytes, 0);
    let size = get_u32(bytes, 4) as usize;
    let remote = get_u32(bytes, 8);
    let id = i32::from_ne_bytes(get_u32(bytes, 20).to_ne_bytes());
    // A right that came with the message is ours to release, even if the
    // message is then rejected.
    let reply =
        (remote != 0 && (bits & 0x1f) == MACH_MSG_TYPE_MOVE_SEND).then_some(SendRight(remote));
    let trailer = size.next_multiple_of(4);
    if bits & MACH_MSGH_BITS_COMPLEX != 0 || size < HEADER || trailer + AUDIT_TRAILER > bytes.len()
    {
        return Err(MachError(MACH_RCV_INVALID_DATA));
    }
    let trailer_size = get_u32(bytes, trailer + 4) as usize;
    if trailer_size < AUDIT_TRAILER {
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
        payload: bytes[HEADER..size].to_vec(),
        sender_pid,
        reply,
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

    #[test]
    fn a_message_round_trips_through_a_port_with_the_senders_pid() {
        let port = ReceiveRight::new().expect("allocate");
        let send = port.make_send().expect("make send");
        let reply = ReceiveRight::new().expect("allocate reply port");
        send.send(7, b"hello", Some(&reply), Some(Duration::from_secs(1)))
            .expect("send");
        let mut buffer = MessageBuffer::new();
        let message = port
            .receive(&mut buffer, Some(Duration::from_secs(1)))
            .expect("receive");
        assert_eq!(message.id, 7);
        // The payload is padded to four bytes.
        assert_eq!(&message.payload[..5], b"hello");
        assert_eq!(message.payload.len(), 8);
        assert_eq!(
            message.sender_pid,
            i32::try_from(std::process::id()).expect("pid fits")
        );
        // The attached right answers on `reply`.
        let answer = message.reply.expect("a reply right");
        answer
            .send(8, b"ok", None, Some(Duration::from_secs(1)))
            .expect("reply");
        let back = reply
            .receive(&mut buffer, Some(Duration::from_secs(1)))
            .expect("receive reply");
        assert_eq!((back.id, &back.payload[..2]), (8, &b"ok"[..]));
        assert!(back.reply.is_none());
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

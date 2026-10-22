//! Mach ports and messages: S10's transport, a message carrying port rights
//! and at most one out-of-line region, with what S16-Q2 asks about: a
//! received message held raw until it is parsed, destroyed or deliberately
//! leaked; a failed send's message, which the kernel hands back with its
//! rights (the SDK's "pseudo-receive", `mach/mach.h`); queue limits;
//! no-senders notifications; and how much of the port space is in use.

use std::ffi::{CString, c_char};
use std::fmt;
use std::time::Duration;

pub(crate) type PortName = u32;

const MACH_PORT_RIGHT_SEND: u32 = 0;
const MACH_PORT_RIGHT_RECEIVE: u32 = 1;
/// `MACH_PORT_DEAD`.
pub(crate) const DEAD: PortName = !0;
const MACH_MSG_TYPE_MOVE_SEND: u32 = 17;
const MACH_MSG_TYPE_COPY_SEND: u32 = 19;
const MACH_MSG_TYPE_MAKE_SEND: u32 = 20;
const MACH_MSG_TYPE_MAKE_SEND_ONCE: u32 = 21;
const MACH_MSGH_BITS_COMPLEX: u32 = 0x8000_0000;
const MACH_SEND_MSG: i32 = 0x1;
const MACH_RCV_MSG: i32 = 0x2;
const MACH_SEND_TIMEOUT: i32 = 0x10;
const MACH_RCV_TIMEOUT: i32 = 0x100;
const MACH_MSG_PORT_DESCRIPTOR: u32 = 0;
const MACH_MSG_OOL_DESCRIPTOR: u32 = 1;
const MACH_MSG_VIRTUAL_COPY: u32 = 1;
/// `MACH_NOTIFY_FIRST` (0100) + 006.
pub(crate) const NOTIFY_NO_SENDERS: i32 = 0o106;
const MACH_PORT_LIMITS_INFO: i32 = 1;
/// `MACH_SEND_TIMED_OUT`.
pub(crate) const SEND_TIMED_OUT: i32 = 0x1000_0004;

const HEADER: usize = 24;
const BODY: usize = 4;
const PORT_DESCRIPTOR: usize = 12;
const OOL_DESCRIPTOR: usize = 16;
/// The largest inline payload.
pub(crate) const MAX_PAYLOAD: usize = 32 * 1024;
/// The most port rights one message carries.
pub(crate) const MAX_PORTS: usize = 64;
/// Room for a header, a body, every descriptor, the payload and a trailer.
const BUFFER: usize =
    HEADER + BODY + MAX_PORTS * PORT_DESCRIPTOR + OOL_DESCRIPTOR + MAX_PAYLOAD + 256;

/// `ipc_info_space_basic_t` (`mach_debug/ipc_info.h`): 24 bytes.
#[repr(C)]
#[derive(Default)]
struct SpaceBasicInfo {
    genno_mask: u32,
    table_size: u32,
    table_next: u32,
    table_inuse: u32,
    reserved: [u32; 2],
}

unsafe extern "C" {
    static mach_task_self_: PortName;
    static bootstrap_port: PortName;
    fn mach_port_allocate(task: PortName, right: u32, name: *mut PortName) -> i32;
    fn mach_port_deallocate(task: PortName, name: PortName) -> i32;
    fn mach_port_mod_refs(task: PortName, name: PortName, right: u32, delta: i32) -> i32;
    fn mach_port_insert_right(
        task: PortName,
        name: PortName,
        right: PortName,
        disposition: u32,
    ) -> i32;
    fn mach_port_get_refs(task: PortName, name: PortName, right: u32, refs: *mut u32) -> i32;
    fn mach_port_set_attributes(
        task: PortName,
        name: PortName,
        flavor: i32,
        info: *const u32,
        count: u32,
    ) -> i32;
    fn mach_port_request_notification(
        task: PortName,
        name: PortName,
        id: i32,
        sync: u32,
        notify: PortName,
        disposition: u32,
        previous: *mut PortName,
    ) -> i32;
    fn mach_port_space_basic_info(task: PortName, info: *mut SpaceBasicInfo) -> i32;
    fn mach_msg(
        message: *mut u8,
        option: i32,
        send_size: u32,
        receive_size: u32,
        receive_name: PortName,
        timeout: u32,
        notify: PortName,
    ) -> i32;
    fn mach_msg_destroy(message: *mut u8);
    fn bootstrap_check_in(bootstrap: PortName, service: *const c_char, port: *mut PortName) -> i32;
    fn bootstrap_look_up(bootstrap: PortName, service: *const c_char, port: *mut PortName) -> i32;
    fn vm_deallocate(task: PortName, address: usize, size: usize) -> i32;
}

/// A Mach error code.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct MachError(pub(crate) i32);

impl MachError {
    const MALFORMED: Self = Self(-1);
    const TOO_LARGE: Self = Self(-2);
}

impl fmt::Debug for MachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mach error {:#x}", self.0)
    }
}

impl fmt::Display for MachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for MachError {}

fn check(status: i32) -> Result<(), MachError> {
    if status == 0 {
        Ok(())
    } else {
        Err(MachError(status))
    }
}

pub(super) fn task() -> PortName {
    // SAFETY: L6.
    unsafe { mach_task_self_ }
}

/// How full this task's port space is: entries in use, and the table's size.
pub(crate) fn space() -> Result<(u32, u32), MachError> {
    let mut info = SpaceBasicInfo::default();
    // SAFETY: L2 and L10, `info` is a live `ipc_info_space_basic_t`.
    check(unsafe { mach_port_space_basic_info(task(), &raw mut info) })?;
    Ok((info.table_inuse, info.table_size))
}

/// The user references this task holds for the send right (or dead name)
/// `name`.
pub(crate) fn send_refs(name: PortName) -> Result<u32, MachError> {
    let mut refs: u32 = 0;
    // SAFETY: L2, `refs`.
    check(unsafe { mach_port_get_refs(task(), name, MACH_PORT_RIGHT_SEND, &raw mut refs) })?;
    Ok(refs)
}

/// A send right, released on drop (L3).
#[derive(Debug)]
pub(crate) struct SendRight(PortName);

impl SendRight {
    /// Takes ownership of a send right (or dead name) this task holds under
    /// `name`.
    pub(crate) fn from_raw(name: PortName) -> Self {
        Self(name)
    }

    pub(crate) fn name(&self) -> PortName {
        self.0
    }
}

impl Drop for SendRight {
    fn drop(&mut self) {
        if self.0 != 0 && self.0 != DEAD {
            // SAFETY: L3, this value owns the right, released once here.
            unsafe { mach_port_deallocate(task(), self.0) };
        }
    }
}

/// A receive right, destroyed on drop (L3).
#[derive(Debug)]
pub(crate) struct ReceiveRight(PortName);

impl Drop for ReceiveRight {
    fn drop(&mut self) {
        // SAFETY: L3, this value owns the receive right, released once here.
        unsafe { mach_port_mod_refs(task(), self.0, MACH_PORT_RIGHT_RECEIVE, -1) };
    }
}

/// The browser's rendezvous: check in a service name.
pub(crate) fn check_in(service: &str) -> Result<ReceiveRight, MachError> {
    let name = CString::new(service).map_err(|_| MachError::MALFORMED)?;
    let mut port: PortName = 0;
    // SAFETY: L1, `name` outlives the call; L2, `port`; L6, `bootstrap_port`.
    check(unsafe { bootstrap_check_in(bootstrap_port, name.as_ptr(), &raw mut port) })?;
    Ok(ReceiveRight(port))
}

/// A helper's side of the rendezvous: look the service up.
pub(crate) fn look_up(service: &str) -> Result<SendRight, MachError> {
    let name = CString::new(service).map_err(|_| MachError::MALFORMED)?;
    let mut port: PortName = 0;
    // SAFETY: L1, `name` outlives the call; L2, `port`; L6, `bootstrap_port`.
    check(unsafe { bootstrap_look_up(bootstrap_port, name.as_ptr(), &raw mut port) })?;
    Ok(SendRight(port))
}

/// A send right to attach to a message: copied, or moved and so given away.
#[derive(Debug)]
pub(crate) enum Attach<'a> {
    Copy(&'a SendRight),
    Move(SendRight),
}

/// A message to send.
#[derive(Debug)]
pub(crate) struct Outgoing<'a> {
    pub(crate) id: i32,
    /// Inline bytes, at most [`MAX_PAYLOAD`].
    pub(crate) payload: &'a [u8],
    /// Rights, in order, at most [`MAX_PORTS`].
    pub(crate) ports: Vec<Attach<'a>>,
    /// Bytes sent out of line, as a virtual copy.
    pub(crate) out_of_line: Option<&'a [u8]>,
}

impl<'a> Outgoing<'a> {
    pub(crate) fn new(id: i32, payload: &'a [u8]) -> Self {
        Self {
            id,
            payload,
            ports: Vec::new(),
            out_of_line: None,
        }
    }
}

/// A message buffer: 8-byte words on the heap, so its bytes are aligned
/// (L4).
pub(crate) struct MessageBuffer {
    words: Vec<u64>,
}

impl MessageBuffer {
    pub(crate) fn new() -> Self {
        Self::with_len(BUFFER)
    }

    fn with_len(len: usize) -> Self {
        Self {
            words: vec![0; len.div_ceil(8)],
        }
    }

    fn bytes(&mut self) -> &mut [u8] {
        let len = self.words.len() * 8;
        let start = self.words.as_mut_ptr().cast::<u8>();
        // SAFETY: L4, the words are owned, live and `len` bytes long, and any
        // byte pattern is a valid `u8`; the borrow of `self` covers the slice.
        unsafe { std::slice::from_raw_parts_mut(start, len) }
    }
}

impl fmt::Debug for MessageBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MessageBuffer")
    }
}

/// Memory the kernel mapped for an out-of-line region, deallocated on drop
/// (L5). S16's messages carry none, but a parsed message owns any it gets.
#[derive(Debug)]
pub(crate) struct OutOfLine {
    address: usize,
    len: usize,
}

impl Drop for OutOfLine {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: L5, the region the kernel mapped, returned once.
            unsafe { vm_deallocate(task(), self.address, self.len) };
        }
    }
}

/// A parsed message: every right it carried is owned by a value here.
#[derive(Debug, Default)]
pub(crate) struct Received {
    pub(crate) id: i32,
    /// The payload, padded to four bytes.
    pub(crate) payload: Vec<u8>,
    /// The send rights it carried, in order.
    pub(crate) ports: Vec<SendRight>,
    pub(crate) out_of_line: Option<OutOfLine>,
    /// The sender's reply port, if it attached one.
    pub(crate) reply: Option<SendRight>,
}

/// A message as the kernel delivered it, or handed back after a failed send:
/// its rights are in this task's space but no value owns them yet. Dropping
/// it destroys it, releasing them all (L3), unless S16-Q2 leaks it on
/// purpose.
pub(crate) struct RawMessage {
    buffer: MessageBuffer,
    done: bool,
}

impl fmt::Debug for RawMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawMessage")
    }
}

impl RawMessage {
    fn from_bytes(bytes: &[u8]) -> Self {
        let size = (get_u32(bytes, 4) as usize).clamp(HEADER, bytes.len());
        let mut buffer = MessageBuffer::with_len(size);
        buffer.bytes()[..size].copy_from_slice(&bytes[..size]);
        Self {
            buffer,
            done: false,
        }
    }

    pub(crate) fn id(&mut self) -> i32 {
        i32::from_ne_bytes(get_u32(self.buffer.bytes(), 20).to_ne_bytes())
    }

    /// The names of the port rights its descriptors carry, in order.
    pub(crate) fn port_names(&mut self) -> Vec<PortName> {
        let bytes = self.buffer.bytes();
        let size = (get_u32(bytes, 4) as usize).min(bytes.len());
        let mut names = Vec::new();
        if get_u32(bytes, 0) & MACH_MSGH_BITS_COMPLEX == 0 || size < HEADER + BODY {
            return names;
        }
        let mut offset = HEADER + BODY;
        for _ in 0..get_u32(bytes, HEADER) {
            if offset + PORT_DESCRIPTOR > size {
                break;
            }
            match get_u32(bytes, offset + 8) >> 24 {
                MACH_MSG_PORT_DESCRIPTOR => {
                    names.push(get_u32(bytes, offset));
                    offset += PORT_DESCRIPTOR;
                }
                MACH_MSG_OOL_DESCRIPTOR => offset += OOL_DESCRIPTOR,
                _ => break,
            }
        }
        names
    }

    /// Hand every right to a value, or, if the message isn't one this crate
    /// can parse whole, destroy it, so that nothing it carried leaks.
    pub(crate) fn parse(mut self) -> Result<Received, MachError> {
        let parsed = parse(self.buffer.bytes());
        if parsed.is_ok() {
            self.done = true;
        }
        // On an error, `self` drops here and destroys the message.
        parsed
    }

    /// Release every right and region the message carries.
    pub(crate) fn destroy(mut self) {
        self.destroy_now();
    }

    /// Drop the message without releasing what it carries: what a receiver
    /// that forgets to destroy dropped messages does (S16-Q2).
    pub(crate) fn leak(mut self) {
        self.done = true;
    }

    fn destroy_now(&mut self) {
        if !self.done {
            self.done = true;
            let bytes = self.buffer.bytes();
            // SAFETY: L4, the buffer holds a whole message as the kernel wrote
            // it, header first; L3, nothing else owns its rights or regions,
            // which this releases once.
            unsafe { mach_msg_destroy(bytes.as_mut_ptr()) };
        }
    }
}

impl Drop for RawMessage {
    fn drop(&mut self) {
        self.destroy_now();
    }
}

impl ReceiveRight {
    /// A new, unnamed port.
    pub(crate) fn new() -> Result<Self, MachError> {
        let mut port: PortName = 0;
        // SAFETY: L2, `port`.
        check(unsafe { mach_port_allocate(task(), MACH_PORT_RIGHT_RECEIVE, &raw mut port) })?;
        Ok(Self(port))
    }

    pub(crate) fn name(&self) -> PortName {
        self.0
    }

    /// A send right to this port, under the same name.
    pub(crate) fn make_send(&self) -> Result<SendRight, MachError> {
        // SAFETY: L3, this task holds the receive right named `self.0`.
        check(unsafe { mach_port_insert_right(task(), self.0, self.0, MACH_MSG_TYPE_MAKE_SEND) })?;
        Ok(SendRight(self.0))
    }

    /// Hold at most `limit` queued messages.
    pub(crate) fn set_queue_limit(&self, limit: u32) -> Result<(), MachError> {
        let info = [limit];
        // SAFETY: L2 and L10, one `mach_port_limits_t`, a count of one.
        check(unsafe {
            mach_port_set_attributes(task(), self.0, MACH_PORT_LIMITS_INFO, info.as_ptr(), 1)
        })
    }

    /// Ask for a no-senders notification on `notify` when this port's last
    /// send right is released.
    pub(crate) fn request_no_senders(&self, notify: &ReceiveRight) -> Result<(), MachError> {
        let mut previous: PortName = 0;
        // SAFETY: L2, `previous`; L3, both receive rights are this task's,
        // and the kernel makes the send-once right it uses.
        check(unsafe {
            mach_port_request_notification(
                task(),
                self.0,
                NOTIFY_NO_SENDERS,
                0,
                notify.0,
                MACH_MSG_TYPE_MAKE_SEND_ONCE,
                &raw mut previous,
            )
        })?;
        drop(SendRight(previous));
        Ok(())
    }

    /// Receive one message and keep it raw, waiting at most `timeout`.
    pub(crate) fn receive_raw(
        &self,
        buffer: &mut MessageBuffer,
        timeout: Option<Duration>,
    ) -> Result<RawMessage, MachError> {
        let (option, millis) = match timeout {
            Some(t) => (
                MACH_RCV_MSG | MACH_RCV_TIMEOUT,
                u32::try_from(t.as_millis()).unwrap_or(u32::MAX),
            ),
            None => (MACH_RCV_MSG, 0),
        };
        let bytes = buffer.bytes();
        let size = u32::try_from(bytes.len()).map_err(|_| MachError::TOO_LARGE)?;
        // SAFETY: L4, `bytes` is an owned, aligned buffer of `size` bytes.
        let status = unsafe { mach_msg(bytes.as_mut_ptr(), option, 0, size, self.0, millis, 0) };
        check(status)?;
        Ok(RawMessage::from_bytes(bytes))
    }

    /// Receive one message and parse it, waiting at most `timeout`.
    pub(crate) fn receive(
        &self,
        buffer: &mut MessageBuffer,
        timeout: Option<Duration>,
    ) -> Result<Received, MachError> {
        self.receive_raw(buffer, timeout)?.parse()
    }
}

/// A failed send: its status, and the message as the kernel handed it back.
#[derive(Debug)]
pub(crate) struct Failed {
    pub(crate) status: MachError,
    pub(crate) message: RawMessage,
}

impl SendRight {
    /// Send `message`; on failure, return it as the kernel left it, with any
    /// rights it carried back in this task's space (S16-Q2), for the caller to
    /// destroy, keep or leak.
    pub(crate) fn try_send(
        &self,
        message: Outgoing<'_>,
        reply: Option<&ReceiveRight>,
        timeout: Option<Duration>,
    ) -> Result<(), Failed> {
        let mut buffer = MessageBuffer::new();
        let bytes = buffer.bytes();
        let size = match encode(bytes, self.0, reply.map(|r| r.0), &message) {
            Ok(size) => size,
            Err(status) => {
                return Err(Failed {
                    status,
                    message: RawMessage { buffer, done: true },
                });
            }
        };
        let (option, millis) = match timeout {
            Some(t) => (
                MACH_SEND_MSG | MACH_SEND_TIMEOUT,
                u32::try_from(t.as_millis()).unwrap_or(u32::MAX),
            ),
            None => (MACH_SEND_MSG, 0),
        };
        // SAFETY: L4, the message was built in `bytes`, `size` bytes long.
        let status = unsafe { mach_msg(bytes.as_mut_ptr(), option, size, 0, 0, millis, 0) };
        // L3: rights moved into the message belong to the kernel now, or are
        // back in the returned message, which owns them.
        for port in message.ports {
            if let Attach::Move(right) = port {
                std::mem::forget(right);
            }
        }
        if status == 0 {
            Ok(())
        } else {
            Err(Failed {
                status: MachError(status),
                message: RawMessage {
                    buffer,
                    done: false,
                },
            })
        }
    }

    /// Send `message`, with a send right to `reply` if one is given. A failed
    /// send destroys the message it gets back, so no right leaks.
    pub(crate) fn send(
        &self,
        message: Outgoing<'_>,
        reply: Option<&ReceiveRight>,
        timeout: Option<Duration>,
    ) -> Result<(), MachError> {
        self.try_send(message, reply, timeout)
            .map_err(|failed| failed.status)
    }
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_ne_bytes());
}

fn get_u32(bytes: &[u8], at: usize) -> u32 {
    let mut word = [0; 4];
    word.copy_from_slice(&bytes[at..at + 4]);
    u32::from_ne_bytes(word)
}

fn encode(
    bytes: &mut [u8],
    remote: PortName,
    reply: Option<PortName>,
    message: &Outgoing<'_>,
) -> Result<u32, MachError> {
    let padded = message.payload.len().next_multiple_of(4);
    if padded > MAX_PAYLOAD || message.ports.len() > MAX_PORTS {
        return Err(MachError::TOO_LARGE);
    }
    let descriptors = message.ports.len() + usize::from(message.out_of_line.is_some());
    let mut bits = MACH_MSG_TYPE_COPY_SEND;
    if reply.is_some() {
        bits |= MACH_MSG_TYPE_MAKE_SEND << 8;
    }
    let mut offset = HEADER;
    if descriptors > 0 {
        bits |= MACH_MSGH_BITS_COMPLEX;
        put_u32(
            bytes,
            HEADER,
            u32::try_from(descriptors).map_err(|_| MachError::TOO_LARGE)?,
        );
        offset += BODY;
        for port in &message.ports {
            let (name, disposition) = match port {
                Attach::Copy(right) => (right.0, MACH_MSG_TYPE_COPY_SEND),
                Attach::Move(right) => (right.0, MACH_MSG_TYPE_MOVE_SEND),
            };
            put_u32(bytes, offset, name);
            put_u32(bytes, offset + 4, 0);
            // Bit-fields, low to high: pad (16), disposition (8), type (8).
            put_u32(
                bytes,
                offset + 8,
                (disposition << 16) | (MACH_MSG_PORT_DESCRIPTOR << 24),
            );
            offset += PORT_DESCRIPTOR;
        }
        if let Some(region) = message.out_of_line {
            let len = u32::try_from(region.len()).map_err(|_| MachError::TOO_LARGE)?;
            let address = region.as_ptr().expose_provenance() as u64;
            bytes[offset..offset + 8].copy_from_slice(&address.to_ne_bytes());
            // Bit-fields, low to high: deallocate, copy, pad, type (8 each).
            put_u32(
                bytes,
                offset + 8,
                (MACH_MSG_VIRTUAL_COPY << 8) | (MACH_MSG_OOL_DESCRIPTOR << 24),
            );
            put_u32(bytes, offset + 12, len);
            offset += OOL_DESCRIPTOR;
        }
    }
    let size = offset + padded;
    let size_u32 = u32::try_from(size).map_err(|_| MachError::TOO_LARGE)?;
    put_u32(bytes, 0, bits);
    put_u32(bytes, 4, size_u32);
    put_u32(bytes, 8, remote);
    put_u32(bytes, 12, reply.unwrap_or(0));
    put_u32(bytes, 16, 0);
    put_u32(bytes, 20, u32::from_ne_bytes(message.id.to_ne_bytes()));
    bytes[offset..offset + message.payload.len()].copy_from_slice(message.payload);
    bytes[offset + message.payload.len()..size].fill(0);
    Ok(size_u32)
}

/// Parse a received message in two passes: first check that every
/// descriptor is one this crate handles and lies within the message, and
/// only then take its rights. A message that fails the first pass is left
/// whole, for [`RawMessage`] to destroy, so no right is released twice or
/// leaked.
fn parse(bytes: &[u8]) -> Result<Received, MachError> {
    let bits = get_u32(bytes, 0);
    let size = get_u32(bytes, 4) as usize;
    if size > bytes.len() || size < HEADER {
        return Err(MachError::MALFORMED);
    }
    let complex = bits & MACH_MSGH_BITS_COMPLEX != 0;
    let count = if complex {
        if size < HEADER + BODY {
            return Err(MachError::MALFORMED);
        }
        get_u32(bytes, HEADER) as usize
    } else {
        0
    };
    let mut offset = if complex { HEADER + BODY } else { HEADER };
    for _ in 0..count {
        if offset + PORT_DESCRIPTOR > size {
            return Err(MachError::MALFORMED);
        }
        match get_u32(bytes, offset + 8) >> 24 {
            MACH_MSG_PORT_DESCRIPTOR => offset += PORT_DESCRIPTOR,
            MACH_MSG_OOL_DESCRIPTOR if offset + OOL_DESCRIPTOR <= size => offset += OOL_DESCRIPTOR,
            _ => return Err(MachError::MALFORMED),
        }
    }
    let remote = get_u32(bytes, 8);
    let mut received = Received {
        id: i32::from_ne_bytes(get_u32(bytes, 20).to_ne_bytes()),
        reply: (remote != 0).then_some(SendRight(remote)),
        payload: bytes[offset..size].to_vec(),
        ..Received::default()
    };
    let mut at = if complex { HEADER + BODY } else { HEADER };
    for _ in 0..count {
        if get_u32(bytes, at + 8) >> 24 == MACH_MSG_PORT_DESCRIPTOR {
            received.ports.push(SendRight(get_u32(bytes, at)));
            at += PORT_DESCRIPTOR;
        } else {
            let mut address = [0; 8];
            address.copy_from_slice(&bytes[at..at + 8]);
            received.out_of_line = Some(OutOfLine {
                address: usize::try_from(u64::from_ne_bytes(address)).unwrap_or(0),
                len: get_u32(bytes, at + 12) as usize,
            });
            at += OOL_DESCRIPTOR;
        }
    }
    Ok(received)
}

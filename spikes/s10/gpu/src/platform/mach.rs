//! Mach ports and messages. S4's transport, extended so that a message can
//! carry port rights as well as one out-of-line region: renderers' memory
//! entries (IP14) and the GPU process's IOSurface objects travel as send rights.

use std::ffi::{CString, c_char};
use std::fmt;
use std::time::Duration;

pub(crate) type PortName = u32;

const MACH_PORT_RIGHT_RECEIVE: u32 = 1;
const MACH_MSG_TYPE_MOVE_SEND: u32 = 17;
const MACH_MSG_TYPE_COPY_SEND: u32 = 19;
const MACH_MSG_TYPE_MAKE_SEND: u32 = 20;
const MACH_MSGH_BITS_COMPLEX: u32 = 0x8000_0000;
const MACH_SEND_MSG: i32 = 0x1;
const MACH_RCV_MSG: i32 = 0x2;
const MACH_SEND_TIMEOUT: i32 = 0x10;
const MACH_RCV_TIMEOUT: i32 = 0x100;
const MACH_MSG_PORT_DESCRIPTOR: u32 = 0;
const MACH_MSG_OOL_DESCRIPTOR: u32 = 1;
const MACH_MSG_VIRTUAL_COPY: u32 = 1;

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

unsafe extern "C" {
    static mach_task_self_: PortName;
    static bootstrap_port: PortName;
    fn mach_port_allocate(task: PortName, right: u32, name: *mut PortName) -> i32;
    fn mach_port_deallocate(task: PortName, name: PortName) -> i32;
    fn mach_port_mod_refs(task: PortName, name: PortName, right: u32, delta: i32) -> i32;
    fn mach_msg(
        message: *mut u8,
        option: i32,
        send_size: u32,
        receive_size: u32,
        receive_name: PortName,
        timeout: u32,
        notify: PortName,
    ) -> i32;
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
    // SAFETY: G6.
    unsafe { mach_task_self_ }
}

/// A send right, released on drop (G3).
#[derive(Debug)]
pub(crate) struct SendRight(PortName);

impl SendRight {
    /// Takes ownership of a send right this task holds under `name`.
    pub(super) fn from_raw(name: PortName) -> Self {
        Self(name)
    }

    pub(super) fn name(&self) -> PortName {
        self.0
    }
}

impl Drop for SendRight {
    fn drop(&mut self) {
        if self.0 != 0 {
            // SAFETY: G3, this value owns the right, released once here.
            unsafe { mach_port_deallocate(task(), self.0) };
        }
    }
}

/// A receive right, destroyed on drop (G3).
#[derive(Debug)]
pub(crate) struct ReceiveRight(PortName);

impl Drop for ReceiveRight {
    fn drop(&mut self) {
        // SAFETY: G3, this value owns the receive right, released once here.
        unsafe { mach_port_mod_refs(task(), self.0, MACH_PORT_RIGHT_RECEIVE, -1) };
    }
}

/// The browser's rendezvous: check in a service name.
pub(crate) fn check_in(service: &str) -> Result<ReceiveRight, MachError> {
    let name = CString::new(service).map_err(|_| MachError::MALFORMED)?;
    let mut port: PortName = 0;
    // SAFETY: G1, `name` outlives the call; G2, `port`; G6, `bootstrap_port`.
    check(unsafe { bootstrap_check_in(bootstrap_port, name.as_ptr(), &raw mut port) })?;
    Ok(ReceiveRight(port))
}

/// A helper's side of the rendezvous: look the service up.
pub(crate) fn look_up(service: &str) -> Result<SendRight, MachError> {
    let name = CString::new(service).map_err(|_| MachError::MALFORMED)?;
    let mut port: PortName = 0;
    // SAFETY: G1, `name` outlives the call; G2, `port`; G6, `bootstrap_port`.
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
/// (G4).
pub(crate) struct MessageBuffer {
    words: Vec<u64>,
}

impl MessageBuffer {
    pub(crate) fn new() -> Self {
        Self {
            words: vec![0; BUFFER.div_ceil(8)],
        }
    }

    fn bytes(&mut self) -> &mut [u8] {
        let len = self.words.len() * 8;
        let start = self.words.as_mut_ptr().cast::<u8>();
        // SAFETY: G4, the words are owned, live and `len` bytes long, and any
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
/// (G5).
#[derive(Debug)]
pub(crate) struct OutOfLine {
    address: usize,
    len: usize,
}

impl OutOfLine {
    pub(crate) fn bytes(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        let start: *const u8 = std::ptr::with_exposed_provenance(self.address);
        // SAFETY: G5, the kernel mapped `len` readable bytes at `address` for
        // this message, owned by this value until it is dropped.
        unsafe { std::slice::from_raw_parts(start, self.len) }
    }
}

impl Drop for OutOfLine {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: G5, the region the kernel mapped, returned once.
            unsafe { vm_deallocate(task(), self.address, self.len) };
        }
    }
}

/// A received message.
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

impl ReceiveRight {
    /// A new, unnamed port.
    pub(crate) fn new() -> Result<Self, MachError> {
        let mut port: PortName = 0;
        // SAFETY: G2, `port`.
        check(unsafe { mach_port_allocate(task(), MACH_PORT_RIGHT_RECEIVE, &raw mut port) })?;
        Ok(Self(port))
    }

    /// Receive one message, waiting at most `timeout`.
    pub(crate) fn receive(
        &self,
        buffer: &mut MessageBuffer,
        timeout: Option<Duration>,
    ) -> Result<Received, MachError> {
        let (option, millis) = match timeout {
            Some(t) => (
                MACH_RCV_MSG | MACH_RCV_TIMEOUT,
                u32::try_from(t.as_millis()).unwrap_or(u32::MAX),
            ),
            None => (MACH_RCV_MSG, 0),
        };
        let bytes = buffer.bytes();
        let size = u32::try_from(bytes.len()).map_err(|_| MachError::TOO_LARGE)?;
        // SAFETY: G4, `bytes` is an owned, aligned buffer of `size` bytes.
        let status = unsafe { mach_msg(bytes.as_mut_ptr(), option, 0, size, self.0, millis, 0) };
        check(status)?;
        parse(bytes)
    }
}

impl SendRight {
    /// Send `message`, with a send right to `reply` if one is given.
    pub(crate) fn send(
        &self,
        message: Outgoing<'_>,
        reply: Option<&ReceiveRight>,
        timeout: Option<Duration>,
    ) -> Result<(), MachError> {
        let mut buffer = MessageBuffer::new();
        let bytes = buffer.bytes();
        let size = encode(bytes, self.0, reply.map(|r| r.0), &message)?;
        let (option, millis) = match timeout {
            Some(t) => (
                MACH_SEND_MSG | MACH_SEND_TIMEOUT,
                u32::try_from(t.as_millis()).unwrap_or(u32::MAX),
            ),
            None => (MACH_SEND_MSG, 0),
        };
        // SAFETY: G4, the message was built in `bytes`, `size` bytes long.
        let status = unsafe { mach_msg(bytes.as_mut_ptr(), option, size, 0, 0, millis, 0) };
        // G3: rights moved into the message belong to the kernel now.
        for port in message.ports {
            if let Attach::Move(right) = port {
                std::mem::forget(right);
            }
        }
        check(status)
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

/// Parse a received message. Every right it carried is owned by the result,
/// taken before the rest is checked, so a malformed message leaks nothing it
/// has already listed.
fn parse(bytes: &[u8]) -> Result<Received, MachError> {
    let bits = get_u32(bytes, 0);
    let size = get_u32(bytes, 4) as usize;
    let remote = get_u32(bytes, 8);
    let mut received = Received {
        id: i32::from_ne_bytes(get_u32(bytes, 20).to_ne_bytes()),
        reply: (remote != 0).then_some(SendRight(remote)),
        ..Received::default()
    };
    if size > bytes.len() || size < HEADER {
        return Err(MachError::MALFORMED);
    }
    let mut offset = HEADER;
    if bits & MACH_MSGH_BITS_COMPLEX != 0 {
        let count = get_u32(bytes, HEADER) as usize;
        offset += BODY;
        for _ in 0..count {
            if offset + PORT_DESCRIPTOR > size {
                return Err(MachError::MALFORMED);
            }
            match get_u32(bytes, offset + 8) >> 24 {
                MACH_MSG_PORT_DESCRIPTOR => {
                    received.ports.push(SendRight(get_u32(bytes, offset)));
                    offset += PORT_DESCRIPTOR;
                }
                MACH_MSG_OOL_DESCRIPTOR if offset + OOL_DESCRIPTOR <= size => {
                    let mut address = [0; 8];
                    address.copy_from_slice(&bytes[offset..offset + 8]);
                    received.out_of_line = Some(OutOfLine {
                        address: usize::try_from(u64::from_ne_bytes(address))
                            .map_err(|_| MachError::MALFORMED)?,
                        len: get_u32(bytes, offset + 12) as usize,
                    });
                    offset += OOL_DESCRIPTOR;
                }
                _ => return Err(MachError::MALFORMED),
            }
        }
    }
    if offset > size {
        return Err(MachError::MALFORMED);
    }
    received.payload = bytes[offset..size].to_vec();
    Ok(received)
}

//! Mach ports (`mach/mach_port.h`, `mach/message.h`): a port set holding a
//! receive right, which the I/O thread's kqueue watches, and send rights
//! that other threads use to post small messages to it. S4 validated Mach
//! IPC between processes; S7 only checks that the same wait receives it.

use core::ffi::c_int;

type PortName = u32;
type KernReturn = c_int;

// `mach/port.h`, `mach/message.h` (N2).
const MACH_PORT_RIGHT_RECEIVE: u32 = 1;
const MACH_PORT_RIGHT_PORT_SET: u32 = 3;
const MACH_MSG_TYPE_COPY_SEND: u32 = 19;
const MACH_MSG_TYPE_MAKE_SEND: u32 = 20;
const MACH_SEND_MSG: c_int = 0x0000_0001;

/// `mach_msg_header_t` (`mach/message.h`).
#[repr(C)]
struct Header {
    bits: u32,
    size: u32,
    remote_port: PortName,
    local_port: PortName,
    voucher_port: PortName,
    id: i32,
}

const _: () = assert!(size_of::<Header>() == 24);

/// The largest inline body [`SendRight::send`] carries.
pub(crate) const MAX_BODY: usize = 64;

/// A message as sent: the header, then the body (N8).
#[repr(C, align(8))]
struct Message {
    header: Header,
    body: [u8; MAX_BODY],
}

// libSystem: `mach/mach_port.h`, `mach/message.h`, `mach/mach_init.h` (N2).
unsafe extern "C" {
    static mach_task_self_: PortName;
    fn mach_port_allocate(task: PortName, right: u32, name: *mut PortName) -> KernReturn;
    fn mach_port_insert_right(
        task: PortName,
        name: PortName,
        right: PortName,
        disposition: u32,
    ) -> KernReturn;
    fn mach_port_insert_member(task: PortName, member: PortName, set: PortName) -> KernReturn;
    fn mach_port_mod_refs(task: PortName, name: PortName, right: u32, delta: c_int) -> KernReturn;
    fn mach_port_deallocate(task: PortName, name: PortName) -> KernReturn;
    fn mach_msg(
        msg: *mut Header,
        option: c_int,
        send_size: u32,
        rcv_size: u32,
        rcv_name: PortName,
        timeout: u32,
        notify: PortName,
    ) -> KernReturn;
}

fn task() -> PortName {
    // SAFETY: N9. libSystem initializes `mach_task_self_` before `main`.
    unsafe { mach_task_self_ }
}

fn check(code: KernReturn) -> Result<(), KernReturn> {
    if code == 0 { Ok(()) } else { Err(code) }
}

fn allocate(right: u32) -> Result<PortName, KernReturn> {
    let mut name: PortName = 0;
    // SAFETY: N1 (`name` is a live out-parameter); N2.
    check(unsafe { mach_port_allocate(task(), right, &raw mut name) })?;
    Ok(name)
}

/// A receive right, released on drop (N8).
#[derive(Debug)]
pub(crate) struct Port(PortName);

/// A port set, released on drop (N8).
#[derive(Debug)]
pub(crate) struct PortSet(PortName);

/// A send right, released on drop (N8).
#[derive(Debug)]
pub(crate) struct SendRight(PortName);

impl Port {
    pub(crate) fn new() -> Result<Self, KernReturn> {
        allocate(MACH_PORT_RIGHT_RECEIVE).map(Self)
    }

    /// A send right to this port.
    pub(crate) fn make_send(&self) -> Result<SendRight, KernReturn> {
        // SAFETY: N8. `self` holds the receive right that MAKE_SEND needs;
        // the new send right is owned by the returned value.
        check(unsafe { mach_port_insert_right(task(), self.0, self.0, MACH_MSG_TYPE_MAKE_SEND) })?;
        Ok(SendRight(self.0))
    }
}

impl PortSet {
    pub(crate) fn new() -> Result<Self, KernReturn> {
        allocate(MACH_PORT_RIGHT_PORT_SET).map(Self)
    }

    pub(crate) fn name(&self) -> PortName {
        self.0
    }

    /// Add `port` to the set; its messages are then received through it.
    pub(crate) fn insert(&self, port: &Port) -> Result<(), KernReturn> {
        // SAFETY: N8. Both rights are owned by live values.
        check(unsafe { mach_port_insert_member(task(), port.0, self.0) })
    }
}

impl SendRight {
    /// Send a message with id `id` and the inline `body` (at most
    /// [`MAX_BODY`] bytes), blocking while the port's queue is full.
    pub(crate) fn send(&self, id: i32, body: &[u8]) -> Result<(), KernReturn> {
        if body.len() > MAX_BODY {
            return Err(-1);
        }
        // Bodies are padded to a multiple of 4 bytes, as Mach requires.
        let padded = body.len().next_multiple_of(4);
        let size = u32::try_from(size_of::<Header>() + padded).map_err(|_| -1)?;
        let mut message = Message {
            header: Header {
                bits: MACH_MSG_TYPE_COPY_SEND,
                size,
                remote_port: self.0,
                local_port: 0,
                voucher_port: 0,
                id,
            },
            body: [0; MAX_BODY],
        };
        message.body[..body.len()].copy_from_slice(body);
        // SAFETY: N8 (the message carries a copy of the send right `self`
        // owns, and `size` covers only initialized bytes of `message`); N1.
        check(unsafe { mach_msg(&raw mut message.header, MACH_SEND_MSG, size, 0, 0, 0, 0) })
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        // SAFETY: N8. The right's only owner releases it once.
        unsafe { mach_port_mod_refs(task(), self.0, MACH_PORT_RIGHT_RECEIVE, -1) };
    }
}

impl Drop for PortSet {
    fn drop(&mut self) {
        // SAFETY: N8. The right's only owner releases it once.
        unsafe { mach_port_mod_refs(task(), self.0, MACH_PORT_RIGHT_PORT_SET, -1) };
    }
}

impl Drop for SendRight {
    fn drop(&mut self) {
        // SAFETY: N8. The right's only owner releases it once.
        unsafe { mach_port_deallocate(task(), self.0) };
    }
}

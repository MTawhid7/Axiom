//! The platform layer: every foreign call this spike makes, into Mach (ports,
//! messages, memory entries), IOSurface, Metal, CoreAnimation and the clock.
//! It is the only place in the crate where `unsafe` is allowed
//! (`tools/unsafe-policy.txt`). S4's library supplies process launch and
//! sandbox entry, under its own invariants.
//!
//! Mach declarations follow SDK 27.0's `mach/message.h`, `mach/mach_port.h`,
//! `mach/mach_vm.h`, `mach/vm_map.h` and `servers/bootstrap.h`; IOSurface's
//! port functions follow `IOSurface/IOSurfaceRef.h`. Each `unsafe` block cites
//! one of these invariants:
//!
//! - **G1, C strings.** Every string passed to C is a [`CString`] that is
//!   still alive when the call returns.
//! - **G2, out-parameters.** Every out-pointer points to a live, properly
//!   aligned local of the declared type, read only after the call returns.
//! - **G3, Mach rights.** Each port right this crate holds is owned by exactly
//!   one value, whose `Drop` releases it once. A right moved into a message is
//!   forgotten by its owner, since the kernel took it.
//! - **G4, message buffers.** Buffers passed to `mach_msg` are owned, 8-byte
//!   aligned arrays at least as large as the sizes passed. Messages are built
//!   and parsed as bytes in safe code, and a received message's descriptors
//!   are read only within the size the kernel reports.
//! - **G5, memory regions.** A region from `mach_vm_allocate` or `mach_vm_map`,
//!   or one the kernel mapped for an out-of-line descriptor, is owned by one
//!   value and deallocated once. Slices into it never outlive the owner. A
//!   region mapped read-only is never written.
//! - **G6, globals.** `mach_task_self_` and `bootstrap_port` are initialized by
//!   libSystem before `main`, and only read; so are framework constants such
//!   as IOSurface's property keys.
//! - **G7, Metal objects.** A buffer made without a copy points into a region
//!   that outlives it: the value that holds the buffer also owns the region,
//!   and drops the buffer first. Shared buffers are written only within their
//!   length, while no GPU work that reads them is outstanding.
//! - **G8, IOSurface ports.** `IOSurfaceCreateMachPort` returns a send right
//!   that the caller owns; `IOSurfaceLookupFromMachPort` doesn't consume the
//!   right it is given, and returns a retained surface.
//! - **G9, CoreAnimation.** Layers are created and changed on one thread, in
//!   explicit transactions, and their contents are IOSurface objects.
//!
//! [`CString`]: std::ffi::CString
#![allow(unsafe_code)]

pub(crate) mod layers;
pub(crate) mod mach;
pub(crate) mod memory;
pub(crate) mod metal;
pub(crate) mod paths;
pub(crate) mod surface;
pub(crate) mod time;

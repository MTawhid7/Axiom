//! The platform layer: every foreign call this spike makes, into Mach (ports,
//! messages, memory entries, the port space and the host's page counts),
//! libproc's resource usage, IOSurface, Metal, CoreAnimation and the clock.
//! It is the only place in the crate where `unsafe` is allowed
//! (`tools/unsafe-policy.txt`). S4's library supplies process launch and
//! sandbox entry, under its own invariants. Much of it is S10's platform
//! layer, cut down to what S16 uses and extended for its questions.
//!
//! Declarations follow SDK 27.0's headers: `mach/message.h`, `mach/mach.h`
//! (`mach_msg_destroy`), `mach/mach_port.h`, `mach/port.h`, `mach/notify.h`,
//! `mach/mach_vm.h`, `mach/vm_map.h`, `mach/task_info.h`, `mach/host_info.h`,
//! `mach/vm_statistics.h`, `mach_debug/ipc_info.h`, `servers/bootstrap.h`,
//! `sys/resource.h`, `sys/wait.h`, `libproc.h` and `IOSurface/IOSurfaceRef.h`.
//! Each `unsafe` block cites one of these invariants:
//!
//! - **L1, C strings.** Every string passed to C is a [`CString`] that is
//!   still alive when the call returns.
//! - **L2, out-parameters.** Every out-pointer points to a live, properly
//!   aligned local of the declared type, read only after the call returns.
//! - **L3, Mach rights.** Each port right this crate holds is owned by exactly
//!   one value, whose `Drop` releases it once. A right moved into a message is
//!   forgotten by its owner, since the kernel took it. A received message is
//!   held raw until it is parsed, which hands each right to a value, or
//!   destroyed, which releases them all; only S16-Q2's deliberate leak drops
//!   one without either, and the test counts what that leaves.
//! - **L4, message buffers.** Buffers passed to `mach_msg` and
//!   `mach_msg_destroy` are owned, 8-byte aligned arrays at least as large as
//!   the sizes passed. Messages are built and parsed as bytes in safe code,
//!   and a received message's descriptors are read only within the size the
//!   kernel reports.
//! - **L5, memory regions.** A region from `mach_vm_allocate` or
//!   `mach_vm_map`, or one the kernel mapped for an out-of-line descriptor, is
//!   owned by one value and deallocated once. Slices into it never outlive the
//!   owner. A region mapped read-only is never written.
//! - **L6, globals.** `mach_task_self_`, `mach_host_self()`'s port and
//!   `bootstrap_port` are initialized by libSystem before `main`, and only
//!   read; so are framework constants such as IOSurface's property keys.
//! - **L7, Metal objects.** A buffer made without a copy points into a region
//!   that outlives it: the value that holds the buffer also owns the region,
//!   and drops the buffer first.
//! - **L8, IOSurface ports.** `IOSurfaceCreateMachPort` returns a send right
//!   that the caller owns; `IOSurfaceLookupFromMachPort` doesn't consume the
//!   name it is given, whatever it names, and returns a retained surface or
//!   nothing.
//! - **L9, CoreAnimation.** Layers are created and changed on one thread, in
//!   explicit transactions, and their contents are IOSurface objects.
//! - **L10, process information.** `proc_pid_rusage`, `task_info`,
//!   `host_statistics64`, `mach_port_space_basic_info` and `waitid` fill
//!   structures whose layouts match the SDK's, with the counts the SDK
//!   defines.
//!
//! [`CString`]: std::ffi::CString
#![allow(unsafe_code)]

pub(crate) mod layers;
pub(crate) mod mach;
pub(crate) mod memory;
pub(crate) mod metal;
pub(crate) mod paths;
pub(crate) mod rusage;
pub(crate) mod surface;
pub(crate) mod time;

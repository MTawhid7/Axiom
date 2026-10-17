//! The platform layer: every foreign call this spike makes, into libsandbox,
//! the Mach kernel interface, `posix_spawn` and libbsm. It is the only place
//! in the crate where `unsafe` is allowed (`tools/unsafe-policy.txt`).
//!
//! Every declaration was checked against the SDK 27.0 headers or, for the
//! undocumented libsandbox functions, against the SDK's `.tbd` export stubs
//! and Chromium's declarations. Each `unsafe` block cites one of these
//! invariants:
//!
//! - **X1, C strings.** Every string passed to C is a [`CString`] that is
//!   still alive when the call returns.
//! - **X2, out-parameters.** Every out-pointer points to a live, properly
//!   aligned local that the call may write, and that is read only after the
//!   call returns.
//! - **X3, library-owned results.** Error strings, parameter sets and compiled
//!   profiles that libsandbox returns are freed exactly once, with the
//!   matching libsandbox function, and never used afterwards. Each lives in
//!   exactly one owner.
//! - **X4, Mach rights.** Each port right this crate obtains (look-up,
//!   check-in, a right carried by a message) is owned by exactly one Rust value,
//!   whose `Drop` releases it once. Raw names never outlive their owner.
//! - **X5, message buffers.** Buffers passed to `mach_msg` are owned, 8-byte
//!   aligned arrays at least as large as the declared send or receive size.
//!   Their contents are built and parsed as bytes in safe code.
//! - **X6, spawn arguments.** `argv` and `envp` are null-terminated arrays of
//!   pointers into [`CString`]s that outlive the call. File actions and spawn
//!   attributes are initialized before use and destroyed exactly once.
//! - **X7, adopted descriptors.** A file descriptor becomes an owning Rust
//!   value only after it has been checked to be an open socket, and at most
//!   once per process.
//! - **X8, framework globals.** Extern statics (`bootstrap_port`,
//!   `mach_task_self_`, `kSBXProfilePureComputation`) are initialized by
//!   libSystem before `main` and are only read.
//! - **X9, JIT code.** A `MAP_JIT` region is written only while the calling
//!   thread's JIT write protection is off, and executed only after it is back
//!   on and the instruction cache has been invalidated for the written bytes.
//!   The code is a fixed, complete function (`mov w0, #42; ret`). The region
//!   is unmapped once, by its only owner.
//! - **X10, out-of-line memory.** A region sent out of line points into a
//!   live Rust slice for the whole `mach_msg` call. A region received out of
//!   line is the kernel's fresh mapping of `len` readable bytes, owned by one
//!   value that reads it and returns it with `vm_deallocate` once.
//! - **X11, compiled profiles.** `sandbox_profile_t` is laid out as Chromium
//!   declares it (`sandbox/mac/seatbelt.cc`: `char* builtin; const uint8_t*
//!   data; size_t size;`). A compiled profile's bytes belong to libsandbox
//!   until it is freed, and are copied out before then. Bytecode applied in
//!   another process is passed in a stack struct with no builtin name, as
//!   Chromium's `ApplyCompiledProfile` does, and outlives the call.
//!
//! [`CString`]: std::ffi::CString
#![allow(unsafe_code)]

pub mod jit;
pub mod mach;
pub mod process;
pub mod sandbox;

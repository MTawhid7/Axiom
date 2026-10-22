//! The crate's foreign and `unsafe` code: the counting allocator, and
//! `proc_pid_rusage`. It is the only place in the crate where `unsafe` is
//! allowed (`tools/unsafe-policy.txt`), and nothing here ships (CR12).
//!
//! Declarations follow the SDK's `sys/resource.h` and `libproc.h` (SDK 27.0,
//! read 2026-10-23). Each `unsafe` block cites one of these invariants:
//!
//! - **I1, the allocator.** [`CountingAllocator`] passes every call, with its
//!   layout and pointer unchanged, to [`std::alloc::System`], so it keeps
//!   `GlobalAlloc`'s contract exactly as `System` does. Its counting touches
//!   only a thread-local `Cell` of integers, which is initialized as a
//!   constant, has no destructor and never allocates.
//! - **I2, resource usage.** `proc_pid_rusage` is given a live, aligned and
//!   zeroed `rusage_info_v6` of the SDK's layout, checked at compile time, the
//!   struct its `RUSAGE_INFO_V6` flavor fills; the struct is read only after
//!   the call returns success.
#![allow(
    unsafe_code,
    reason = "ES11: the crate's one unsafe module, allowlisted in tools/unsafe-policy.txt under I1 and I2"
)]

mod alloc;
mod rusage;

pub use alloc::CountingAllocator;
pub(crate) use alloc::thread_counts;
pub(crate) use rusage::cpu_counts;

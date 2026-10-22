//! Support for Axiom's tests: golden files, and the counts of PF5's first tier.
//!
//! # Charter (ES15)
//!
//! 1. **Purpose.** What every crate's tests share and no shipping crate may
//!    link: golden files rewritten only on purpose (ES21), and the
//!    deterministic counts that make PF5's first tier a unit test (ES23):
//!    allocations on the current thread, and the instructions and cycles of
//!    the process. The loopback HTTP server (TS8) and fixtures join at M1-CP4.
//! 2. **Rules.** ES21, ES23, PF5, TS8; ES13 for the files and the environment
//!    it reads, which test crates may.
//! 3. **Place** (`tools/crate-map.toml`): outside the layers, in the `test`
//!    group, so no shipping crate depends on it (CR12); panic class ordinary.
//!    **A leaf:** it depends on no Axiom crate, so every crate's tests can use
//!    it without a cycle (CR3). Its `unsafe` is confined to [`sys`]
//!    (`tools/unsafe-policy.txt`; ES11), and never ships.
//! 4. **Interface.**
//!    - [`assert_golden!`] compares a text with `tests/golden/<name>` in the
//!      calling crate, or rewrites the file when `AXIOM_BLESS=1` is set.
//!    - [`count_allocations`] runs a closure and counts the allocations the
//!      current thread made during it, once the test binary has declared
//!      [`CountingAllocator`] as its global allocator.
//!    - [`count_cpu`] runs a closure and counts the instructions the process
//!      retired and the cycles it ran during it, from `proc_pid_rusage` (S16
//!      found instructions steady to 0.03–0.15% from run to run).
//! 5. **Invariants.** The `SAFETY` invariants are [`sys`]'s, I1 and I2.
//!    Counting never changes what is counted: the allocator forwards every
//!    call to the system's unchanged.
//! 6. **Threads.** Allocation counts are per thread, so tests running in
//!    parallel don't disturb each other. CPU counts are per process:
//!    a test that asserts them runs alone in its own test binary.
//! 7. **Tests.** Each module's, and `tests/` for what needs its own binary:
//!    the global allocator, and the instruction counter.
//! 8. **Efficiency.** Counting an allocation costs a thread-local increment.

#![deny(unsafe_code)]

mod counts;
mod golden;
pub mod sys;

pub use counts::{Allocations, Cpu, count_allocations, count_cpu};
pub use golden::{Mismatch, Mode, check_golden};
pub use sys::CountingAllocator;

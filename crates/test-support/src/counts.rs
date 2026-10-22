//! PF5's first tier as unit tests (ES23): allocations and instructions,
//! counted around a closure.

use std::hint::black_box;
use std::io;

use crate::sys;

/// What the current thread allocated during a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Allocations {
    /// Calls to `alloc`, `alloc_zeroed` and `realloc`.
    pub count: u64,
    /// The bytes those calls asked for; a `realloc` counts its new size.
    pub bytes: u64,
}

/// Runs `f` and returns what the current thread allocated meanwhile.
///
/// # Panics
///
/// If the test binary hasn't declared [`crate::CountingAllocator`] as its
/// global allocator, since every count would then read zero.
pub fn count_allocations<R>(f: impl FnOnce() -> R) -> (R, Allocations) {
    assert!(
        counting_allocator_installed(),
        "test_support::CountingAllocator isn't this test binary's global allocator; declare \
         `#[global_allocator] static ALLOCATOR: test_support::CountingAllocator = test_support::CountingAllocator;`"
    );
    let (count_before, bytes_before) = sys::thread_counts();
    let result = f();
    let (count_after, bytes_after) = sys::thread_counts();
    let allocations = Allocations {
        count: count_after.wrapping_sub(count_before),
        bytes: bytes_after.wrapping_sub(bytes_before),
    };
    (result, allocations)
}

fn counting_allocator_installed() -> bool {
    let (before, _) = sys::thread_counts();
    drop(black_box(Box::new(black_box(0_u8))));
    let (after, _) = sys::thread_counts();
    after != before
}

/// The instructions a process retired and the cycles it ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cpu {
    /// Instructions retired, on every core.
    pub instructions: u64,
    /// Cycles run, on every core. Cycles over the core's frequency are a
    /// time; instructions are steadier from run to run (S16).
    pub cycles: u64,
}

/// Runs `f` and returns the instructions the whole process retired, and the
/// cycles it ran, meanwhile.
///
/// The counts cover every thread, so a test that asserts them runs alone in
/// its own test binary (`tests/<name>.rs` with one test).
pub fn count_cpu<R>(f: impl FnOnce() -> R) -> io::Result<(R, Cpu)> {
    let (instructions_before, cycles_before) = sys::cpu_counts()?;
    let result = f();
    let (instructions_after, cycles_after) = sys::cpu_counts()?;
    let cpu = Cpu {
        instructions: instructions_after.saturating_sub(instructions_before),
        cycles: cycles_after.saturating_sub(cycles_before),
    };
    Ok((result, cpu))
}

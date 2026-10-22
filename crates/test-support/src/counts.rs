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

/// Runs `f` and returns the instructions the whole process retired meanwhile.
///
/// The count covers every thread, so a test that asserts it runs alone in
/// its own test binary (`tests/<name>.rs` with one test).
pub fn count_instructions<R>(f: impl FnOnce() -> R) -> io::Result<(R, u64)> {
    let before = sys::instructions_retired()?;
    let result = f();
    let after = sys::instructions_retired()?;
    Ok((result, after.saturating_sub(before)))
}

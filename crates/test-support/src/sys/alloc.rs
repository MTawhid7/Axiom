//! The counting allocator: the system's, with a count per thread of the
//! allocations made and the bytes they asked for.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Allocations and bytes this thread has asked for since it started.
    static COUNTS: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

/// The system allocator, counting each thread's allocations. A test binary
/// that counts allocations declares it as its global allocator:
///
/// ```ignore
/// #[global_allocator]
/// static ALLOCATOR: test_support::CountingAllocator = test_support::CountingAllocator;
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct CountingAllocator;

fn count(size: usize) {
    // `try_with` because a thread that is exiting may still allocate; its
    // count no longer matters then.
    let _ = COUNTS.try_with(|counts| {
        let (allocations, bytes) = counts.get();
        counts.set((allocations.wrapping_add(1), bytes.wrapping_add(size as u64)));
    });
}

/// This thread's allocations and bytes so far.
pub(crate) fn thread_counts() -> (u64, u64) {
    COUNTS.try_with(Cell::get).unwrap_or((0, 0))
}

// SAFETY: I1, every method forwards to `System` with its arguments unchanged,
// so `System`'s implementation of the contract is this one's.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: I1, the caller's contract for `alloc` is passed on as is.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: I1, the caller's contract for `alloc_zeroed` is passed on as is.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: I1, `ptr` came from `System` through this allocator, with
        // `layout`, as the caller's contract for `dealloc` guarantees.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count(new_size);
        // SAFETY: I1, the caller's contract for `realloc` is passed on as is.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

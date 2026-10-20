//! S9-Q2's planted data race: threads add to one counter with no
//! synchronization. Unsound by design, so it is compiled only for tests, and
//! only the thread sanitizer's run calls it.
#![allow(unsafe_code)]

use std::cell::UnsafeCell;
use std::thread;

struct Racy(UnsafeCell<u64>);

// SAFETY: none. This is S9-Q2's planted race: `Racy` is shared between threads
// with no synchronization, which is what the thread sanitizer must report.
unsafe impl Sync for Racy {}

impl Racy {
    fn add_one(&self) {
        // SAFETY: none. The planted race: every thread writes here unsynchronized.
        unsafe { *self.0.get() += 1 };
    }
}

pub(crate) fn count_racy(threads: usize, per_thread: u64) -> u64 {
    let counter = Racy(UnsafeCell::new(0));
    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                for _ in 0..per_thread {
                    counter.add_one();
                }
            });
        }
    });
    counter.0.into_inner()
}

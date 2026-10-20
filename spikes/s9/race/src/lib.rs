//! Spike S9-Q2's subject: a counter that threads share, for the thread
//! sanitizer (`-Zsanitizer=thread`; docs/testing.md §9). [`count_atomic`] shares
//! it correctly. The test-only `planted` module shares it with a data race,
//! which the sanitizer must report; only the sanitizer's run executes it.
#![deny(unsafe_code)]

#[cfg(test)]
mod planted;

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

/// Adds `per_thread` to one counter from each of `threads` threads, atomically.
pub fn count_atomic(threads: usize, per_thread: u64) -> u64 {
    let counter = AtomicU64::new(0);
    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                for _ in 0..per_thread {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    counter.into_inner()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_atomic_count_is_exact() {
        assert_eq!(super::count_atomic(4, 10_000), 40_000);
    }

    #[test]
    #[ignore = "a deliberate data race, run only under ThreadSanitizer (S9-Q2)"]
    fn the_planted_race() {
        let _ = super::planted::count_racy(2, 10_000);
    }
}

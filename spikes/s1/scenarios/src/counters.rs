//! Per-thread live-object counters, which turn GC correctness into exact
//! integers.
//!
//! Pattern implementations embed a zero-sized [`Live<K>`] token in every
//! counted object. Creating the token increments the thread's counter for `K`,
//! and dropping it decrements the counter. Moving an object, for example during
//! Pattern B's compaction, runs neither, so the counts track exactly the
//! objects that exist.
//!
//! The counters are per thread, so tests running in parallel stay isolated.
//! `Live<K>` is `!Send`, so a token can never be dropped on a thread whose
//! counter it doesn't belong to.

use core::marker::PhantomData;
use std::cell::Cell;
use std::thread::LocalKey;

thread_local! {
    static NODES: Cell<usize> = const { Cell::new(0) };
    static CLOSURES: Cell<usize> = const { Cell::new(0) };
}

/// A kind of counted object.
pub trait CounterKind: 'static {
    #[doc(hidden)]
    fn counter() -> &'static LocalKey<Cell<usize>>;
}

/// Counts DOM nodes (including the document and id-less nodes).
#[derive(Debug)]
pub enum Nodes {}

/// Counts listener closures.
#[derive(Debug)]
pub enum Closures {}

impl CounterKind for Nodes {
    fn counter() -> &'static LocalKey<Cell<usize>> {
        &NODES
    }
}

impl CounterKind for Closures {
    fn counter() -> &'static LocalKey<Cell<usize>> {
        &CLOSURES
    }
}

/// A zero-sized token: alive ⇔ counted.
#[derive(Debug)]
pub struct Live<K: CounterKind> {
    /// `*const ()` makes the token `!Send + !Sync`. The counter is per thread.
    _kind: PhantomData<(K, *const ())>,
}

impl<K: CounterKind> Live<K> {
    /// Count one more live object of kind `K`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        K::counter().with(|c| c.set(c.get() + 1));
        Self { _kind: PhantomData }
    }
}

impl<K: CounterKind> Default for Live<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: CounterKind> Clone for Live<K> {
    /// A clone is another live object, so it counts again.
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<K: CounterKind> Drop for Live<K> {
    #[inline]
    fn drop(&mut self) {
        K::counter().with(|c| c.set(c.get() - 1));
    }
}

/// Live objects of kind `K` on the current thread.
#[must_use]
pub fn live<K: CounterKind>() -> usize {
    K::counter().with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_count_creation_clone_and_drop() {
        let base = live::<Nodes>();
        let a = Live::<Nodes>::new();
        let b = a.clone();
        assert_eq!(live::<Nodes>(), base + 2);
        assert_eq!(live::<Closures>(), 0, "kinds are independent");
        drop(a);
        assert_eq!(live::<Nodes>(), base + 1);
        let moved = b; // moving runs neither constructor nor destructor
        assert_eq!(live::<Nodes>(), base + 1);
        drop(moved);
        assert_eq!(live::<Nodes>(), base);
    }

    #[test]
    fn counters_are_per_thread() {
        let _here = Live::<Closures>::new();
        let there = std::thread::spawn(|| {
            let _t = Live::<Closures>::new();
            live::<Closures>()
        })
        .join()
        .expect("thread panicked");
        assert_eq!(there, 1);
        assert_eq!(live::<Closures>(), 1);
    }

    #[test]
    fn tokens_are_zero_sized() {
        assert_eq!(size_of::<Live<Nodes>>(), 0);
    }
}

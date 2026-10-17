//! Typed arenas, plus the per-heap bookkeeping (`HeapCore`) that the `heap!`
//! macro embeds. As in S1, except that the root stack lives outside the heap
//! and a [`Barrier`] runs on allocation and after collection.

use core::ops::{Index, IndexMut};
use core::time::Duration;
use std::time::Instant;

use crate::barrier::{Barrier, NoBarrier};
use crate::handle::Handle;
use crate::roots::{RootFrame, RootStack};
use crate::trace::{Allocated, Forwarding, Marker, Roots, Trace};

/// One arena: a dense `Vec` of one object type, addressed by [`Handle`]s.
#[derive(Debug)]
pub struct Arena<T> {
    items: Vec<T>,
    epoch: u16,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            epoch: 0,
        }
    }
}

impl<T: Allocated> Arena<T> {
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Validate a handle. In debug builds, this panics if the handle was issued
    /// before the last collection and not relocated since.
    #[inline]
    fn slot(&self, handle: Handle<'_, T>) -> usize {
        debug_assert_eq!(
            handle.epoch(),
            self.epoch,
            "stale handle: used after a collection without rooting or re-binding"
        );
        handle.index()
    }
}

impl<T: Allocated> Index<Handle<'_, T>> for Arena<T> {
    type Output = T;

    #[inline]
    fn index(&self, handle: Handle<'_, T>) -> &T {
        &self.items[self.slot(handle)]
    }
}

impl<T: Allocated> IndexMut<Handle<'_, T>> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, handle: Handle<'_, T>) -> &mut T {
        let slot = self.slot(handle);
        &mut self.items[slot]
    }
}

/// When a heap wants to collect: S1's policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pacing {
    pub min_bytes: usize,
    pub growth_percent: usize,
}

impl Pacing {
    /// Collect at every safepoint: the stress mode of the oracle tests.
    pub const STRESS: Self = Self {
        min_bytes: 0,
        growth_percent: 0,
    };

    /// Never collect unless asked: mutator-only benchmarks.
    pub const NEVER: Self = Self {
        min_bytes: usize::MAX,
        growth_percent: 0,
    };

    #[must_use]
    pub const fn budget(&self, survived_bytes: usize) -> usize {
        let proportional = survived_bytes.saturating_mul(self.growth_percent) / 100;
        if proportional > self.min_bytes {
            proportional
        } else {
            self.min_bytes
        }
    }
}

impl Default for Pacing {
    fn default() -> Self {
        Self {
            min_bytes: 256 * 1024,
            growth_percent: 100,
        }
    }
}

/// Heap accounting. Byte counts are `size_of` the objects in their arenas,
/// not counting `Vec` slack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    pub objects: usize,
    pub bytes: usize,
    pub peak_objects: usize,
    pub peak_bytes: usize,
    pub allocated_since_collection: usize,
    pub total_allocated_objects: u64,
    pub total_allocated_bytes: u64,
    pub collections: u64,
    pub last_pause: Duration,
    pub max_pause: Duration,
    pub total_pause: Duration,
    pub objects_after_collection: usize,
    pub bytes_after_collection: usize,
}

/// The type-independent part of a heap. Every `heap!` heap embeds one.
#[derive(Debug)]
pub struct HeapCore<'r, S: RootStack, B: Barrier = NoBarrier> {
    roots: &'r S,
    pacing: Pacing,
    epoch: u16,
    metrics: Metrics,
    /// The write barrier's state.
    pub barrier: B,
}

impl<'r, S: RootStack, B: Barrier> HeapCore<'r, S, B> {
    #[must_use]
    pub fn new(roots: &'r S, pacing: Pacing) -> Self {
        Self {
            roots,
            pacing,
            epoch: 0,
            metrics: Metrics::default(),
            barrier: B::default(),
        }
    }

    /// Open a root frame (see [`RootFrame`]). It borrows the root stack, not
    /// the heap.
    #[inline]
    #[must_use]
    pub fn frame(&self) -> RootFrame<'r, S> {
        RootFrame::new(self.roots)
    }

    /// The root stack.
    #[inline]
    #[must_use]
    pub const fn roots(&self) -> &'r S {
        self.roots
    }

    /// The number of rooted values currently on the stack.
    #[must_use]
    pub fn rooted(&self) -> usize {
        self.roots.height()
    }

    /// Whether the allocation budget since the last collection is spent.
    #[inline]
    #[must_use]
    pub fn wants_collection(&self) -> bool {
        self.metrics.allocated_since_collection
            >= self.pacing.budget(self.metrics.bytes_after_collection)
    }

    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.metrics
    }

    #[inline]
    fn note_allocation(&mut self, bytes: usize) {
        let m = &mut self.metrics;
        m.objects += 1;
        m.bytes += bytes;
        m.peak_objects = m.peak_objects.max(m.objects);
        m.peak_bytes = m.peak_bytes.max(m.bytes);
        m.allocated_since_collection += bytes;
        m.total_allocated_objects += 1;
        m.total_allocated_bytes += bytes as u64;
    }
}

/// Access to the arena for `T` in a heap. The `heap!` macro implements this.
pub trait Store<T: Allocated> {
    fn arena(&self) -> &Arena<T>;
    fn arena_mut(&mut self) -> &mut Arena<T>;
}

/// A heap's layout: one arena per type, plus a `HeapCore`. The `heap!` macro
/// implements this, and [`collect_heap`] runs the collection algorithm on it.
pub trait HeapLayout<'r, S: RootStack, B: Barrier> {
    /// The length of each arena, in `ARENA` order.
    fn arena_lens(&self) -> Vec<usize>;
    /// Call `Trace::mark` on item `index` of arena `arena`.
    fn mark_item(&self, arena: usize, index: usize, marker: &mut Marker);
    /// Compact every arena and relocate the handles inside the survivors.
    fn compact_all(&mut self, forwarding: &Forwarding);
    /// (objects, bytes) currently stored.
    fn size(&self) -> (usize, usize);
    fn core(&mut self) -> &mut HeapCore<'r, S, B>;
}

/// Allocate `value` in `heap` without a safepoint: this never collects.
#[inline]
pub fn alloc<'r, S, B, H, T>(heap: &mut H, value: T) -> Handle<'static, T>
where
    S: RootStack + 'r,
    B: Barrier,
    H: HeapLayout<'r, S, B> + Store<T>,
    T: Allocated,
{
    let index = heap.arena().items.len();
    let core = heap.core();
    core.note_allocation(size_of::<T>());
    core.barrier.on_alloc(T::ARENA, index);
    let arena = heap.arena_mut();
    let handle = Handle::new(index, arena.epoch);
    arena.items.push(value);
    handle
}

/// A full stop-the-world mark–compact collection.
///
/// 1. Mark from the root stack and `roots`, with a worklist.
/// 2. Rank the survivors in each arena.
/// 3. Compact every arena in place, keeping order, so a survivor's rank is its
///    new index; dead objects are dropped.
/// 4. Relocate every handle and value, in heap data, roots and root slots,
///    through the ranks.
pub fn collect_heap<'r, S, B, H, R>(heap: &mut H, roots: &mut R)
where
    S: RootStack + 'r,
    B: Barrier,
    H: HeapLayout<'r, S, B> + ?Sized,
    R: Roots + ?Sized,
{
    let start = Instant::now();
    let lens = heap.arena_lens();
    let core = heap.core();
    let stack = core.roots;
    let mut marker = Marker::new(&lens, core.epoch);
    stack.for_each(|value| marker.mark_value(value));
    roots.mark(&mut marker);
    while let Some((arena, index)) = marker.pop() {
        heap.mark_item(arena, index, &mut marker);
    }

    let core = heap.core();
    core.epoch = core.epoch.wrapping_add(1);
    let forwarding = marker.into_forwarding(core.epoch);
    heap.compact_all(&forwarding);
    roots.relocate(&forwarding);
    stack.update(|value| forwarding.relocated(value));

    let (objects, bytes) = heap.size();
    let lens = heap.arena_lens();
    let core = heap.core();
    core.barrier.after_collection(&lens);
    let pause = start.elapsed();
    let m = &mut core.metrics;
    m.collections += 1;
    m.last_pause = pause;
    m.max_pause = m.max_pause.max(pause);
    m.total_pause += pause;
    m.objects = objects;
    m.bytes = bytes;
    m.objects_after_collection = objects;
    m.bytes_after_collection = bytes;
    m.allocated_since_collection = 0;
}

/// Used by the `heap!` macro's mark dispatch. Not part of the public API.
#[doc(hidden)]
#[inline]
pub fn __mark_item<T: Trace>(arena: &Arena<T>, index: usize, marker: &mut Marker) {
    arena.items[index].mark(marker);
}

/// Used by the `heap!` macro's compaction. Not part of the public API.
#[doc(hidden)]
pub fn __compact<T: Trace>(arena: &mut Arena<T>, forwarding: &Forwarding) {
    let mut index = 0;
    arena.items.retain(|_| {
        let keep = forwarding.is_live(T::ARENA, index);
        index += 1;
        keep
    });
    for item in &mut arena.items {
        item.relocate(forwarding);
    }
    arena.epoch = forwarding.epoch();
}

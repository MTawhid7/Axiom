//! Typed arenas, plus the per-heap bookkeeping (`HeapCore`) that the `heap!`
//! macro embeds.

use core::ops::{Index, IndexMut};
use core::time::Duration;
use std::time::Instant;

use crate::handle::Handle;
use crate::roots::{RootFrame, RootStack};
use crate::trace::{Allocated, Forwarding, Marker, Roots, Trace};

/// One arena: a dense `Vec` of one object type, addressed by [`Handle`]s.
#[derive(Debug)]
pub struct Arena<T> {
    items: Vec<T>,
    epoch: u32,
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

/// When a heap wants to collect: the same policy as Pattern A's `Pacing`, so
/// that the two patterns are comparable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pacing {
    pub min_bytes: usize,
    pub growth_percent: usize,
}

impl Pacing {
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
#[derive(Debug, Default)]
pub struct HeapCore {
    roots: RootStack,
    pacing: Pacing,
    epoch: u32,
    metrics: Metrics,
}

impl HeapCore {
    #[must_use]
    pub fn new(pacing: Pacing) -> Self {
        Self {
            pacing,
            ..Self::default()
        }
    }

    /// Open a root frame (see [`RootFrame`]).
    #[must_use]
    pub fn frame(&self) -> RootFrame {
        self.roots.frame()
    }

    /// The number of rooted handles currently on the stack.
    #[must_use]
    pub fn rooted(&self) -> usize {
        self.roots.len()
    }

    /// Whether the allocation budget since the last collection is spent.
    #[must_use]
    pub fn wants_collection(&self) -> bool {
        self.metrics.allocated_since_collection
            >= self.pacing.budget(self.metrics.bytes_after_collection)
    }

    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.metrics
    }

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
pub trait HeapLayout {
    /// The length of each arena, in `ARENA` order.
    fn arena_lens(&self) -> Vec<usize>;
    /// Call `Trace::mark` on item `index` of arena `arena`.
    fn mark_item(&self, arena: usize, index: usize, marker: &mut Marker);
    /// Compact every arena and relocate the handles inside the survivors.
    fn compact_all(&mut self, forwarding: &Forwarding);
    /// (objects, bytes) currently stored.
    fn size(&self) -> (usize, usize);
    fn core(&mut self) -> &mut HeapCore;
}

/// Allocate `value` in `heap` without a safepoint: this never collects.
#[inline]
pub fn alloc<H, T>(heap: &mut H, value: T) -> Handle<'static, T>
where
    H: HeapLayout + Store<T>,
    T: Allocated,
{
    heap.core().note_allocation(size_of::<T>());
    let arena = heap.arena_mut();
    let handle = Handle::new(arena.items.len(), arena.epoch);
    arena.items.push(value);
    handle
}

/// A full stop-the-world mark–compact collection.
///
/// 1. Mark from the root stack and `roots`, with a worklist.
/// 2. Rank the survivors in each arena.
/// 3. Compact every arena in place, keeping order, so a survivor's rank is its
///    new index; dead objects are dropped.
/// 4. Relocate every handle, in heap data, roots and root slots, through the ranks.
pub fn collect_heap<H, R>(heap: &mut H, roots: &mut R)
where
    H: HeapLayout + ?Sized,
    R: Roots + ?Sized,
{
    let start = Instant::now();
    let lens = heap.arena_lens();
    let core = heap.core();
    let mut marker = Marker::new(&lens, core.epoch);
    core.roots.mark(&mut marker);
    roots.mark(&mut marker);
    while let Some((arena, index)) = marker.pop() {
        heap.mark_item(arena, index, &mut marker);
    }

    let core = heap.core();
    core.epoch = core.epoch.wrapping_add(1);
    let forwarding = marker.into_forwarding(core.epoch);
    heap.compact_all(&forwarding);
    roots.relocate(&forwarding);

    let (objects, bytes) = heap.size();
    let core = heap.core();
    core.roots.relocate(&forwarding);
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

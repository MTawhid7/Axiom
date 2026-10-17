//! The heap: allocation, accounting, and the stop-the-world mark–sweep collector.

use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr::NonNull;
use std::alloc;
use std::time::{Duration, Instant};

use super::gc::{Color, GcBox, GcHeader, vtable_of};
use super::trace::{Trace, Tracer};
use crate::arena::Pacing;
use crate::metrics::Metrics;

/// A marker that makes `'a` invariant.
pub(crate) type Invariant<'a> = PhantomData<Cell<&'a ()>>;

/// The capability to allocate in, and write to, one arena for the length of one
/// mutation scope `'gc`.
#[repr(transparent)]
pub struct Mutation<'gc> {
    heap: Heap,
    _brand: Invariant<'gc>,
}

impl Mutation<'_> {
    pub(crate) const fn new(heap: Heap) -> Self {
        Self {
            heap,
            _brand: PhantomData,
        }
    }

    pub(crate) const fn heap(&self) -> &Heap {
        &self.heap
    }

    pub(crate) const fn heap_mut(&mut self) -> &mut Heap {
        &mut self.heap
    }

    /// Whether the allocation budget since the last collection is spent.
    ///
    /// Collection never happens inside a mutation scope (I1). A long task should
    /// check this and *yield*: return from the scope so that the driver can call
    /// [`Arena::collect_debt`](crate::Arena::collect_debt).
    #[inline]
    pub fn wants_collection(&self) -> bool {
        self.heap.wants_collection()
    }

    /// Current heap metrics.
    pub fn metrics(&self) -> Metrics {
        self.heap.metrics()
    }
}

impl core::fmt::Debug for Mutation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Mutation").finish_non_exhaustive()
    }
}

/// Every allocation, plus accounting. Shared (`&`) during mutation;
/// exclusive (`&mut`) during collection.
pub(crate) struct Heap {
    /// Head of the intrusive list of every allocated box: reachable, garbage
    /// not yet collected, or dead but kept for weak references.
    all: Cell<Option<NonNull<GcHeader>>>,
    /// Mark stack. Only used during a collection; kept to reuse its capacity.
    gray: Vec<NonNull<GcHeader>>,
    pacing: Pacing,
    objects: Cell<usize>,
    bytes: Cell<usize>,
    peak_objects: Cell<usize>,
    peak_bytes: Cell<usize>,
    allocated_since_collection: Cell<usize>,
    total_allocated_objects: Cell<u64>,
    total_allocated_bytes: Cell<u64>,
    barriers: Cell<u64>,
    collections: u64,
    last_pause: Duration,
    max_pause: Duration,
    total_pause: Duration,
    objects_after_collection: usize,
    bytes_after_collection: usize,
}

impl Heap {
    pub(crate) const fn new(pacing: Pacing) -> Self {
        Self {
            all: Cell::new(None),
            gray: Vec::new(),
            pacing,
            objects: Cell::new(0),
            bytes: Cell::new(0),
            peak_objects: Cell::new(0),
            peak_bytes: Cell::new(0),
            allocated_since_collection: Cell::new(0),
            total_allocated_objects: Cell::new(0),
            total_allocated_bytes: Cell::new(0),
            barriers: Cell::new(0),
            collections: 0,
            last_pause: Duration::ZERO,
            max_pause: Duration::ZERO,
            total_pause: Duration::ZERO,
            objects_after_collection: 0,
            bytes_after_collection: 0,
        }
    }

    /// Box `value` and link it into the list of all allocations.
    pub(super) fn allocate<T: Trace>(&self, value: T) -> NonNull<GcBox<T>> {
        let vtable = vtable_of::<T>();
        let layout = vtable.layout;
        // SAFETY: `layout` is `GcBox<T>`'s, which is never zero-sized because
        // it contains a `GcHeader`.
        let raw = unsafe { alloc::alloc(layout) };
        let Some(ptr) = NonNull::new(raw.cast::<GcBox<T>>()) else {
            alloc::handle_alloc_error(layout)
        };
        let header = GcHeader {
            next: Cell::new(self.all.get()),
            color: Cell::new(Color::White),
            weakly_referenced: Cell::new(false),
            dead: Cell::new(false),
            vtable,
        };
        // SAFETY: `ptr` was just allocated with `GcBox<T>`'s layout, so it is
        // valid for writes and aligned.
        unsafe { ptr.as_ptr().write(GcBox { header, value }) };
        self.all.set(Some(ptr.cast()));

        let objects = self.objects.get() + 1;
        let bytes = self.bytes.get() + layout.size();
        self.objects.set(objects);
        self.bytes.set(bytes);
        self.peak_objects.set(self.peak_objects.get().max(objects));
        self.peak_bytes.set(self.peak_bytes.get().max(bytes));
        self.allocated_since_collection
            .set(self.allocated_since_collection.get() + layout.size());
        self.total_allocated_objects
            .set(self.total_allocated_objects.get() + 1);
        self.total_allocated_bytes
            .set(self.total_allocated_bytes.get() + layout.size() as u64);
        ptr
    }

    /// The write barrier. It is a counter, because this collector is not
    /// incremental (see [`Gc::write`](crate::Gc::write)).
    #[inline]
    pub(super) fn write_barrier(&self) {
        self.barriers.set(self.barriers.get() + 1);
    }

    pub(crate) fn wants_collection(&self) -> bool {
        self.allocated_since_collection.get() >= self.pacing.budget(self.bytes_after_collection)
    }

    pub(crate) fn metrics(&self) -> Metrics {
        Metrics {
            objects: self.objects.get(),
            bytes: self.bytes.get(),
            peak_objects: self.peak_objects.get(),
            peak_bytes: self.peak_bytes.get(),
            allocated_since_collection: self.allocated_since_collection.get(),
            total_allocated_objects: self.total_allocated_objects.get(),
            total_allocated_bytes: self.total_allocated_bytes.get(),
            barriers: self.barriers.get(),
            collections: self.collections,
            last_pause: self.last_pause,
            max_pause: self.max_pause,
            total_pause: self.total_pause,
            objects_after_collection: self.objects_after_collection,
            bytes_after_collection: self.bytes_after_collection,
        }
    }

    /// A full, stop-the-world collection, rooted at `root`.
    ///
    /// Taking `&mut self` is what enforces I1: no mutation scope can be open.
    pub(crate) fn collect<R: Trace + ?Sized>(&mut self, root: &R) {
        let start = Instant::now();
        self.mark(root);
        self.sweep();
        let pause = start.elapsed();
        self.collections += 1;
        self.last_pause = pause;
        self.max_pause = self.max_pause.max(pause);
        self.total_pause += pause;
        self.objects_after_collection = self.objects.get();
        self.bytes_after_collection = self.bytes.get();
        self.allocated_since_collection.set(0);
    }

    fn mark<R: Trace + ?Sized>(&mut self, root: &R) {
        let mut tracer = Tracer::new(&mut self.gray);
        root.trace(&mut tracer);
        while let Some(header) = tracer.pop() {
            // SAFETY: only allocated boxes are pushed (`Tracer::mark`'s contract).
            let h = unsafe { header.as_ref() };
            // SAFETY: gray boxes are reachable, so by I6 their value is undropped;
            // the vtable belongs to this box.
            unsafe { (h.vtable.trace)(header, &mut tracer) };
            h.color.set(Color::Black);
        }
    }

    /// Free or drop everything left unmarked, and reset the marks of the rest.
    fn sweep(&mut self) {
        let mut link: &Cell<Option<NonNull<GcHeader>>> = &self.all;
        while let Some(header) = link.get() {
            // SAFETY: every box on the list is allocated until it is unlinked below.
            let h = unsafe { header.as_ref() };
            let weakly_referenced = h.weakly_referenced.replace(false);
            if h.color.replace(Color::White) != Color::White {
                link = &h.next; // reachable: keep
                continue;
            }
            // Unreachable. Mark it dead *before* dropping, so that a panic inside
            // `Drop` can never lead to a second drop.
            if !h.dead.replace(true) {
                // SAFETY: the value is initialized (it was not dead), and since it
                // is unreachable (I3) nothing can use it again.
                unsafe { (h.vtable.drop_value)(header) };
            }
            if weakly_referenced {
                link = &h.next; // a reachable `GcWeak` still points here (I6): keep the memory
                continue;
            }
            link.set(h.next.get());
            let layout = h.vtable.layout;
            self.objects.set(self.objects.get() - 1);
            self.bytes.set(self.bytes.get() - layout.size());
            // SAFETY: allocated by `allocate` with this layout, and now unlinked, so
            // it is never touched again. `h` is not used after this point.
            unsafe { alloc::dealloc(header.as_ptr().cast(), layout) };
        }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        let mut next = self.all.take();
        while let Some(header) = next {
            // SAFETY: every box on the list is allocated, and each is freed exactly once.
            let h = unsafe { header.as_ref() };
            next = h.next.get();
            let layout = h.vtable.layout;
            if !h.dead.replace(true) {
                // SAFETY: the arena is being dropped, so no mutation scope or root
                // can observe the value again. It was initialized (not dead).
                unsafe { (h.vtable.drop_value)(header) };
            }
            // SAFETY: allocated by `allocate` with this layout; never touched again.
            unsafe { alloc::dealloc(header.as_ptr().cast(), layout) };
        }
    }
}

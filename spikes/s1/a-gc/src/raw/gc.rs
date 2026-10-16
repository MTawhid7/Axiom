//! Boxes, headers, vtables, and the `Gc` / `GcWeak` pointers.

use core::alloc::Layout;
use core::cell::Cell;
use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::{self, NonNull};

use super::heap::{Invariant, Mutation};
use super::lock::Write;
use super::trace::{Trace, Tracer};

/// Tri-colour marking state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Color {
    /// Not (yet) reached in the current collection.
    White,
    /// Reached; its children have not been traced yet.
    Gray,
    /// Reached, and its children have been traced.
    Black,
}

/// Bytes of header in front of every value: `size_of::<GcBox<T>>()` equals this
/// plus `size_of::<T>()`, for any 8-byte-aligned `T`.
pub const BOX_HEADER_BYTES: usize = size_of::<GcHeader>();

/// The per-object header, first in every box (I5).
pub(super) struct GcHeader {
    /// The next box in the heap's list of every allocation.
    pub(super) next: Cell<Option<NonNull<GcHeader>>>,
    pub(super) color: Cell<Color>,
    /// A `GcWeak` to this box was traced in the current collection.
    pub(super) weakly_referenced: Cell<bool>,
    /// The value has been dropped. The memory is kept only for `GcWeak`s (I6).
    pub(super) dead: Cell<bool>,
    pub(super) vtable: &'static VTable,
}

/// The type-erased operations of a `GcBox<T>`.
pub(super) struct VTable {
    pub(super) layout: Layout,
    pub(super) needs_trace: bool,
    /// # Safety
    /// The header must belong to an allocated `GcBox<T>` (this vtable's `T`)
    /// whose value has not been dropped.
    pub(super) trace: unsafe fn(NonNull<GcHeader>, &mut Tracer<'_>),
    /// # Safety
    /// As `trace`. The value must never be used afterwards.
    pub(super) drop_value: unsafe fn(NonNull<GcHeader>),
}

#[repr(C)]
pub(super) struct GcBox<T> {
    pub(super) header: GcHeader,
    pub(super) value: T,
}

/// One `'static` vtable per traced type, made by const promotion.
trait HasVTable {
    const VTABLE: &'static VTable;
}

impl<T: Trace> HasVTable for T {
    const VTABLE: &'static VTable = &VTable {
        layout: Layout::new::<GcBox<T>>(),
        needs_trace: T::NEEDS_TRACE,
        trace: trace_value::<T>,
        drop_value: drop_value::<T>,
    };
}

pub(super) fn vtable_of<T: Trace>() -> &'static VTable {
    <T as HasVTable>::VTABLE
}

/// # Safety
/// `header` must belong to an allocated `GcBox<T>` whose value has not been dropped.
unsafe fn trace_value<T: Trace>(header: NonNull<GcHeader>, tracer: &mut Tracer<'_>) {
    // SAFETY: by the contract, and I5: the header starts a `#[repr(C)]` `GcBox<T>`.
    let value = unsafe { &header.cast::<GcBox<T>>().as_ref().value };
    value.trace(tracer);
}

/// # Safety
/// As `trace_value`, and the value is never used again.
unsafe fn drop_value<T>(header: NonNull<GcHeader>) {
    // SAFETY: by the contract and I5: the header starts an allocated `GcBox<T>`.
    let value = unsafe { &raw mut (*header.cast::<GcBox<T>>().as_ptr()).value };
    // SAFETY: the value is initialized (contract), and no reference to it is live,
    // because it is unreachable (I3). So dropping it in place is sound.
    unsafe { ptr::drop_in_place(value) };
}

/// A pointer to a garbage-collected `T`, branded with its mutation scope `'gc`.
///
/// `Gc` is `Copy` and pointer-sized. It dereferences to `&T`, and writes go
/// through interior mutability behind a write barrier ([`Gc::write`]).
pub struct Gc<'gc, T> {
    ptr: NonNull<GcBox<T>>,
    _brand: Invariant<'gc>,
}

impl<'gc, T: Trace + 'gc> Gc<'gc, T> {
    /// Allocate `value` in `mc`'s arena.
    #[inline]
    pub fn new(mc: &Mutation<'gc>, value: T) -> Self {
        Self {
            ptr: mc.heap().allocate(value),
            _brand: PhantomData,
        }
    }
}

impl<'gc, T: 'gc> Gc<'gc, T> {
    /// The value, for the whole mutation scope.
    #[inline]
    #[expect(
        clippy::should_implement_trait,
        reason = "gc-arena's API. `AsRef` cannot express it: it returns `&'gc T`, which outlives `&self`"
    )]
    pub fn as_ref(this: Self) -> &'gc T {
        // SAFETY: I1 and I6. `this` exists inside a mutation scope of its arena,
        // nothing is collected during the scope, and the box is allocated with
        // its value undropped.
        unsafe { &this.ptr.as_ref().value }
    }

    /// Whether both pointers name the same object.
    #[inline]
    pub fn ptr_eq(this: Self, other: Self) -> bool {
        this.ptr == other.ptr
    }

    /// The object's address, for identity hashing or debugging.
    #[inline]
    pub fn as_ptr(this: Self) -> *const T {
        ptr::from_ref(Gc::as_ref(this))
    }

    /// A weak pointer to the same object.
    #[inline]
    pub fn downgrade(this: Self) -> GcWeak<'gc, T> {
        GcWeak {
            ptr: this.ptr,
            _brand: PhantomData,
        }
    }

    /// Write access to the object: applies its write barrier, then returns the
    /// proof ([`Write`]) that unlocks its [`Lock`](crate::Lock) and
    /// [`RefLock`](crate::RefLock) fields (see [`unlock!`](crate::unlock)).
    ///
    /// This collector is not incremental, so the barrier only counts writes. It
    /// is kept for two reasons:
    /// - it measures gc-arena's *authentic* ceremony;
    /// - an incremental collector would rely on it.
    #[inline]
    pub fn write(mc: &Mutation<'gc>, this: Self) -> &'gc Write<T> {
        mc.heap().write_barrier();
        // SAFETY: this object's barrier was just applied, in the current
        // mutation scope (`Write`'s contract).
        unsafe { Write::assume(Gc::as_ref(this)) }
    }
}

impl<T> Clone for Gc<'_, T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Gc<'_, T> {}

impl<'gc, T: 'gc> Deref for Gc<'gc, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        Gc::as_ref(*self)
    }
}

impl<T> fmt::Debug for Gc<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gc({:p})", self.ptr)
    }
}

// SAFETY: reports its only pointer (I3). It has no `Drop`.
unsafe impl<'gc, T: 'gc> Trace for Gc<'gc, T> {
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        // SAFETY: I6. During a collection, every `Gc` reachable from the root
        // points to an allocated box.
        unsafe { tracer.mark(self.ptr.cast()) };
    }
}

/// A weak pointer: it doesn't keep its target alive, and [`upgrade`](Self::upgrade)
/// fails once the target has been collected.
pub struct GcWeak<'gc, T> {
    ptr: NonNull<GcBox<T>>,
    _brand: Invariant<'gc>,
}

impl<'gc, T: 'gc> GcWeak<'gc, T> {
    fn header(self) -> &'gc GcHeader {
        // SAFETY: I6. The box stays allocated while any reachable `GcWeak` points
        // to it, and I5 says the header comes first.
        unsafe { self.ptr.cast::<GcHeader>().as_ref() }
    }

    /// A strong pointer, unless the target has been collected.
    ///
    /// It takes `mc` to match gc-arena's API. An incremental collector would
    /// apply a barrier here, because upgrading can resurrect an object that the
    /// collector has not marked.
    #[inline]
    pub fn upgrade(self, mc: &Mutation<'gc>) -> Option<Gc<'gc, T>> {
        let _ = mc;
        (!self.header().dead.get()).then_some(Gc {
            ptr: self.ptr,
            _brand: PhantomData,
        })
    }

    /// Whether the target has been collected (its value dropped).
    #[inline]
    pub fn is_dead(self) -> bool {
        self.header().dead.get()
    }

    /// Whether both pointers name the same object.
    #[inline]
    pub fn ptr_eq(this: Self, other: Self) -> bool {
        this.ptr == other.ptr
    }
}

impl<T> Clone for GcWeak<'_, T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for GcWeak<'_, T> {}

impl<T> fmt::Debug for GcWeak<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GcWeak({:p})", self.ptr)
    }
}

// SAFETY: reports its only pointer, as weak (I3, I6). It has no `Drop`.
unsafe impl<'gc, T: 'gc> Trace for GcWeak<'gc, T> {
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        // SAFETY: I6. A reachable `GcWeak` points to an allocated box.
        unsafe { tracer.mark_weak(self.ptr.cast()) };
    }
}

#[cfg(test)]
mod tests {
    use super::{GcBox, GcHeader};

    /// Per-object memory overhead, recorded for the S1 memory comparison:
    /// two pointers (list link, vtable) and three one-byte flags, padded to 8.
    #[test]
    fn header_overhead_is_three_words() {
        assert_eq!(size_of::<GcHeader>(), 24);
        assert_eq!(size_of::<GcBox<u64>>(), 32);
        assert_eq!(size_of::<GcBox<()>>(), 24);
    }
}

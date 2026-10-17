//! C1's root stack (ADR-0008 SH7): the only `unsafe` in `s3-gc`.
//!
//! The crate root declares `#![deny(unsafe_code)]`, and only this module
//! re-allows it, which `tools/check-unsafe-policy.sh` enforces. Miri checks it
//! under Stacked Borrows and Tree Borrows with strict provenance.
//!
//! **Design.** One allocation of `capacity` slots, each a `Cell<Value>` of 8
//! bytes, made zeroed so that the system can commit its pages lazily. A push
//! is a capacity check and two stores; a [`Scoped`](crate::Scoped) keeps a
//! reference to its slot, so reading it back is one load. There is no borrow
//! tracking: Pattern B's scope tokens already keep handles from being used
//! across a collection, and `Cell` makes mutation through shared references
//! sound. The `unsafe` is only in obtaining the slots: allocating them zeroed,
//! viewing them as a slice, and freeing them.
//!
//! The capacity is fixed: pushing beyond it panics, as overflowing a thread's
//! stack does. The default, 1,048,576 slots, reserves 8 MiB of address space
//! and commits only the pages actually touched.
//!
//! # Invariants
//!
//! Every `SAFETY:` comment in this module refers to these.
//!
//! - **R1: the allocation.** `slots` points to an allocation made by
//!   `alloc_zeroed` with `layout(capacity)` in [`RawStack::with_capacity`],
//!   of nonzero size, and freed only in `Drop`.
//! - **R2: every slot is initialized.** Zeroed bytes are a valid `Value`
//!   (`+0.0`), since `Value` is a `repr(transparent)` `u64`, and `Cell<Value>`
//!   has the same layout as `Value`. Slots are only ever written through
//!   `Cell::set`, with valid values.
//! - **R3: only shared references.** The module never creates a `&mut` to a
//!   slot, so shared references may coexist and mutate through `Cell`. Each
//!   one is derived from `&self` and cannot outlive it, so none survives into
//!   `Drop`.
#![allow(unsafe_code)]

use core::alloc::Layout;
use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;
use std::alloc;

use crate::roots::RootStack;
use crate::value::Value;

/// The default capacity, in slots.
#[cfg(not(miri))]
pub const DEFAULT_CAPACITY: usize = 1 << 20;
/// Smaller under Miri, which zeroes the allocation eagerly.
#[cfg(miri)]
pub const DEFAULT_CAPACITY: usize = 1 << 12;

/// C1's root stack. See the [module docs](self).
pub struct RawStack {
    len: Cell<usize>,
    slots: NonNull<Cell<Value>>,
    capacity: usize,
}

fn layout(capacity: usize) -> Layout {
    Layout::array::<Cell<Value>>(capacity).expect("root stack capacity overflows isize")
}

impl RawStack {
    /// A stack of `capacity` slots.
    ///
    /// # Panics
    /// If `capacity` is zero or its byte size overflows `isize`.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "a root stack needs at least one slot");
        let layout = layout(capacity);
        // SAFETY: R1. The layout has nonzero size: `capacity` > 0 slots of 8 bytes.
        let ptr = unsafe { alloc::alloc_zeroed(layout) };
        #[expect(
            clippy::cast_ptr_alignment,
            reason = "alloc_zeroed returns memory aligned for the layout, which is Cell<Value>'s"
        )]
        let slots = NonNull::new(ptr.cast::<Cell<Value>>())
            .unwrap_or_else(|| alloc::handle_alloc_error(layout));
        Self {
            len: Cell::new(0),
            slots,
            capacity,
        }
    }

    /// The number of slots.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    fn slots(&self) -> &[Cell<Value>] {
        // SAFETY: R1: `slots` is non-null, aligned for `Cell<Value>`, and
        // points to one live allocation of `capacity` slots, which is no more
        // than `isize::MAX` bytes (`layout` checked it). R2: every slot is an
        // initialized `Cell<Value>`. R3: the slice is shared and borrows
        // `self`, so it cannot outlive the allocation or alias a `&mut`.
        unsafe { core::slice::from_raw_parts(self.slots.as_ptr(), self.capacity) }
    }
}

impl Default for RawStack {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl Drop for RawStack {
    fn drop(&mut self) {
        // SAFETY: R1: the pointer came from `alloc_zeroed` with this layout and
        // has not been freed. R3: `drop` has `&mut self`, so no slot reference
        // derived from `&self` is still alive.
        unsafe { alloc::dealloc(self.slots.as_ptr().cast(), layout(self.capacity)) }
    }
}

impl RootStack for RawStack {
    type Slot<'s> = &'s Cell<Value>;

    #[inline]
    fn height(&self) -> usize {
        self.len.get()
    }

    #[inline]
    fn push(&self, value: Value) -> &Cell<Value> {
        let index = self.len.get();
        let slot = self.slots().get(index).expect("root stack overflow");
        slot.set(value);
        self.len.set(index + 1);
        slot
    }

    #[inline]
    fn read<'s>(slot: &'s Cell<Value>) -> Value
    where
        Self: 's,
    {
        slot.get()
    }

    /// Only ever shrinks, like `Vec::truncate`.
    #[inline]
    fn truncate(&self, height: usize) {
        if height < self.len.get() {
            self.len.set(height);
        }
    }

    fn for_each(&self, mut f: impl FnMut(Value)) {
        for slot in &self.slots()[..self.len.get()] {
            f(slot.get());
        }
    }

    fn update(&self, mut f: impl FnMut(Value) -> Value) {
        for slot in &self.slots()[..self.len.get()] {
            slot.set(f(slot.get()));
        }
    }
}

impl fmt::Debug for RawStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawStack")
            .field("len", &self.len.get())
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

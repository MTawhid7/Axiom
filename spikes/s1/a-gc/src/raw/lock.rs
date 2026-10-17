//! Interior mutability that can be written only through a write barrier.

use core::cell::{BorrowError, Cell, Ref, RefCell, RefMut};
use core::fmt;
use core::ops::Deref;
use core::ptr;

use super::gc::Gc;
use super::heap::Mutation;
use super::trace::{Trace, Tracer};

/// A `Cell` whose writes go through a write barrier.
///
/// Read it with [`Lock::get`]. Write it through [`Write<Lock<T>>::unlock`],
/// which you get from [`Gc::write`] plus [`unlock!`](crate::unlock), or through
/// [`Gc::set`] when the `Lock` is the whole object.
#[derive(Default)]
#[repr(transparent)]
pub struct Lock<T> {
    cell: Cell<T>,
}

impl<T> Lock<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            cell: Cell::new(value),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }
}

impl<T: Copy> Lock<T> {
    #[inline]
    pub fn get(&self) -> T {
        self.cell.get()
    }
}

impl<T> fmt::Debug for Lock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lock").finish_non_exhaustive()
    }
}

/// A `RefCell` whose mutable borrows go through a write barrier.
///
/// Borrow it shared with [`RefLock::borrow`]. Borrow it mutably through
/// [`Write<RefLock<T>>::unlock`], or through [`Gc::borrow_mut`] when the
/// `RefLock` is the whole object.
#[derive(Default)]
#[repr(transparent)]
pub struct RefLock<T> {
    cell: RefCell<T>,
}

impl<T> RefLock<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            cell: RefCell::new(value),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }

    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        self.cell.borrow()
    }

    #[inline]
    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
        self.cell.try_borrow()
    }
}

impl<T> fmt::Debug for RefLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefLock").finish_non_exhaustive()
    }
}

/// Proof that the write barrier of the `Gc` object containing a `T` has been
/// applied in the current mutation scope.
///
/// It is obtained from [`Gc::write`] and narrowed to a field with
/// [`field!`](crate::field). It unlocks [`Lock`] and [`RefLock`] for writing.
#[repr(transparent)]
#[non_exhaustive]
pub struct Write<T> {
    /// Public only so that [`field!`](crate::field) can pattern-match through it,
    /// which it does with `..`. `#[non_exhaustive]` makes a `Write` impossible to
    /// construct outside this crate.
    #[doc(hidden)]
    pub __inner: T,
}

impl<T> Write<T> {
    /// # Safety
    ///
    /// The write barrier of the `Gc` object that contains `value` must have been
    /// applied in the current mutation scope, by [`Gc::write`]. It is required for
    /// soundness only under an incremental collector, but it is part of the
    /// contract regardless.
    #[inline]
    pub unsafe fn assume(value: &T) -> &Self {
        // SAFETY: `Write<T>` is `#[repr(transparent)]` over `T`.
        unsafe { &*ptr::from_ref(value).cast::<Self>() }
    }

    #[inline]
    pub fn get(&self) -> &T {
        &self.__inner
    }
}

impl<T> Deref for Write<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.__inner
    }
}

impl<T> Write<Lock<T>> {
    /// The barrier was applied, so writing is allowed.
    #[inline]
    pub fn unlock(&self) -> &Cell<T> {
        &self.__inner.cell
    }
}

impl<T> Write<RefLock<T>> {
    /// The barrier was applied, so mutable borrowing is allowed.
    #[inline]
    pub fn unlock(&self) -> &RefCell<T> {
        &self.__inner.cell
    }
}

impl<T> fmt::Debug for Write<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Write").finish_non_exhaustive()
    }
}

impl<'gc, T: 'gc> Gc<'gc, Lock<T>> {
    /// Set the value, applying the barrier.
    #[inline]
    pub fn set(self, mc: &Mutation<'gc>, value: T) {
        Gc::write(mc, self).unlock().set(value);
    }
}

impl<'gc, T: 'gc> Gc<'gc, RefLock<T>> {
    /// Borrow the value for as long as the mutation scope lasts.
    #[inline]
    pub fn borrow(self) -> Ref<'gc, T> {
        Gc::as_ref(self).borrow()
    }

    /// Borrow mutably, applying the barrier.
    #[inline]
    pub fn borrow_mut(self, mc: &Mutation<'gc>) -> RefMut<'gc, T> {
        Gc::write(mc, self).unlock().borrow_mut()
    }
}

/// Narrow a `&Write<Struct>` to a `&Write<FieldType>` for one of its fields.
///
/// ```ignore
/// field!(Gc::write(mc, node), Node, parent)
/// ```
///
/// The struct pattern makes the macro reject any projection that isn't a direct
/// field: it cannot go through `Deref`, a pointer, or an enum variant. So the
/// barrier that was applied always belongs to the object that owns the field.
#[macro_export]
macro_rules! field {
    ($value:expr, $type:path, $field:ident) => {
        (match *$value {
            $crate::Write {
                __inner: $type { ref $field, .. },
                ..
            } => {
                // SAFETY: `$value: &Write<$type>` proves the barrier of the object
                // that owns `$field` was applied in this mutation scope, and the
                // struct pattern ensures `$field` is a direct field of that object.
                unsafe { $crate::Write::assume($field) }
            }
        })
    };
}

/// `field!(...).unlock()`: write access to a [`Lock`] or [`RefLock`] field.
///
/// ```ignore
/// unlock!(Gc::write(mc, node), Node, parent).set(Some(new_parent));
/// ```
#[macro_export]
macro_rules! unlock {
    ($value:expr, $type:path, $field:ident) => {
        ($crate::field!($value, $type, $field)).unlock()
    };
}

// SAFETY: traces the contents (I3). Its `Drop` is `Cell`'s, which only drops the contents (I4).
unsafe impl<T: Trace> Trace for Lock<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        // SAFETY: tracing runs only during a collection, outside every mutation
        // scope (I1). No `Cell` write can run and no `&mut T` exists, so a shared
        // read is sound.
        unsafe { &*self.cell.as_ptr() }.trace(tracer);
    }
}

// SAFETY: traces the contents (I3). Its `Drop` is `RefCell`'s, which only drops the contents (I4).
unsafe impl<T: Trace> Trace for RefLock<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;

    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        // SAFETY: as for `Lock`. No borrow is active during a collection; even a
        // leaked `RefMut` no longer holds a usable reference.
        unsafe { &*self.cell.as_ptr() }.trace(tracer);
    }
}

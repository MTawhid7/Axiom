//! The `Trace` trait, the `Tracer`, and `Trace` impls for std types.

use core::cell::{Cell, RefCell};
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;
use std::collections::VecDeque;

use super::gc::{Color, GcHeader};

/// A type whose `Gc` references the collector can trace.
///
/// # Safety
///
/// An implementation must satisfy three requirements:
/// 1. **Complete tracing (I3).** `trace` calls `trace` on **every** `Gc` and
///    `GcWeak` owned by `self`, whether directly or through owned non-`Gc` data.
/// 2. **`NEEDS_TRACE` is honest.** It may be `false` only if the type can never
///    contain a `Gc` or a `GcWeak`.
/// 3. **No `Gc` dereference in `Drop` (I4).** If the type has a `Drop` impl, it
///    must not dereference a `Gc`.
///
/// Prefer `#[derive(Trace)]`, which satisfies all three by construction.
pub unsafe trait Trace {
    /// `false` if tracing this type can never find a `Gc`. Containers use it to
    /// skip whole subtrees, and the collector uses it to blacken objects
    /// without visiting them.
    const NEEDS_TRACE: bool = true;

    fn trace(&self, tracer: &mut Tracer<'_>);
}

/// Accumulates mark work during a collection. Only the collector can create one.
pub struct Tracer<'a> {
    gray: &'a mut Vec<NonNull<GcHeader>>,
}

impl<'a> Tracer<'a> {
    pub(super) const fn new(gray: &'a mut Vec<NonNull<GcHeader>>) -> Self {
        Self { gray }
    }

    pub(super) fn pop(&mut self) -> Option<NonNull<GcHeader>> {
        self.gray.pop()
    }

    /// Mark a box as strongly reachable.
    ///
    /// # Safety
    ///
    /// `header` must belong to an allocated box (I6).
    #[inline]
    pub(super) unsafe fn mark(&mut self, header: NonNull<GcHeader>) {
        // SAFETY: by the contract.
        let h = unsafe { header.as_ref() };
        if h.color.get() == Color::White {
            if h.vtable.needs_trace {
                h.color.set(Color::Gray);
                self.gray.push(header);
            } else {
                h.color.set(Color::Black);
            }
        }
    }

    /// Record that a `GcWeak` to this box is reachable: the sweep then keeps its
    /// memory even if the value dies (I6).
    ///
    /// # Safety
    ///
    /// `header` must belong to an allocated box (I6).
    #[inline]
    #[expect(
        clippy::unused_self,
        reason = "the `&mut Tracer` receiver is the capability proving a collection is in progress"
    )]
    pub(super) unsafe fn mark_weak(&mut self, header: NonNull<GcHeader>) {
        // SAFETY: by the contract.
        unsafe { header.as_ref() }.weakly_referenced.set(true);
    }
}

impl fmt::Debug for Tracer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tracer")
            .field("pending", &self.gray.len())
            .finish()
    }
}

/// Types that can never contain a `Gc`.
macro_rules! trace_leaf {
    ($($t:ty),* $(,)?) => {$(
        // SAFETY: the type can never contain a `Gc` or a `GcWeak`, and its
        // `Drop`, if it has one, cannot dereference one.
        unsafe impl Trace for $t {
            const NEEDS_TRACE: bool = false;
            #[inline]
            fn trace(&self, _: &mut Tracer<'_>) {}
        }
    )*};
}

trace_leaf!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    str,
    String,
    core::time::Duration,
);

// SAFETY: `'static` data can't hold a `Gc<'gc, _>` for every `'gc` (I2, I4).
unsafe impl<T: ?Sized + 'static> Trace for &'static T {
    const NEEDS_TRACE: bool = false;
    #[inline]
    fn trace(&self, _: &mut Tracer<'_>) {}
}

// SAFETY: `PhantomData` owns nothing.
unsafe impl<T: ?Sized> Trace for PhantomData<T> {
    const NEEDS_TRACE: bool = false;
    #[inline]
    fn trace(&self, _: &mut Tracer<'_>) {}
}

// SAFETY: only `'static` contents. A `Cell` holding a `Gc` would allow writes
// without a barrier, so such a `Cell` is deliberately *not* `Trace` (I2, I4).
unsafe impl<T: ?Sized + 'static> Trace for Cell<T> {
    const NEEDS_TRACE: bool = false;
    #[inline]
    fn trace(&self, _: &mut Tracer<'_>) {}
}

// SAFETY: as for `Cell`.
unsafe impl<T: ?Sized + 'static> Trace for RefCell<T> {
    const NEEDS_TRACE: bool = false;
    #[inline]
    fn trace(&self, _: &mut Tracer<'_>) {}
}

// SAFETY: traces the contents, if any (I3). Its `Drop` only drops the contents (I4).
unsafe impl<T: Trace> Trace for Option<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Some(value) = self {
            value.trace(tracer);
        }
    }
}

// SAFETY: traces the contents (I3). Its `Drop` only drops the contents (I4).
unsafe impl<T: Trace + ?Sized> Trace for Box<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        (**self).trace(tracer);
    }
}

// SAFETY: traces every element (I3).
unsafe impl<T: Trace> Trace for [T] {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if T::NEEDS_TRACE {
            for value in self {
                value.trace(tracer);
            }
        }
    }
}

// SAFETY: traces every element (I3).
unsafe impl<T: Trace, const N: usize> Trace for [T; N] {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        self.as_slice().trace(tracer);
    }
}

// SAFETY: traces every element (I3). Its `Drop` only drops the elements (I4).
unsafe impl<T: Trace> Trace for Vec<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        self.as_slice().trace(tracer);
    }
}

// SAFETY: traces every element (I3). Its `Drop` only drops the elements (I4).
unsafe impl<T: Trace> Trace for VecDeque<T> {
    const NEEDS_TRACE: bool = T::NEEDS_TRACE;
    #[inline]
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if T::NEEDS_TRACE {
            for value in self {
                value.trace(tracer);
            }
        }
    }
}

macro_rules! trace_tuple {
    ($($name:ident)+) => {
        // SAFETY: traces every element (I3). Its `Drop` only drops the elements (I4).
        unsafe impl<$($name: Trace),+> Trace for ($($name,)+) {
            const NEEDS_TRACE: bool = false $(|| $name::NEEDS_TRACE)+;
            #[inline]
            #[allow(non_snake_case)]
            fn trace(&self, tracer: &mut Tracer<'_>) {
                let ($($name,)+) = self;
                $($name.trace(tracer);)+
            }
        }
    };
}

trace_tuple!(A);
trace_tuple!(A B);
trace_tuple!(A B C);
trace_tuple!(A B C D);

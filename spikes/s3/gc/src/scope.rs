//! GC scopes: zero-sized borrow tokens that make "a handle used after a
//! possible collection" a compile error.
//!
//! Every function that may collect takes a [`GcScope`]. The caller passes
//! `gc.reborrow()`, which borrows its own scope *mutably*. Handles bound to
//! `gc.nogc()` hold a *shared* borrow of that scope. The borrow checker
//! therefore rejects any use of such a handle after the call:
//!
//! ```compile_fail,E0502
//! # use s3_gc::{GcScope, GcToken, Handle};
//! fn may_collect(_: GcScope<'_>) {}
//! fn caller(handle: Handle<'_, u8>, mut gc: GcScope<'_>) {
//!     let handle = handle.bind(gc.nogc());
//!     may_collect(gc.reborrow());
//!     let _ = handle; // stale: compaction may have moved it
//! }
//! ```
//!
//! This is the discipline Nova uses. Unlike Pattern A's branding, it protects
//! *correctness*, not memory safety: handles are bounds-checked indices, so a
//! stale handle is a logic error, never undefined behaviour. Debug builds
//! detect stale use at run time too (`Handle`'s epoch check).

use core::marker::PhantomData;

/// The root of a scope chain. Create one on the stack at an entry point (the
/// driver, an event-loop task) and derive the first [`GcScope`] from it.
#[derive(Debug, Default)]
pub struct GcToken(());

impl GcToken {
    #[must_use]
    pub const fn new() -> Self {
        Self(())
    }
}

/// Permission to collect. Pass `gc.reborrow()` to a callee that may collect.
#[derive(Debug)]
pub struct GcScope<'a> {
    _token: PhantomData<&'a mut GcToken>,
}

/// Proof that no collection happens while it is borrowed. Bind handles to it.
#[derive(Clone, Copy, Debug)]
pub struct NoGcScope<'a> {
    _token: PhantomData<&'a GcToken>,
}

impl<'a> GcScope<'a> {
    /// The first scope of a chain.
    pub const fn new(_token: &'a mut GcToken) -> Self {
        Self {
            _token: PhantomData,
        }
    }

    /// A no-collection view. Handles bound to it die at the next `reborrow`.
    #[inline]
    #[must_use]
    pub const fn nogc(&self) -> NoGcScope<'_> {
        NoGcScope {
            _token: PhantomData,
        }
    }

    /// Lend this scope to a callee that may collect.
    #[inline]
    pub const fn reborrow(&mut self) -> GcScope<'_> {
        GcScope {
            _token: PhantomData,
        }
    }

    /// Give up the right to collect for the rest of `'a`.
    #[inline]
    #[must_use]
    pub const fn into_nogc(self) -> NoGcScope<'a> {
        NoGcScope {
            _token: PhantomData,
        }
    }
}

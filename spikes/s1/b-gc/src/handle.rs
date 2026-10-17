//! Handles: typed `u32` indices into one arena.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::num::NonZeroU32;

use crate::roots::{RootFrame, Scoped};
use crate::scope::NoGcScope;
use crate::trace::Allocated;

/// A typed index into the arena that holds `T`. It is valid until the next
/// collection, which compacts the arenas and so moves objects.
///
/// - **`'a`** ties the handle to a [`NoGcScope`] (`bind`), so the borrow checker
///   rejects its use after a possible collection. `'static` handles
///   (`unbind`) are stored in heap data and roots, where the collector
///   rewrites them.
/// - **Size.** The index is stored plus one in a `NonZeroU32`, so a handle is
///   4 bytes and `Option<Handle>` is too. Debug builds add a 4-byte collection
///   *epoch*, which turns stale use into a deterministic panic.
pub struct Handle<'a, T> {
    raw: NonZeroU32,
    #[cfg(debug_assertions)]
    epoch: u32,
    _marker: Brand<'a, T>,
}

/// Covariant in the scope lifetime, so that a handle bound to a longer scope
/// can be used where a shorter one is expected. `fn() -> T` keeps the handle a
/// plain index: `Send`, `Sync` and `'static` whatever `T` is.
type Brand<'a, T> = PhantomData<(&'a (), fn() -> T)>;

impl<T> Handle<'_, T> {
    #[inline]
    pub(crate) fn new(index: usize, epoch: u32) -> Self {
        let raw = u32::try_from(index + 1)
            .ok()
            .and_then(NonZeroU32::new)
            .expect("arena index exceeds u32");
        #[cfg(not(debug_assertions))]
        let _ = epoch;
        Self {
            raw,
            #[cfg(debug_assertions)]
            epoch,
            _marker: PhantomData,
        }
    }

    /// The index into the arena.
    #[inline]
    pub fn index(self) -> usize {
        (self.raw.get() - 1) as usize
    }

    /// The collection epoch this handle was issued in. Always 0 in release builds.
    #[inline]
    pub fn epoch(self) -> u32 {
        #[cfg(debug_assertions)]
        return self.epoch;
        #[cfg(not(debug_assertions))]
        0
    }

    /// Re-attach to a no-collection scope (Nova's `bind`).
    #[inline]
    #[must_use]
    #[expect(
        clippy::elidable_lifetime_names,
        reason = "the named lifetime is the API: the result lives exactly as long as the scope"
    )]
    pub fn bind<'b>(self, _gc: NoGcScope<'b>) -> Handle<'b, T> {
        self.with_lifetime()
    }

    /// Detach from any scope, so the handle can be stored in heap data or roots,
    /// or passed to a callee that may collect (which must re-bind it).
    #[inline]
    #[must_use]
    pub fn unbind(self) -> Handle<'static, T> {
        self.with_lifetime()
    }

    #[inline]
    fn with_lifetime<'b>(self) -> Handle<'b, T> {
        Handle {
            raw: self.raw,
            #[cfg(debug_assertions)]
            epoch: self.epoch,
            _marker: PhantomData,
        }
    }

    /// Root this handle in `frame`, so it survives collections until the frame
    /// is dropped. Read it back, relocated, with [`Scoped::get`].
    #[inline]
    #[expect(
        clippy::elidable_lifetime_names,
        reason = "the named lifetime is the API: the root lives exactly as long as the frame borrow"
    )]
    pub fn scope<'f>(self, frame: &'f RootFrame) -> Scoped<'f, T>
    where
        T: Allocated,
    {
        frame.push(self)
    }

    /// A weak reference: it is cleared, not relocated, if its target dies.
    #[inline]
    #[must_use]
    pub fn downgrade(self) -> WeakHandle<T> {
        WeakHandle {
            target: self.unbind(),
        }
    }
}

impl<T> Clone for Handle<'_, T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<'_, T> {}

impl<T> PartialEq for Handle<'_, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Handle<'_, T> {}

impl<T> Hash for Handle<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> fmt::Debug for Handle<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle({})", self.index())
    }
}

/// A weak reference into an arena. When the collector finds the target dead, it
/// sets the `Option<WeakHandle>` holding it to `None`; while the target lives,
/// the collector relocates the handle.
pub struct WeakHandle<T> {
    target: Handle<'static, T>,
}

impl<T> WeakHandle<T> {
    /// The target. A `WeakHandle` the collector has kept always refers to a
    /// live object.
    #[inline]
    pub fn upgrade(self) -> Handle<'static, T> {
        self.target
    }

    pub(crate) fn target_mut(&mut self) -> &mut Handle<'static, T> {
        &mut self.target
    }
}

impl<T> Clone for WeakHandle<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for WeakHandle<T> {}

impl<T> fmt::Debug for WeakHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WeakHandle({})", self.target.index())
    }
}

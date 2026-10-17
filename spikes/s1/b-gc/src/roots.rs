//! The explicit root context: a stack of root slots, plus RAII frames.

use core::cell::RefCell;
use core::fmt;
use core::marker::PhantomData;
use std::rc::Rc;

use crate::handle::Handle;
use crate::trace::{Allocated, Forwarding, Marker};

/// One rooted handle, type-erased to (arena, index, epoch).
#[derive(Clone, Copy, Debug)]
pub(crate) struct RootSlot {
    arena: usize,
    index: usize,
    epoch: u32,
}

/// The stack of rooted handles. It is shared, via `Rc`, between the heap
/// (which marks and relocates the slots) and the frames (which pop them when
/// dropped).
#[derive(Clone, Debug, Default)]
pub struct RootStack(Rc<RefCell<Vec<RootSlot>>>);

impl RootStack {
    /// Open a frame. Roots pushed through it are popped when it is dropped.
    #[must_use]
    pub fn frame(&self) -> RootFrame {
        RootFrame {
            height: self.0.borrow().len(),
            stack: self.clone(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub(crate) fn mark(&self, marker: &mut Marker) {
        for slot in self.0.borrow().iter() {
            marker.mark_raw(slot.arena, slot.index, slot.epoch);
        }
    }

    pub(crate) fn relocate(&self, forwarding: &Forwarding) {
        for slot in self.0.borrow_mut().iter_mut() {
            slot.index = forwarding.rank(slot.arena, slot.index);
            slot.epoch = forwarding.epoch();
        }
    }
}

/// A scope for roots. Dropping it pops everything pushed through it. Frames
/// must be dropped in LIFO order (scoping makes that natural).
pub struct RootFrame {
    stack: RootStack,
    height: usize,
}

impl RootFrame {
    pub(crate) fn push<'f, T: Allocated>(&'f self, handle: Handle<'_, T>) -> Scoped<'f, T> {
        let mut slots = self.stack.0.borrow_mut();
        slots.push(RootSlot {
            arena: T::ARENA,
            index: handle.index(),
            epoch: handle.epoch(),
        });
        Scoped {
            frame: self,
            slot: slots.len() - 1,
            _marker: PhantomData,
        }
    }
}

impl Drop for RootFrame {
    fn drop(&mut self) {
        self.stack.0.borrow_mut().truncate(self.height);
    }
}

impl fmt::Debug for RootFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootFrame")
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// A rooted handle. It is kept alive, and relocated, by its frame.
pub struct Scoped<'f, T> {
    frame: &'f RootFrame,
    slot: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Scoped<'_, T> {
    /// The current handle. Bind it (`.bind(gc.nogc())`) before use.
    #[inline]
    pub fn get(&self) -> Handle<'static, T> {
        let slot = self.frame.stack.0.borrow()[self.slot];
        Handle::new(slot.index, slot.epoch)
    }
}

impl<T> fmt::Debug for Scoped<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scoped")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

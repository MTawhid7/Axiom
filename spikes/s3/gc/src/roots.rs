//! Root stacks: where native code keeps values alive across a possible
//! collection (ADR-0008 SH6, part 2).
//!
//! Three interchangeable implementations of [`RootStack`], for S3-Q1:
//!
//! | Stack | Storage | A rooted value is found by | `unsafe` |
//! |---|---|---|---|
//! | [`RefCellStack`] | `RefCell<Vec<Value>>`: S1's borrow-tracked design, with 8-byte slots | stack and index; every access borrows the `RefCell` | none |
//! | [`CellStack`] | chunks of 1,024 `Cell<Value>` slots, allocated on demand and never moved | a reference to its slot | none |
//! | [`RawStack`](crate::RawStack) | one lazily committed, fixed-capacity array of `Cell<Value>` slots (C1) | a reference to its slot | `src/raw/` only |
//!
//! Frames and scoped handles are the same for all three. A frame borrows the
//! stack, not the heap, so code holding a frame can still borrow the heap
//! mutably. That is why the stack lives outside the heap (`HeapCore` holds a
//! `&'r` reference to it), where S1 shared it through an `Rc`.
//!
//! **Misuse is a logic error, never undefined behaviour.** Frames are meant
//! to be dropped in LIFO order. If one is not, a later push can overwrite a
//! slot that a live [`Scoped`] still names: it then reads the wrong value,
//! which debug-build epochs usually catch, exactly as with a stale handle.

use core::cell::{Cell, OnceCell, RefCell};
use core::fmt;
use core::marker::PhantomData;

use crate::count;
use crate::handle::Handle;
use crate::trace::Allocated;
use crate::value::Value;

/// Storage for rooted values. Not `Sync`: a root stack belongs to one agent's
/// thread, like its heap.
pub trait RootStack: fmt::Debug {
    /// What a [`Scoped`] keeps, to find its value again.
    type Slot<'s>: Copy
    where
        Self: 's;

    /// The number of rooted values.
    fn height(&self) -> usize;

    /// Root a value. Panics if the stack is full.
    fn push(&self, value: Value) -> Self::Slot<'_>;

    /// The value in a slot, as relocated by any collection since.
    fn read<'s>(slot: Self::Slot<'s>) -> Value
    where
        Self: 's;

    /// Pop every value above `height`.
    fn truncate(&self, height: usize);

    /// Visit every rooted value, bottom to top (marking).
    fn for_each(&self, f: impl FnMut(Value));

    /// Replace every rooted value with `f(value)` (relocation).
    fn update(&self, f: impl FnMut(Value) -> Value);
}

/// A scope for roots. Dropping it pops everything pushed through it.
pub struct RootFrame<'s, S: RootStack> {
    stack: &'s S,
    height: usize,
}

impl<'s, S: RootStack> RootFrame<'s, S> {
    #[inline]
    #[must_use]
    pub fn new(stack: &'s S) -> Self {
        count::frame();
        Self {
            height: stack.height(),
            stack,
        }
    }

    #[inline]
    fn push<'f>(&'f self, value: Value) -> S::Slot<'f> {
        count::push();
        let stack: &'f S = self.stack;
        stack.push(value)
    }
}

impl<S: RootStack> Drop for RootFrame<'_, S> {
    #[inline]
    fn drop(&mut self) {
        self.stack.truncate(self.height);
    }
}

impl<S: RootStack> fmt::Debug for RootFrame<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootFrame")
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// A rooted handle. It is kept alive, and relocated, by its frame.
pub struct Scoped<'f, T, S: RootStack + 'f> {
    slot: S::Slot<'f>,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Allocated, S: RootStack> Scoped<'_, T, S> {
    /// The current handle. Bind it (`.bind(gc.nogc())`) before use.
    #[inline]
    pub fn get(&self) -> Handle<'static, T> {
        count::read();
        Handle::from_rooted(S::read(self.slot))
    }
}

impl<T, S: RootStack> fmt::Debug for Scoped<'_, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scoped").finish_non_exhaustive()
    }
}

/// A rooted [`Value`] of any kind.
pub struct ScopedValue<'f, S: RootStack + 'f> {
    slot: S::Slot<'f>,
}

impl<S: RootStack> ScopedValue<'_, S> {
    /// The current value, relocated by any collection since it was rooted.
    #[inline]
    pub fn get(&self) -> Value {
        count::read();
        S::read(self.slot)
    }
}

impl<S: RootStack> fmt::Debug for ScopedValue<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedValue").finish_non_exhaustive()
    }
}

impl<T: Allocated> Handle<'_, T> {
    /// Root this handle in `frame`, so it survives collections until the frame
    /// is dropped. Read it back, relocated, with [`Scoped::get`].
    #[inline]
    pub fn scope<'f, S: RootStack>(self, frame: &'f RootFrame<'_, S>) -> Scoped<'f, T, S> {
        Scoped {
            slot: frame.push(self.to_value()),
            _marker: PhantomData,
        }
    }
}

impl Value {
    /// Root this value in `frame` (see [`Handle::scope`]).
    #[inline]
    pub fn scope<'f, S: RootStack>(self, frame: &'f RootFrame<'_, S>) -> ScopedValue<'f, S> {
        ScopedValue {
            slot: frame.push(self),
        }
    }
}

// ----- RefCellStack -----

/// S1's design: a growable vector behind a `RefCell`, so every access checks
/// and updates a borrow flag. Slots are 8 bytes here (S1's were 24), and
/// frames borrow the stack instead of sharing it through an `Rc`.
#[derive(Debug, Default)]
pub struct RefCellStack(RefCell<Vec<Value>>);

impl RootStack for RefCellStack {
    type Slot<'s> = (&'s Self, usize);

    #[inline]
    fn height(&self) -> usize {
        self.0.borrow().len()
    }

    #[inline]
    fn push(&self, value: Value) -> (&Self, usize) {
        let mut slots = self.0.borrow_mut();
        slots.push(value);
        (self, slots.len() - 1)
    }

    #[inline]
    fn read<'s>((stack, index): (&'s Self, usize)) -> Value
    where
        Self: 's,
    {
        stack.0.borrow()[index]
    }

    #[inline]
    fn truncate(&self, height: usize) {
        self.0.borrow_mut().truncate(height);
    }

    fn for_each(&self, mut f: impl FnMut(Value)) {
        for &value in self.0.borrow().iter() {
            f(value);
        }
    }

    fn update(&self, mut f: impl FnMut(Value) -> Value) {
        for value in self.0.borrow_mut().iter_mut() {
            *value = f(*value);
        }
    }
}

// ----- CellStack -----

/// Slots per chunk: 8 KiB.
const CHUNK: usize = 1024;
/// Chunks: at most 1,048,576 roots.
const CHUNKS: usize = 1024;

type Chunk = [Cell<Value>; CHUNK];

/// A safe stack without borrow tracking. Slots are `Cell`s, which allow
/// mutation through shared references at no run-time cost. They live in
/// fixed-size chunks that are allocated on first use and never move, so a
/// [`Scoped`] can keep a reference to its slot, and reading it back is one
/// load.
pub struct CellStack {
    len: Cell<usize>,
    chunks: Box<[OnceCell<Box<Chunk>>]>,
}

impl Default for CellStack {
    fn default() -> Self {
        Self {
            len: Cell::new(0),
            chunks: (0..CHUNKS).map(|_| OnceCell::new()).collect(),
        }
    }
}

impl CellStack {
    #[inline]
    fn chunk(&self, chunk: usize) -> &Chunk {
        self.chunks
            .get(chunk)
            .expect("root stack overflow")
            .get_or_init(|| Box::new([const { Cell::new(Value::UNDEFINED) }; CHUNK]))
    }

    fn occupied(&self) -> impl Iterator<Item = &Cell<Value>> {
        let len = self.len.get();
        self.chunks
            .iter()
            .filter_map(OnceCell::get)
            .flat_map(|chunk| chunk.iter())
            .take(len)
    }
}

impl RootStack for CellStack {
    type Slot<'s> = &'s Cell<Value>;

    #[inline]
    fn height(&self) -> usize {
        self.len.get()
    }

    #[inline]
    fn push(&self, value: Value) -> &Cell<Value> {
        let index = self.len.get();
        let slot = &self.chunk(index / CHUNK)[index % CHUNK];
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
        for slot in self.occupied() {
            f(slot.get());
        }
    }

    fn update(&self, mut f: impl FnMut(Value) -> Value) {
        for slot in self.occupied() {
            slot.set(f(slot.get()));
        }
    }
}

impl fmt::Debug for CellStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CellStack")
            .field("len", &self.len.get())
            .finish_non_exhaustive()
    }
}

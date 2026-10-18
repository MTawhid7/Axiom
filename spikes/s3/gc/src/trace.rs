//! Tracing and relocation: the traits heap data implements, the marker, and
//! rank-based forwarding. As in S1, plus [`Value`] support.

use crate::handle::{Handle, WeakHandle};
use crate::value::Value;

/// A type stored in an arena. The `heap!` macro implements this and assigns
/// each type its arena number, below [`MAX_ARENAS`](crate::MAX_ARENAS).
pub trait Allocated: Sized + 'static {
    const ARENA: usize;
}

/// Heap data that holds handles or values.
///
/// This is a *safe* trait. A wrong implementation is a logic error, never
/// undefined behaviour: indices are bounds-checked. Debug builds catch the
/// common mistakes:
/// - forgetting to `mark` a handle trips an assertion when it is relocated
///   (the target is dead);
/// - forgetting to `relocate` one trips the epoch check on its next use.
pub trait Trace: Allocated {
    /// Mark every *strong* handle and value (not weak ones).
    fn mark(&self, marker: &mut Marker);
    /// Rewrite every handle and value, strong and weak, after compaction.
    fn relocate(&mut self, forwarding: &Forwarding);
}

/// The heap's external roots (globals, registers), traced and relocated
/// alongside the root stack.
pub trait Roots {
    fn mark(&self, marker: &mut Marker);
    fn relocate(&mut self, forwarding: &Forwarding);
}

/// A growable bitset. One bit per arena slot.
#[derive(Clone, Debug, Default)]
pub(crate) struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    fn with_len(len: usize) -> Self {
        Self {
            words: vec![0; len.div_ceil(64)],
        }
    }

    /// Set bit `i`; returns whether it was newly set.
    #[inline]
    fn insert(&mut self, i: usize) -> bool {
        let (word, bit) = (i / 64, 1_u64 << (i % 64));
        let newly = self.words[word] & bit == 0;
        self.words[word] |= bit;
        newly
    }

    #[inline]
    pub(crate) fn contains(&self, i: usize) -> bool {
        self.words[i / 64] & (1_u64 << (i % 64)) != 0
    }
}

/// Marks reachable objects, using a worklist instead of recursion, so that deep
/// graphs cannot overflow the stack. Worklist entries are 8 bytes (SH11).
#[derive(Debug)]
pub struct Marker {
    marks: Vec<BitSet>,
    work: Vec<(u32, u32)>,
    epoch: u16,
}

impl Marker {
    pub(crate) fn new(arena_lens: &[usize], epoch: u16) -> Self {
        Self {
            marks: arena_lens
                .iter()
                .map(|&len| BitSet::with_len(len))
                .collect(),
            work: Vec::new(),
            epoch,
        }
    }

    /// Mark a strong handle.
    #[inline]
    pub fn mark<T: Allocated>(&mut self, handle: Handle<'_, T>) {
        self.mark_raw(T::ARENA, handle.index(), handle.epoch());
    }

    /// Mark the handle, if any.
    #[inline]
    pub fn mark_opt<T: Allocated>(&mut self, handle: Option<Handle<'_, T>>) {
        if let Some(handle) = handle {
            self.mark(handle);
        }
    }

    /// Mark the object a value refers to, if it refers to one.
    #[inline]
    pub fn mark_value(&mut self, value: Value) {
        if let Some((arena, raw, epoch)) = value.heap_ref() {
            self.mark_raw(arena, raw as usize - 1, epoch);
        }
    }

    #[inline]
    pub(crate) fn mark_raw(&mut self, arena: usize, index: usize, epoch: u16) {
        debug_assert_eq!(
            epoch, self.epoch,
            "stale handle in heap data or roots: it was not relocated by the last collection"
        );
        let _ = epoch;
        if self.marks[arena].insert(index) {
            self.work.push((
                u32::try_from(arena).expect("few arenas"),
                u32::try_from(index).expect("arena indices fit u32"),
            ));
        }
    }

    pub(crate) fn pop(&mut self) -> Option<(usize, usize)> {
        self.work
            .pop()
            .map(|(arena, index)| (arena as usize, index as usize))
    }

    pub(crate) fn into_forwarding(self, epoch: u16) -> Forwarding {
        Forwarding {
            ranks: self.marks.into_iter().map(Rank::new).collect(),
            epoch,
        }
    }
}

/// The surviving slots of one arena, plus per-word prefix counts. An object's
/// new index is its *rank*: the number of survivors before it. That costs 4
/// bytes per 64 slots, instead of a forwarding table per object.
#[derive(Debug)]
struct Rank {
    live: BitSet,
    prefix: Vec<u32>,
}

impl Rank {
    fn new(live: BitSet) -> Self {
        let mut prefix = Vec::with_capacity(live.words.len());
        let mut running = 0_u32;
        for word in &live.words {
            prefix.push(running);
            running += word.count_ones();
        }
        Self { live, prefix }
    }

    #[inline]
    fn rank(&self, i: usize) -> usize {
        debug_assert!(
            self.live.contains(i),
            "relocating a handle to a dead object: some Trace::mark missed it"
        );
        let word = i / 64;
        let below = self.live.words[word] & ((1_u64 << (i % 64)) - 1);
        self.prefix[word] as usize + below.count_ones() as usize
    }
}

/// Rewrites handles and values after compaction.
#[derive(Debug)]
pub struct Forwarding {
    ranks: Vec<Rank>,
    epoch: u16,
}

impl Forwarding {
    /// Relocate a strong handle.
    #[inline]
    pub fn relocate<T: Allocated>(&self, handle: &mut Handle<'static, T>) {
        *handle = Handle::new(self.rank(T::ARENA, handle.index()), self.epoch);
    }

    /// Relocate the handle, if any.
    #[inline]
    pub fn relocate_opt<T: Allocated>(&self, handle: &mut Option<Handle<'static, T>>) {
        if let Some(handle) = handle {
            self.relocate(handle);
        }
    }

    /// Relocate a weak handle, or clear it if its target died.
    #[inline]
    pub fn relocate_weak<T: Allocated>(&self, weak: &mut Option<WeakHandle<T>>) {
        if let Some(handle) = weak {
            if self.ranks[T::ARENA].live.contains(handle.upgrade().index()) {
                self.relocate(handle.target_mut());
            } else {
                *weak = None;
            }
        }
    }

    /// Relocate a value in place.
    #[inline]
    pub fn relocate_value(&self, value: &mut Value) {
        *value = self.relocated(*value);
    }

    /// The value after compaction: references are rewritten, everything else
    /// is unchanged.
    #[inline]
    #[must_use]
    pub fn relocated(&self, value: Value) -> Value {
        match value.heap_ref() {
            Some((arena, raw, _)) => {
                let index = self.rank(arena, raw as usize - 1);
                Value::from_heap(
                    arena,
                    u32::try_from(index + 1).expect("arena indices fit u32"),
                    self.epoch,
                )
            }
            None => value,
        }
    }

    pub(crate) fn rank(&self, arena: usize, index: usize) -> usize {
        self.ranks[arena].rank(index)
    }

    pub(crate) fn is_live(&self, arena: usize, index: usize) -> bool {
        self.ranks[arena].live.contains(index)
    }

    pub(crate) const fn epoch(&self) -> u16 {
        self.epoch
    }
}

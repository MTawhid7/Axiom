//! The interface the suites drive. Each pattern implements it once, as a thin
//! layer over its own DOM crate: [`PatternA`](crate::PatternA) and
//! [`PatternB`](crate::PatternB).

use core::time::Duration;

use s1_scenarios::{LogEntry, NodeId, Op, Outcome, Script, Tag};

use crate::harness::Pattern;

/// How a benchmark heap is configured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// Collect automatically, with the pattern's default pacing. When false,
    /// nothing is collected unless a suite asks, so mutator costs are measured
    /// without GC.
    pub auto_collect: bool,
    /// Unreachable nodes allocated right after each node that a plan creates.
    /// They are interleaved with the live nodes in allocation order, which is
    /// the realistic case for both a sweep and a compaction.
    pub garbage_per_node: u16,
}

impl Config {
    /// Mutator only: no automatic collection, no extra garbage.
    pub const MUTATOR: Self = Self {
        auto_collect: false,
        garbage_per_node: 0,
    };
}

/// A DOM implementation under test.
///
/// Ops are applied the way the pattern's own script driver applies them
/// (`s1_a_dom::driver`, `s1_b_dom::driver`), including the per-op collection
/// check. Only `MakeWeak` is unsupported, since no plan uses it.
pub trait Dom: Sized {
    const PATTERN: Pattern;

    fn new(node_slots: u32, config: Config) -> Self;

    /// Apply `ops` in order. `Collect` and `Checkpoint` both run a full
    /// collection; neither records anything.
    fn apply(&mut self, ops: &[Op]);

    /// A pre-order walk from the document over `first_child`,
    /// `next_sibling` and `parent`, reading each node's id, kind and
    /// attribute count. It allocates nothing.
    fn traverse(&self) -> Fold;

    /// The canonical digest of the tree (the same one checkpoints record).
    fn tree_digest(&self) -> u64;

    /// A full collection.
    fn collect(&mut self);

    /// Listener invocations logged so far.
    fn log(&self) -> &[LogEntry];

    fn clear_log(&mut self);

    fn heap(&self) -> HeapStats;

    /// Run a whole script through the pattern's own driver, with its default
    /// pacing. This is the code path the CP4 and CP6 oracle tests verified.
    fn run_script(script: &Script) -> ScenarioRun;
}

/// What a traversal computes: the number of nodes visited, and an
/// order-sensitive hash of each node's id, kind and attribute count.
/// Equal folds mean both patterns walked the same tree in the same order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fold {
    pub nodes: u64,
    pub hash: u64,
}

impl Fold {
    #[inline]
    pub fn visit(&mut self, id: Option<NodeId>, kind: u64, attributes: usize) {
        let id = id.map_or(0, |id| u64::from(id.0) + 1);
        self.nodes += 1;
        self.hash = (self.hash.rotate_left(5) ^ id ^ (kind << 32) ^ ((attributes as u64) << 48))
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// [`Fold`] kind codes, shared so both patterns hash identically.
pub mod kind {
    use s1_scenarios::Tag;

    pub const DOCUMENT: u64 = 0;
    pub const TEXT: u64 = 1;

    #[inline]
    pub const fn element(tag: Tag) -> u64 {
        2 + tag as u64
    }
}

/// The heap accounting both patterns report, in a common shape.
///
/// Byte counts are each pattern's own logical accounting:
/// - Pattern A counts boxes: a 24-byte header plus the value. Allocator
///   rounding is not included.
/// - Pattern B counts `size_of` the objects in their arenas. `Vec` capacity
///   slack is not included.
///
/// Resident memory is measured outside the process (`s1-probe run` under
/// `/usr/bin/time -l`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeapStats {
    pub objects: usize,
    pub bytes: usize,
    pub peak_objects: usize,
    pub peak_bytes: usize,
    pub total_allocated_objects: u64,
    pub total_allocated_bytes: u64,
    pub collections: u64,
    pub max_pause: Duration,
    pub total_pause: Duration,
}

/// The result of [`Dom::run_script`].
#[derive(Clone, Debug)]
pub struct ScenarioRun {
    pub outcome: Outcome,
    pub peak_live_nodes: usize,
    pub heap: HeapStats,
    /// Pattern A only: how often a dispatch yielded to the collector.
    pub dispatch_yields: Option<u64>,
}

/// The tag of the unreachable nodes that [`Config::garbage_per_node`] adds.
pub const GARBAGE_TAG: Tag = Tag::Div;

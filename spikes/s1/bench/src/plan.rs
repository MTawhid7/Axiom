//! Pattern-independent benchmark inputs.
//!
//! Every plan starts from a tree made by the CP2 `tree` generator (realistic
//! depth, fan-out, text and attribute mix). Every op a plan contains is
//! applied to the reference interpreter before it is recorded, so plans are
//! valid by construction, and come with the results both patterns must
//! reproduce.
//!
//! **Allocation order.** The generator grows trees breadth-first. A parser
//! creates nodes in document order (pre-order), which is the common case for
//! a page and the friendliest one for locality in both patterns. Plans use
//! [`Order::Document`] unless they say otherwise. The `traverse` suite also
//! measures [`Order::Breadth`] and a [`shuffle`]d tree, to show how much the
//! order matters.

use s1_scenarios::generate::{self, TreeParams};
use s1_scenarios::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, LogEntry, NodeId, Op, Reference, Rng,
    Size,
};

/// A benchmark input: ops that establish a state, then the ops a suite times.
#[derive(Clone, Debug)]
pub struct Plan {
    /// Ops that build the initial state.
    pub setup: Vec<Op>,
    /// Ops to time on top of `setup`. Empty for tree-only plans.
    pub work: Vec<Op>,
    pub node_slots: u32,
    /// Nodes in the tree after `setup`, counting the document. Every created
    /// node is attached, so this is also the live-node count.
    pub nodes: u64,
    /// Tree digest after `setup`.
    pub setup_digest: u64,
    /// Tree digest after `setup` and then `work`.
    pub final_digest: u64,
    /// The listener log that `work` produces, from a fresh `setup` state.
    pub log: Vec<LogEntry>,
}

/// The order in which a tree's nodes are created and attached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Pre-order, as a parser builds a document.
    Document,
    /// Level by level, as the CP2 generator grows it.
    Breadth,
}

/// Records ops, applying each one to a [`Reference`] model first. The model
/// rejects invalid ops and answers the queries that keep plans valid.
#[derive(Debug)]
struct Recorder {
    ops: Vec<Op>,
    model: Reference,
    rng: Rng,
    node_slots: u32,
    next_listener: u32,
    /// Ids of the tree's elements and text nodes, in id order.
    elements: Vec<NodeId>,
    texts: Vec<NodeId>,
}

impl Recorder {
    /// Start from the CP2 tree generator's tree of `elements` elements.
    fn tree(elements: u32, order: Order, seed: u64) -> Self {
        let params = TreeParams {
            elements,
            forget_ids: false,
            ..TreeParams::preset(Size::Large)
        };
        let script = generate::tree(&params, seed).script;
        let ops = match order {
            Order::Document => document_order(&script.ops, script.node_slots),
            Order::Breadth => script
                .ops
                .into_iter()
                .filter(|op| *op != Op::Checkpoint)
                .collect(),
        };
        let mut rec = Self {
            ops: Vec::with_capacity(ops.len()),
            model: Reference::new(),
            // A different stream from the generator's, for what plans add.
            rng: Rng::seed_from_u64(seed ^ 0x5EED_0FB3_C0DE),
            node_slots: script.node_slots,
            next_listener: 0,
            elements: Vec::new(),
            texts: Vec::new(),
        };
        for op in ops {
            match op {
                Op::CreateElement { id, .. } => rec.elements.push(id),
                Op::CreateText { id, .. } => rec.texts.push(id),
                _ => {}
            }
            rec.record(op);
        }
        rec.elements.sort_unstable_by_key(|id| id.0);
        rec.texts.sort_unstable_by_key(|id| id.0);
        rec
    }

    fn record(&mut self, op: Op) {
        if let Err(error) = self.model.apply(&op) {
            panic!("plan bug: {op:?} is invalid: {error}");
        }
        self.ops.push(op);
    }

    fn add_listener(&mut self, node: NodeId, spec: ListenerSpec) {
        let listener = ListenerId(self.next_listener);
        self.next_listener += 1;
        self.record(Op::AddListener {
            node,
            listener,
            spec,
        });
    }

    /// The model's current tree digest. Checkpoints only record, so this does
    /// not change the model's state; the op is not part of the plan.
    fn digest(&mut self) -> u64 {
        self.model
            .apply(&Op::Checkpoint)
            .expect("a checkpoint is always valid");
        self.model
            .outcome()
            .checkpoints
            .last()
            .expect("a checkpoint was just recorded")
            .tree_digest
    }

    /// Every created node: elements, then text.
    fn pool(&self) -> Vec<NodeId> {
        self.elements.iter().chain(&self.texts).copied().collect()
    }

    /// A random node from `pool` that can have children, else the document.
    fn random_container(&mut self, pool: &[NodeId]) -> NodeId {
        (0..16)
            .map(|_| *self.rng.pick(pool))
            .find(|&id| self.model.can_have_children(id))
            .unwrap_or(NodeId::DOCUMENT)
    }

    /// The first `setup_len` recorded ops are setup; the rest are work.
    fn finish(mut self, setup_len: usize, setup_digest: u64, final_digest: u64) -> Plan {
        let work = self.ops.split_off(setup_len);
        let nodes = (1 + self.elements.len() + self.texts.len()) as u64;
        let (outcome, _) = self.model.into_parts();
        Plan {
            setup: self.ops,
            work,
            node_slots: self.node_slots,
            nodes,
            setup_digest,
            final_digest,
            log: outcome.log,
        }
    }
}

/// The tree generator's ops, reordered so that every node is created, given
/// its attributes, and attached in document order. Ids, attributes and the
/// resulting tree are unchanged.
fn document_order(ops: &[Op], node_slots: u32) -> Vec<Op> {
    let slots = node_slots as usize;
    let mut created: Vec<Option<Op>> = vec![None; slots];
    let mut attributes: Vec<Vec<Op>> = vec![Vec::new(); slots];
    let mut children: Vec<Vec<NodeId>> = vec![Vec::new(); slots];
    for op in ops {
        match op {
            Op::CreateElement { id, .. } | Op::CreateText { id, .. } => {
                created[id.index()] = Some(op.clone());
            }
            Op::SetAttribute { node, .. } => attributes[node.index()].push(op.clone()),
            // The generator attaches every node exactly once, so append order
            // is final child order.
            Op::AppendChild { parent, child } => children[parent.index()].push(*child),
            Op::Checkpoint => {}
            other => unreachable!("the tree generator does not emit {other:?}"),
        }
    }
    let mut out = Vec::with_capacity(ops.len());
    let mut stack: Vec<(NodeId, NodeId)> = children[NodeId::DOCUMENT.index()]
        .iter()
        .rev()
        .map(|&child| (NodeId::DOCUMENT, child))
        .collect();
    while let Some((parent, node)) = stack.pop() {
        out.push(
            created[node.index()]
                .take()
                .expect("each node is created once"),
        );
        out.append(&mut attributes[node.index()]);
        out.push(Op::AppendChild {
            parent,
            child: node,
        });
        stack.extend(
            children[node.index()]
                .iter()
                .rev()
                .map(|&child| (node, child)),
        );
    }
    out
}

/// A generated tree of `elements` elements, plus text nodes. There are no
/// work ops: `build` times the setup itself, and `traverse` and `collect`
/// time operations that are not ops.
#[must_use]
pub fn tree(elements: u32, order: Order, seed: u64) -> Plan {
    let mut rec = Recorder::tree(elements, order, seed);
    let setup_len = rec.ops.len();
    let digest = rec.digest();
    rec.finish(setup_len, digest, digest)
}

/// A tree, then `ops` random mutations: 40% moves (append elsewhere), 20%
/// insert-befores, 15% removes, 25% attribute writes.
///
/// Removed nodes stay referenced by their ids, so later moves can reattach
/// them, and moves may target detached subtrees, as scripts building document
/// fragments do. Many nodes end up detached.
#[must_use]
pub fn mutate(elements: u32, ops: u32, seed: u64) -> Plan {
    let mut rec = Recorder::tree(elements, Order::Document, seed);
    let setup_len = rec.ops.len();
    let setup_digest = rec.digest();
    let pool = rec.pool();

    let mut recorded = 0;
    while recorded < ops {
        let op = match rec.rng.below(100) {
            0..40 => {
                let child = *rec.rng.pick(&pool);
                let parent = rec.random_container(&pool);
                if rec.model.is_inclusive_ancestor(child, parent) {
                    continue;
                }
                Op::AppendChild { parent, child }
            }
            40..60 => {
                let parent = rec.random_container(&pool);
                let children = rec.model.child_ids(parent);
                if children.is_empty() {
                    continue;
                }
                let reference = *rec.rng.pick(&children);
                let child = *rec.rng.pick(&pool);
                if rec.model.is_inclusive_ancestor(child, parent) {
                    continue;
                }
                Op::InsertBefore {
                    parent,
                    child,
                    reference,
                }
            }
            60..75 => Op::Remove {
                node: *rec.rng.pick(&pool),
            },
            _ => {
                let node = *rec.rng.pick(&rec.elements);
                let name = *rec.rng.pick(&AttrName::ALL);
                let value = format!("m{}", rec.rng.below(1_000)).into();
                Op::SetAttribute { node, name, value }
            }
        };
        rec.record(op);
        recorded += 1;
    }

    let final_digest = rec.digest();
    rec.finish(setup_len, setup_digest, final_digest)
}

/// A tree, then `moves` random moves that keep every node attached: each
/// appends a random node, or inserts it before a random child, under a random
/// element outside its own subtree. Afterwards tree order no longer follows
/// allocation order, as in a page whose scripts rearranged it.
#[must_use]
pub fn shuffle(elements: u32, moves: u32, seed: u64) -> Plan {
    let mut rec = Recorder::tree(elements, Order::Document, seed);
    let setup_len = rec.ops.len();
    let setup_digest = rec.digest();
    let pool = rec.pool();

    let mut recorded = 0;
    while recorded < moves {
        let child = *rec.rng.pick(&pool);
        // Every node stays attached, so every container is attached too.
        let parent = rec.random_container(&pool);
        if rec.model.is_inclusive_ancestor(child, parent) {
            continue;
        }
        let children = rec.model.child_ids(parent);
        let op = if !children.is_empty() && rec.rng.ratio(1, 2) {
            Op::InsertBefore {
                parent,
                child,
                reference: *rec.rng.pick(&children),
            }
        } else {
            Op::AppendChild { parent, child }
        };
        rec.record(op);
        recorded += 1;
    }

    let final_digest = rec.digest();
    rec.finish(setup_len, setup_digest, final_digest)
}

/// A tree with `listeners` listeners in setup, then `dispatches` dispatches
/// at random elements.
///
/// Listener closures never allocate GC objects (80% log only, 10% set an
/// attribute on their node, 10% stop propagation), so the suite measures the
/// dispatch machinery, and repeated runs of `work` see the same state. A
/// quarter of the closures capture a second, random node, as in CP2's mix;
/// 0.2% of listeners sit on the document.
#[must_use]
pub fn dispatch(elements: u32, listeners: u32, dispatches: u32, seed: u64) -> Plan {
    let mut rec = Recorder::tree(elements, Order::Document, seed);
    let pool = rec.pool();
    for _ in 0..listeners {
        let on_document = rec.rng.ratio(2, 1000);
        let node = if on_document {
            NodeId::DOCUMENT
        } else {
            *rec.rng.pick(&rec.elements)
        };
        let behavior = match rec.rng.below(100) {
            // The document has no attributes, so it only gets the other two.
            0..10 if !on_document => Behavior::SetAttribute {
                name: *rec.rng.pick(&AttrName::ALL),
                value: format!("d{}", rec.rng.below(100)).into(),
            },
            10..20 => Behavior::StopPropagation,
            _ => Behavior::LogOnly,
        };
        let spec = ListenerSpec {
            event: *rec.rng.pick(&EventType::ALL),
            capture: rec.rng.ratio(1, 3),
            behavior,
            extra_capture: rec.rng.ratio(1, 4).then(|| *rec.rng.pick(&pool)),
        };
        rec.add_listener(node, spec);
    }
    let setup_len = rec.ops.len();
    let setup_digest = rec.digest();

    for _ in 0..dispatches {
        let target = *rec.rng.pick(&rec.elements);
        let event = *rec.rng.pick(&EventType::ALL);
        let bubbles = rec.rng.ratio(4, 5);
        rec.record(Op::Dispatch {
            target,
            event,
            bubbles,
        });
    }
    let final_digest = rec.digest();
    rec.finish(setup_len, setup_digest, final_digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_order_builds_the_same_tree() {
        let document = tree(300, Order::Document, 7);
        let breadth = tree(300, Order::Breadth, 7);
        assert_eq!(document.setup_digest, breadth.setup_digest);
        assert_eq!(document.nodes, breadth.nodes);
        assert_ne!(document.setup, breadth.setup, "only the order differs");
        // In document order, every node is attached right after it is created.
        let creates = |plan: &Plan| {
            plan.setup
                .iter()
                .filter(|op| matches!(op, Op::CreateElement { .. } | Op::CreateText { .. }))
                .count()
        };
        assert_eq!(creates(&document) as u64, document.nodes - 1);
    }

    #[test]
    fn shuffles_rearrange_the_tree() {
        let plan = shuffle(300, 500, 7);
        assert_eq!(plan.work.len(), 500);
        assert_ne!(plan.setup_digest, plan.final_digest);
    }
}

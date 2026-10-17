//! Pattern B behind the benchmark [`Dom`] interface.
//!
//! Ops are handled as in `s1_b_dom::driver`: a safepoint after every op, plus
//! the safepoints inside allocating DOM operations; dispatch is the ordinary
//! function that roots its path and snapshots.

use s1_b_dom::dom::{
    Agent, NodeKind, NodeRef, add_listener, dispatch, insert_before, remove, remove_listener,
    set_attribute, tree_digest,
};
use s1_b_gc::{GcScope, GcToken, Metrics, Pacing};
use s1_scenarios::{LogEntry, NodeId, Op, Script};

use crate::dom::{Config, Dom, Fold, GARBAGE_TAG, HeapStats, ScenarioRun, kind};
use crate::harness::Pattern;

/// Never collects automatically: the budget can't be reached.
const NO_AUTO_COLLECTION: Pacing = Pacing {
    min_bytes: usize::MAX,
    growth_percent: 0,
};

/// Pattern B's DOM, configured for benchmarking.
#[derive(Debug)]
pub struct PatternB {
    agent: Agent,
    token: GcToken,
    garbage_per_node: u16,
}

/// The live handle for a plan id, read fresh from the roots.
fn node(agent: &Agent, id: NodeId) -> NodeRef<'static> {
    agent.globals.ids[id.index()].expect("plans only name live ids")
}

/// Every op except `Dispatch`, `Collect` and `Checkpoint`.
fn apply(agent: &mut Agent, op: &Op, garbage_per_node: u16, mut gc: GcScope<'_>) {
    match op {
        Op::CreateElement { id, tag } => {
            let created = agent
                .create_node(NodeKind::Element(*tag), Some(*id), gc.reborrow())
                .unbind();
            agent.globals.ids[id.index()] = Some(created);
            alloc_garbage(agent, garbage_per_node, gc);
        }
        Op::CreateText { id, data } => {
            let created = agent
                .create_node(NodeKind::Text(data.clone()), Some(*id), gc.reborrow())
                .unbind();
            agent.globals.ids[id.index()] = Some(created);
            alloc_garbage(agent, garbage_per_node, gc);
        }
        Op::AppendChild { parent, child } => {
            let (parent, child) = (node(agent, *parent), node(agent, *child));
            insert_before(&mut agent.heap, parent, child, None);
        }
        Op::InsertBefore {
            parent,
            child,
            reference,
        } => {
            let (parent, child, reference) = (
                node(agent, *parent),
                node(agent, *child),
                node(agent, *reference),
            );
            insert_before(&mut agent.heap, parent, child, Some(reference));
        }
        Op::Remove { node: target } => {
            let target = node(agent, *target);
            remove(&mut agent.heap, target);
        }
        Op::SetAttribute {
            node: target,
            name,
            value,
        } => {
            let target = node(agent, *target);
            set_attribute(&mut agent.heap.nodes[target], *name, value);
        }
        Op::AddListener {
            node: target,
            listener,
            spec,
        } => {
            let target = node(agent, *target);
            let extra = spec.extra_capture.map(|id| node(agent, id));
            add_listener(agent, target, *listener, spec, extra, gc);
        }
        Op::RemoveListener {
            node: target,
            listener,
        } => {
            let target = node(agent, *target);
            remove_listener(&mut agent.heap, target, *listener);
        }
        Op::Forget { node: target } => agent.globals.ids[target.index()] = None,
        Op::MakeWeak { .. } => unimplemented!("no benchmark plan makes weak references"),
        Op::Dispatch { .. } | Op::Collect | Op::Checkpoint => {
            unreachable!("handled by PatternB::apply")
        }
    }
}

/// Unreachable nodes, each behind its own safepoint (as `create_node` does).
fn alloc_garbage(agent: &mut Agent, count: u16, mut gc: GcScope<'_>) {
    for _ in 0..count {
        agent.create_node(NodeKind::Element(GARBAGE_TAG), None, gc.reborrow());
    }
}

impl Dom for PatternB {
    const PATTERN: Pattern = Pattern::B;

    fn new(node_slots: u32, config: Config) -> Self {
        let pacing = if config.auto_collect {
            Pacing::default()
        } else {
            NO_AUTO_COLLECTION
        };
        Self {
            agent: Agent::new(node_slots as usize, 0, pacing),
            token: GcToken::new(),
            garbage_per_node: config.garbage_per_node,
        }
    }

    fn apply(&mut self, ops: &[Op]) {
        let mut gc = GcScope::new(&mut self.token);
        for op in ops {
            match op {
                &Op::Dispatch {
                    target,
                    event,
                    bubbles,
                } => {
                    let target = node(&self.agent, target);
                    dispatch(&mut self.agent, target, event, bubbles, gc.reborrow());
                }
                Op::Collect | Op::Checkpoint => self.agent.collect(),
                op => apply(&mut self.agent, op, self.garbage_per_node, gc.reborrow()),
            }
            self.agent.safepoint(gc.reborrow());
        }
    }

    /// Pre-order, iterative, allocation-free. Each step indexes the node arena
    /// once: a bounds check (plus an epoch check in debug builds), then a
    /// plain field read.
    fn traverse(&self) -> Fold {
        let nodes = &self.agent.heap.nodes;
        let document = self.agent.globals.document;
        let mut fold = Fold::default();
        let mut node = document;
        loop {
            let mut data = &nodes[node];
            let code = match &data.kind {
                NodeKind::Document => kind::DOCUMENT,
                NodeKind::Element(tag) => kind::element(*tag),
                NodeKind::Text(_) => kind::TEXT,
            };
            fold.visit(data.id, code, data.attributes.len());
            if let Some(child) = data.first_child {
                node = child;
                continue;
            }
            loop {
                if node == document {
                    return fold;
                }
                if let Some(next) = data.next_sibling {
                    node = next;
                    break;
                }
                node = data.parent.expect("attached nodes have a parent");
                data = &nodes[node];
            }
        }
    }

    fn tree_digest(&self) -> u64 {
        tree_digest(&self.agent.heap, self.agent.globals.document)
    }

    fn collect(&mut self) {
        self.agent.collect();
    }

    fn log(&self) -> &[LogEntry] {
        &self.agent.log
    }

    fn clear_log(&mut self) {
        self.agent.log.clear();
    }

    fn heap(&self) -> HeapStats {
        self.agent.heap.core.metrics().into()
    }

    fn run_script(script: &Script) -> ScenarioRun {
        let report = s1_b_dom::run(script);
        ScenarioRun {
            outcome: report.outcome,
            peak_live_nodes: report.peak_live_nodes,
            heap: report.metrics.into(),
            dispatch_yields: None,
        }
    }
}

impl From<Metrics> for HeapStats {
    fn from(m: Metrics) -> Self {
        Self {
            objects: m.objects,
            bytes: m.bytes,
            peak_objects: m.peak_objects,
            peak_bytes: m.peak_bytes,
            total_allocated_objects: m.total_allocated_objects,
            total_allocated_bytes: m.total_allocated_bytes,
            collections: m.collections,
            max_pause: m.max_pause,
            total_pause: m.total_pause,
        }
    }
}

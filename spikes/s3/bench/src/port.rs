//! The DOM port behind a driver that applies `s1_scenarios` ops, as
//! `s1_bench::pattern_b` drives S1's Pattern B: a safepoint after every op,
//! plus the safepoints inside allocating operations.

use s1_bench::dom::{Fold, kind};
use s1_scenarios::{LogEntry, NodeId, Op};
use s3_gc::{Barrier, GcScope, GcToken, Metrics, Pacing, RootStack};

use crate::dom::{
    Agent, NodeKind, NodeRef, add_listener, dispatch, insert_before, remove, remove_listener,
    set_attribute, tree_digest,
};

/// The port, configured for benchmarking.
#[derive(Debug)]
pub struct Port<'r, S: RootStack, B: Barrier = s3_gc::NoBarrier> {
    agent: Agent<'r, S, B>,
    token: GcToken,
}

/// The live handle for a plan id, read fresh from the roots.
fn node<S: RootStack, B: Barrier>(agent: &Agent<'_, S, B>, id: NodeId) -> NodeRef<'static> {
    agent.globals.ids[id.index()].expect("plans only name live ids")
}

/// Every op except `Dispatch`, `Collect` and `Checkpoint`.
fn apply<S: RootStack, B: Barrier>(agent: &mut Agent<'_, S, B>, op: &Op, gc: GcScope<'_>) {
    match op {
        Op::CreateElement { id, tag } => {
            let created = agent
                .create_node(NodeKind::Element(*tag), Some(*id), gc)
                .unbind();
            agent.globals.ids[id.index()] = Some(created);
        }
        Op::CreateText { id, data } => {
            let created = agent
                .create_node(NodeKind::Text(data.clone()), Some(*id), gc)
                .unbind();
            agent.globals.ids[id.index()] = Some(created);
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
            unreachable!("handled by Port::apply")
        }
    }
}

impl<'r, S: RootStack, B: Barrier> Port<'r, S, B> {
    /// A port whose root frames use `roots`. Without `auto_collect`, nothing
    /// is collected unless asked.
    #[must_use]
    pub fn new(roots: &'r S, node_slots: u32, auto_collect: bool) -> Self {
        let pacing = if auto_collect {
            Pacing::default()
        } else {
            Pacing::NEVER
        };
        Self {
            agent: Agent::new(roots, node_slots as usize, pacing),
            token: GcToken::new(),
        }
    }

    pub fn apply(&mut self, ops: &[Op]) {
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
                op => apply(&mut self.agent, op, gc.reborrow()),
            }
            self.agent.safepoint(gc.reborrow());
        }
    }

    /// S1's traversal: pre-order, iterative, allocation-free, through the
    /// getters.
    #[must_use]
    pub fn traverse(&self) -> Fold {
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
            if let Some(child) = data.first_child() {
                node = child;
                continue;
            }
            loop {
                if node == document {
                    return fold;
                }
                if let Some(next) = data.next_sibling() {
                    node = next;
                    break;
                }
                node = data.parent().expect("attached nodes have a parent");
                data = &nodes[node];
            }
        }
    }

    #[must_use]
    pub fn tree_digest(&self) -> u64 {
        tree_digest(&self.agent.heap, self.agent.globals.document)
    }

    pub fn collect(&mut self) {
        self.agent.collect();
    }

    #[must_use]
    pub fn log(&self) -> &[LogEntry] {
        &self.agent.log
    }

    pub fn clear_log(&mut self) {
        self.agent.log.clear();
    }

    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.agent.heap.core.metrics()
    }

    #[must_use]
    pub const fn agent(&self) -> &Agent<'r, S, B> {
        &self.agent
    }
}

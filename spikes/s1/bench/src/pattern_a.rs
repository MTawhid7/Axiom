//! Pattern A behind the benchmark [`Dom`] interface.
//!
//! Ops are handled as in `s1_a_dom::driver`:
//! - one mutation scope per op;
//! - `collect_debt` between ops;
//! - dispatch as the resumable state machine stored in the root, with a
//!   collection whenever it yields.

use s1_a_dom::dispatch::{Dispatch, Step};
use s1_a_dom::dom::{
    Node, NodeKind, NodeRef, add_listener, insert_before, remove, remove_listener, set_attribute,
    tree_digest,
};
use s1_a_gc::{Arena, Gc, Metrics, Mutation, Pacing, Rootable, Trace};
use s1_scenarios::{EventType, LogEntry, NodeId, Op, Script};

use crate::dom::{Config, Dom, Fold, GARBAGE_TAG, HeapStats, ScenarioRun, kind};
use crate::harness::Pattern;

/// Never collects automatically: the budget can't be reached.
const NO_AUTO_COLLECTION: Pacing = Pacing {
    min_bytes: usize::MAX,
    growth_percent: 0,
};

/// The arena root: the document, the id table, and a dispatch in progress.
#[derive(Debug, Trace)]
pub struct Root<'gc> {
    document: NodeRef<'gc>,
    ids: Vec<Option<NodeRef<'gc>>>,
    dispatch: Option<Dispatch<'gc>>,
}

#[derive(Debug)]
pub struct Roots;

impl Rootable for Roots {
    type Root<'gc> = Root<'gc>;
}

/// Pattern A's DOM, configured for benchmarking.
#[derive(Debug)]
pub struct PatternA {
    arena: Arena<Roots>,
    log: Vec<LogEntry>,
    garbage_per_node: u16,
}

impl<'gc> Root<'gc> {
    fn node(&self, id: NodeId) -> NodeRef<'gc> {
        self.ids[id.index()].expect("plans only name live ids")
    }

    /// Every op except `Dispatch`, `Collect` and `Checkpoint`.
    fn apply(&mut self, mc: &Mutation<'gc>, op: &Op, garbage_per_node: u16) {
        match op {
            Op::CreateElement { id, tag } => {
                self.ids[id.index()] = Some(Node::alloc(mc, NodeKind::Element(*tag), Some(*id)));
                alloc_garbage(mc, garbage_per_node);
            }
            Op::CreateText { id, data } => {
                self.ids[id.index()] =
                    Some(Node::alloc(mc, NodeKind::Text(data.clone()), Some(*id)));
                alloc_garbage(mc, garbage_per_node);
            }
            Op::AppendChild { parent, child } => {
                insert_before(mc, self.node(*parent), self.node(*child), None);
            }
            Op::InsertBefore {
                parent,
                child,
                reference,
            } => insert_before(
                mc,
                self.node(*parent),
                self.node(*child),
                Some(self.node(*reference)),
            ),
            Op::Remove { node } => remove(mc, self.node(*node)),
            Op::SetAttribute { node, name, value } => {
                set_attribute(&self.node(*node), *name, value);
            }
            Op::AddListener {
                node,
                listener,
                spec,
            } => {
                let extra = spec.extra_capture.map(|id| self.node(id));
                add_listener(mc, self.node(*node), *listener, spec, extra);
            }
            Op::RemoveListener { node, listener } => {
                remove_listener(mc, self.node(*node), *listener);
            }
            Op::Forget { node } => self.ids[node.index()] = None,
            Op::MakeWeak { .. } => unimplemented!("no benchmark plan makes weak references"),
            Op::Dispatch { .. } | Op::Collect | Op::Checkpoint => {
                unreachable!("handled by PatternA::apply")
            }
        }
    }
}

fn alloc_garbage(mc: &Mutation<'_>, count: u16) {
    for _ in 0..count {
        Node::alloc(mc, NodeKind::Element(GARBAGE_TAG), None);
    }
}

impl PatternA {
    fn dispatch(&mut self, target: NodeId, event: EventType, bubbles: bool) {
        self.arena.mutate_root(|_, root| {
            root.dispatch = Some(Dispatch::begin(root.node(target), event, bubbles));
        });
        loop {
            let log = &mut self.log;
            let step = self.arena.mutate_root(|mc, root| {
                root.dispatch
                    .as_mut()
                    .expect("a dispatch is in progress")
                    .step(mc, log)
            });
            match step {
                Step::Finished => break,
                Step::Yielded => {
                    self.arena.collect_debt();
                }
            }
        }
        self.arena.mutate_root(|_, root| root.dispatch = None);
    }
}

impl Dom for PatternA {
    const PATTERN: Pattern = Pattern::A;

    fn new(node_slots: u32, config: Config) -> Self {
        let pacing = if config.auto_collect {
            Pacing::default()
        } else {
            NO_AUTO_COLLECTION
        };
        let arena = Arena::with_pacing(pacing, |mc| {
            let document = Node::alloc(mc, NodeKind::Document, Some(NodeId::DOCUMENT));
            let mut ids = vec![None; (node_slots as usize).max(1)];
            ids[NodeId::DOCUMENT.index()] = Some(document);
            Root {
                document,
                ids,
                dispatch: None,
            }
        });
        Self {
            arena,
            log: Vec::new(),
            garbage_per_node: config.garbage_per_node,
        }
    }

    fn apply(&mut self, ops: &[Op]) {
        let garbage = self.garbage_per_node;
        for op in ops {
            match op {
                &Op::Dispatch {
                    target,
                    event,
                    bubbles,
                } => self.dispatch(target, event, bubbles),
                Op::Collect | Op::Checkpoint => self.arena.collect_all(),
                op => self
                    .arena
                    .mutate_root(|mc, root| root.apply(mc, op, garbage)),
            }
            self.arena.collect_debt();
        }
    }

    fn traverse(&self) -> Fold {
        self.arena.mutate(|_, root| traverse(root.document))
    }

    fn tree_digest(&self) -> u64 {
        self.arena.mutate(|_, root| tree_digest(root.document))
    }

    fn collect(&mut self) {
        self.arena.collect_all();
    }

    fn log(&self) -> &[LogEntry] {
        &self.log
    }

    fn clear_log(&mut self) {
        self.log.clear();
    }

    fn heap(&self) -> HeapStats {
        self.arena.metrics().into()
    }

    fn run_script(script: &Script) -> ScenarioRun {
        let report = s1_a_dom::run(script);
        ScenarioRun {
            outcome: report.outcome,
            peak_live_nodes: report.peak_live_nodes,
            heap: report.metrics.into(),
            dispatch_yields: Some(report.dispatch_yields),
        }
    }
}

/// Pre-order, iterative, allocation-free. Reading the attribute count takes a
/// `RefCell` borrow: in this pattern every mutable field of a GC object needs
/// interior mutability, because the heap hands out only shared references.
fn traverse(document: NodeRef<'_>) -> Fold {
    let mut fold = Fold::default();
    let mut node = document;
    loop {
        let code = match &node.kind {
            NodeKind::Document => kind::DOCUMENT,
            NodeKind::Element(tag) => kind::element(*tag),
            NodeKind::Text(_) => kind::TEXT,
        };
        fold.visit(node.id, code, node.attributes.borrow().len());
        if let Some(child) = node.first_child.get() {
            node = child;
            continue;
        }
        loop {
            if Gc::ptr_eq(node, document) {
                return fold;
            }
            if let Some(next) = node.next_sibling.get() {
                node = next;
                break;
            }
            node = node.parent.get().expect("attached nodes have a parent");
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

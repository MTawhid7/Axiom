//! The script driver: executes ops, schedules collections, and records the outcome.

use s1_a_gc::{Arena, Gc, GcWeak, Metrics, Mutation, Pacing, Rootable, Trace};
use s1_scenarios::counters::{Closures, Nodes, live};
use s1_scenarios::{CheckpointRecord, LogEntry, NodeId, Op, Outcome, Script};

use crate::dispatch::{Dispatch, Step};
use crate::dom::{
    Node, NodeKind, NodeRef, add_listener, insert_before, remove, remove_listener, set_attribute,
    tree_digest,
};

/// The arena root: the document, the driver's id tables, and any dispatch in
/// progress. Everything the collector must keep alive is reachable from here.
#[derive(Debug, Trace)]
pub struct DomRoot<'gc> {
    document: NodeRef<'gc>,
    ids: Vec<Option<NodeRef<'gc>>>,
    weak: Vec<Option<GcWeak<'gc, Node<'gc>>>>,
    /// One more than the largest weak id created so far.
    weak_high: usize,
    dispatch: Option<Dispatch<'gc>>,
}

#[derive(Debug)]
pub struct Dom;

impl Rootable for Dom {
    type Root<'gc> = DomRoot<'gc>;
}

impl<'gc> DomRoot<'gc> {
    fn node(&self, id: NodeId) -> NodeRef<'gc> {
        self.ids[id.index()].expect("valid scripts only name live ids")
    }

    /// Every op except `Dispatch`, `Collect` and `Checkpoint`, which the
    /// driver handles because they cross mutation scopes.
    fn apply(&mut self, mc: &Mutation<'gc>, op: &Op) {
        match op {
            Op::CreateElement { id, tag } => {
                self.ids[id.index()] = Some(Node::alloc(mc, NodeKind::Element(*tag), Some(*id)));
            }
            Op::CreateText { id, data } => {
                self.ids[id.index()] =
                    Some(Node::alloc(mc, NodeKind::Text(data.clone()), Some(*id)));
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
            Op::MakeWeak { weak, target } => {
                self.weak[weak.index()] = Some(Gc::downgrade(self.node(*target)));
                self.weak_high = self.weak_high.max(weak.index() + 1);
            }
            Op::Forget { node } => self.ids[node.index()] = None,
            Op::Dispatch { .. } | Op::Collect | Op::Checkpoint => {
                unreachable!("handled by the driver")
            }
        }
    }

    /// Record the observable state. Must run right after a full collection.
    fn checkpoint(&self, base: (usize, usize)) -> CheckpointRecord {
        CheckpointRecord {
            tree_digest: tree_digest(self.document),
            live_nodes: live::<Nodes>() - base.0,
            live_closures: live::<Closures>() - base.1,
            weak_alive: self.weak[..self.weak_high]
                .iter()
                .map(|w| w.map(|w| !w.is_dead()))
                .collect(),
        }
    }
}

/// The outcome of a run, plus heap statistics.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub outcome: Outcome,
    /// The highest live-node count at any collection opportunity. Nodes die
    /// only in collections, so this is the exact peak.
    pub peak_live_nodes: usize,
    /// How often a dispatch yielded to let the collector run.
    pub dispatch_yields: u64,
    pub metrics: Metrics,
}

/// Run `script` with the default [`Pacing`].
pub fn run(script: &Script) -> RunReport {
    run_with(script, Pacing::default())
}

/// Run `script` with explicit pacing.
pub fn run_with(script: &Script, pacing: Pacing) -> RunReport {
    let base = (live::<Nodes>(), live::<Closures>());
    let mut arena = Arena::<Dom>::with_pacing(pacing, |mc| {
        let document = Node::alloc(mc, NodeKind::Document, Some(NodeId::DOCUMENT));
        let mut ids = vec![None; (script.node_slots as usize).max(1)];
        ids[NodeId::DOCUMENT.index()] = Some(document);
        DomRoot {
            document,
            ids,
            weak: vec![None; script.weak_slots as usize],
            weak_high: 0,
            dispatch: None,
        }
    });
    let mut log: Vec<LogEntry> = Vec::new();
    let mut checkpoints = Vec::new();
    let mut peak = 0;
    let mut dispatch_yields = 0;

    for op in &script.ops {
        match op {
            Op::Dispatch {
                target,
                event,
                bubbles,
            } => {
                arena.mutate_root(|_, root| {
                    root.dispatch = Some(Dispatch::begin(root.node(*target), *event, *bubbles));
                });
                loop {
                    let step = arena.mutate_root(|mc, root| {
                        root.dispatch
                            .as_mut()
                            .expect("a dispatch is in progress")
                            .step(mc, &mut log)
                    });
                    peak = peak.max(live::<Nodes>() - base.0);
                    match step {
                        Step::Finished => break,
                        Step::Yielded => {
                            dispatch_yields += 1;
                            arena.collect_debt();
                        }
                    }
                }
                arena.mutate_root(|_, root| root.dispatch = None);
            }
            Op::Collect => arena.collect_all(),
            Op::Checkpoint => {
                arena.collect_all();
                checkpoints.push(arena.mutate(|_, root| root.checkpoint(base)));
            }
            op => arena.mutate_root(|mc, root| root.apply(mc, op)),
        }
        peak = peak.max(live::<Nodes>() - base.0);
        arena.collect_debt();
    }

    RunReport {
        outcome: Outcome { log, checkpoints },
        peak_live_nodes: peak,
        dispatch_yields,
        metrics: arena.metrics(),
    }
}

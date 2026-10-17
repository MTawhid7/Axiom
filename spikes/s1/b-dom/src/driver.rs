//! The script driver: executes ops and records the outcome. Collections happen
//! at allocation safepoints and between ops.

use s1_b_gc::{GcScope, GcToken, Metrics, Pacing};
use s1_scenarios::counters::{Closures, Nodes, live};
use s1_scenarios::{CheckpointRecord, NodeId, Op, Outcome, Script};

use crate::dom::{
    Agent, NodeKind, NodeRef, add_listener, dispatch, insert_before, remove, remove_listener,
    set_attribute, tree_digest,
};

/// The outcome of a run, plus heap statistics.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub outcome: Outcome,
    /// The highest live-node count. It is sampled before every collection
    /// and at the end, and nodes die only in collections, so it is exact.
    pub peak_live_nodes: usize,
    pub metrics: Metrics,
}

/// Run `script` with the default [`Pacing`].
pub fn run(script: &Script) -> RunReport {
    run_with(script, Pacing::default())
}

/// Run `script` with explicit pacing.
pub fn run_with(script: &Script, pacing: Pacing) -> RunReport {
    let base_closures = live::<Closures>();
    let base_nodes = live::<Nodes>();
    let mut agent = Agent::new(
        script.node_slots as usize,
        script.weak_slots as usize,
        pacing,
    );
    let mut token = GcToken::new();
    let mut gc = GcScope::new(&mut token);
    let mut checkpoints = Vec::new();

    for op in &script.ops {
        match op {
            Op::Dispatch {
                target,
                event,
                bubbles,
            } => {
                let target = node(&agent, *target);
                dispatch(&mut agent, target, *event, *bubbles, gc.reborrow());
            }
            Op::Collect => agent.collect(),
            Op::Checkpoint => {
                agent.collect();
                checkpoints.push(checkpoint(&agent, base_nodes, base_closures));
            }
            op => apply(&mut agent, op, gc.reborrow()),
        }
        agent.safepoint(gc.reborrow());
    }
    agent.note_peak();

    RunReport {
        outcome: Outcome {
            log: std::mem::take(&mut agent.log),
            checkpoints,
        },
        peak_live_nodes: agent.peak_live_nodes,
        metrics: agent.heap.core.metrics(),
    }
}

/// The live handle for a scenario id. It is read fresh from the roots, so it
/// is always current.
fn node(agent: &Agent, id: NodeId) -> NodeRef<'static> {
    agent.globals.ids[id.index()].expect("valid scripts only name live ids")
}

/// Every op except `Dispatch`, `Collect` and `Checkpoint`.
fn apply(agent: &mut Agent, op: &Op, gc: GcScope<'_>) {
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
        Op::MakeWeak { weak, target } => {
            let target = node(agent, *target);
            agent.globals.weak[weak.index()] = Some(Some(target.downgrade()));
            agent.globals.weak_high = agent.globals.weak_high.max(weak.index() + 1);
        }
        Op::Forget { node: target } => agent.globals.ids[target.index()] = None,
        Op::Dispatch { .. } | Op::Collect | Op::Checkpoint => {
            unreachable!("handled by the driver")
        }
    }
}

/// Record the observable state. Must run right after a full collection.
fn checkpoint(agent: &Agent, base_nodes: usize, base_closures: usize) -> CheckpointRecord {
    let globals = &agent.globals;
    CheckpointRecord {
        tree_digest: tree_digest(&agent.heap, globals.document),
        live_nodes: live::<Nodes>() - base_nodes,
        live_closures: live::<Closures>() - base_closures,
        weak_alive: globals.weak[..globals.weak_high]
            .iter()
            .map(|slot| slot.map(|weak| weak.is_some()))
            .collect(),
    }
}

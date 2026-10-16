//! The reference interpreter: the oracle that both GC patterns must match.
//!
//! It favours obviousness over speed:
//! - Storage is plain vectors and indices, and nothing is ever freed.
//! - At each [`Op::Checkpoint`], the live set is computed by a graph search
//!   from the roots defined in [`crate::script`]. That is exactly the set a
//!   correct full collection must retain. Unreachable objects can never become
//!   reachable again, because ops name nodes only through live ids.
//! - Garbage from [`Behavior::AllocGarbage`] is counted but never stored,
//!   because nothing can observe it.
//! - Each listener and its closure are one record here. They are 1:1 in both
//!   patterns, so live closures equal reachable listeners.
//!
//! Generators also drive [`Reference`] step by step as their model of the DOM
//! (see [`crate::generate`]), so every generated script is valid by construction.

use core::fmt;
use core::iter::successors;

use crate::digest::TreeDigest;
use crate::outcome::{CheckpointRecord, LogEntry, Outcome, Phase};
use crate::script::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, NodeId, Op, Script, Tag, WeakId,
};

/// Index of a node in [`Reference::nodes`].
type Ix = usize;
/// Index of a listener in [`Reference::listeners`].
type Lx = usize;

const DOCUMENT: Ix = 0;

#[derive(Debug)]
enum Kind {
    Document,
    Element(Tag),
    Text(Box<str>),
}

#[derive(Debug)]
struct Node {
    kind: Kind,
    id: Option<NodeId>,
    parent: Option<Ix>,
    children: Vec<Ix>,
    attributes: Vec<(AttrName, Box<str>)>,
    listeners: Vec<Lx>,
}

impl Node {
    const fn new(kind: Kind, id: Option<NodeId>) -> Self {
        Self {
            kind,
            id,
            parent: None,
            children: Vec::new(),
            attributes: Vec::new(),
            listeners: Vec::new(),
        }
    }

    const fn can_have_children(&self) -> bool {
        !matches!(self.kind, Kind::Text(_))
    }
}

/// A listener together with its closure.
#[derive(Debug)]
struct Listener {
    id: ListenerId,
    event: EventType,
    capture: bool,
    removed: bool,
    behavior: Behavior,
    /// The closure's first capture: the node it was registered on.
    own: Ix,
    extra: Option<Ix>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Slot {
    #[default]
    Unused,
    Live(Ix),
    Forgotten,
}

/// Volume and coverage counters, used by tests and for sizing benchmarks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Every node ever created, including the document, fresh nodes and garbage.
    pub nodes_allocated: u64,
    /// Nodes created by [`Behavior::AllocGarbage`].
    pub garbage_allocated: u64,
    /// Every listener (and so every closure) ever created.
    pub listeners_allocated: u64,
    pub dispatches: u64,
    /// Listener invocations, which equals the number of log entries.
    pub invocations: u64,
    /// Dispatches that skipped at least one remaining node because of
    /// `stopPropagation`.
    pub stopped_dispatches: u64,
    /// Matching listeners that were skipped because their *removed* flag was
    /// set while they sat in a snapshot.
    pub skipped_removed: u64,
    /// `Collect` plus `Checkpoint` ops.
    pub collections: u64,
    /// The largest live-node count observed at any checkpoint.
    pub max_live_nodes: usize,
}

/// Why an op is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The node id was never created.
    UnknownNode(NodeId),
    /// The node id was forgotten earlier.
    ForgottenNode(NodeId),
    /// A creation op reused an existing node id.
    NodeIdReused(NodeId),
    /// The document cannot be forgotten.
    ForgetDocument,
    /// A tree op would insert the document, or make a node its own ancestor.
    HierarchyRequest { parent: NodeId, child: NodeId },
    /// Text nodes cannot have children.
    CannotHaveChildren(NodeId),
    /// The `InsertBefore` reference is not a child of the parent.
    NotAChild { parent: NodeId, reference: NodeId },
    /// Only elements have attributes.
    NotAnElement(NodeId),
    /// A listener id was reused.
    ListenerIdReused(ListenerId),
    /// `RemoveListener` named a listener that was never created.
    UnknownListener(ListenerId),
    /// A weak slot was reused.
    WeakIdReused(WeakId),
}

/// An invalid op, with its position in the script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub op_index: usize,
    pub kind: ErrorKind,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid op #{}: {:?}", self.op_index, self.kind)
    }
}

impl std::error::Error for ScriptError {}

/// The reference DOM model.
#[derive(Debug)]
pub struct Reference {
    nodes: Vec<Node>,
    listeners: Vec<Listener>,
    /// Indexed by [`NodeId`].
    ids: Vec<Slot>,
    /// Indexed by [`ListenerId`]: whether that id has been created.
    listener_created: Vec<bool>,
    /// Indexed by [`WeakId`]: the target of each created slot.
    weak: Vec<Option<Ix>>,
    outcome: Outcome,
    stats: Stats,
    applied: usize,
}

impl Default for Reference {
    fn default() -> Self {
        Self::new()
    }
}

/// Run `script` from a fresh document, returning its outcome and statistics.
pub fn run(script: &Script) -> Result<(Outcome, Stats), ScriptError> {
    let mut model = Reference::new();
    for op in &script.ops {
        model.apply(op)?;
    }
    Ok(model.into_parts())
}

impl Reference {
    /// A fresh model containing only the document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::new(Kind::Document, Some(NodeId::DOCUMENT))],
            listeners: Vec::new(),
            ids: vec![Slot::Live(DOCUMENT)],
            listener_created: Vec::new(),
            weak: Vec::new(),
            outcome: Outcome::default(),
            stats: Stats {
                nodes_allocated: 1,
                ..Stats::default()
            },
            applied: 0,
        }
    }

    /// Apply the next op of a script.
    pub fn apply(&mut self, op: &Op) -> Result<(), ScriptError> {
        let op_index = self.applied;
        self.applied += 1;
        self.step(op).map_err(|kind| ScriptError { op_index, kind })
    }

    /// The outcome so far.
    #[must_use]
    pub const fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    /// The statistics so far.
    #[must_use]
    pub const fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Consume the model, returning its outcome and statistics.
    #[must_use]
    pub fn into_parts(self) -> (Outcome, Stats) {
        (self.outcome, self.stats)
    }

    // ----- Queries (used by generators; unknown and forgotten ids are "absent") -----

    /// Whether `id` names a created, not-forgotten node.
    #[must_use]
    pub fn is_live(&self, id: NodeId) -> bool {
        self.ix(id).is_some()
    }

    /// Whether `id` is a live element.
    #[must_use]
    pub fn is_element(&self, id: NodeId) -> bool {
        self.ix(id)
            .is_some_and(|ix| matches!(self.nodes[ix].kind, Kind::Element(_)))
    }

    /// Whether `id` is a live element or the document.
    #[must_use]
    pub fn can_have_children(&self, id: NodeId) -> bool {
        self.ix(id)
            .is_some_and(|ix| self.nodes[ix].can_have_children())
    }

    /// The parent's id, if the parent has one that is still live. A parent whose
    /// id was forgotten still exists, but a script can no longer name it.
    #[must_use]
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        let ix = self.ix(id)?;
        self.nodes[ix].parent.and_then(|p| self.live_id_of(p))
    }

    /// The live ids of `id`'s children, in order. Children that have no id,
    /// or whose id was forgotten, are skipped.
    #[must_use]
    pub fn child_ids(&self, id: NodeId) -> Vec<NodeId> {
        self.ix(id).map_or_else(Vec::new, |ix| {
            self.nodes[ix]
                .children
                .iter()
                .filter_map(|&c| self.live_id_of(c))
                .collect()
        })
    }

    /// The node's id, if it has one and the driver still holds it.
    fn live_id_of(&self, ix: Ix) -> Option<NodeId> {
        self.nodes[ix].id.filter(|&id| self.ix(id) == Some(ix))
    }

    /// Whether `ancestor` is `node` or one of its ancestors.
    #[must_use]
    pub fn is_inclusive_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        match (self.ix(ancestor), self.ix(node)) {
            (Some(a), Some(n)) => self.is_inclusive_ancestor_ix(a, n),
            _ => false,
        }
    }

    /// The listeners currently registered on `id`, in order.
    #[must_use]
    pub fn listeners_on(&self, id: NodeId) -> Vec<ListenerId> {
        self.ix(id).map_or_else(Vec::new, |ix| {
            self.nodes[ix]
                .listeners
                .iter()
                .map(|&lx| self.listeners[lx].id)
                .collect()
        })
    }

    // ----- Op semantics -----

    fn step(&mut self, op: &Op) -> Result<(), ErrorKind> {
        match op {
            Op::CreateElement { id, tag } => self.create(*id, Kind::Element(*tag)),
            Op::CreateText { id, data } => self.create(*id, Kind::Text(data.clone())),
            Op::AppendChild { parent, child } => {
                let (p, c) = (self.live(*parent)?, self.live(*child)?);
                self.insert(p, c, None)
            }
            Op::InsertBefore {
                parent,
                child,
                reference,
            } => {
                let (p, c, r) = (
                    self.live(*parent)?,
                    self.live(*child)?,
                    self.live(*reference)?,
                );
                if self.nodes[r].parent != Some(p) {
                    return Err(ErrorKind::NotAChild {
                        parent: *parent,
                        reference: *reference,
                    });
                }
                self.insert(p, c, Some(r))
            }
            Op::Remove { node } => {
                let n = self.live(*node)?;
                self.detach(n);
                Ok(())
            }
            Op::SetAttribute { node, name, value } => {
                let n = self.live(*node)?;
                self.set_attribute(n, *name, value)
            }
            Op::AddListener {
                node,
                listener,
                spec,
            } => self.add_listener(*node, *listener, spec),
            Op::RemoveListener { node, listener } => {
                let n = self.live(*node)?;
                if !self
                    .listener_created
                    .get(listener.index())
                    .copied()
                    .unwrap_or(false)
                {
                    return Err(ErrorKind::UnknownListener(*listener));
                }
                self.remove_listener(n, *listener);
                Ok(())
            }
            Op::Dispatch {
                target,
                event,
                bubbles,
            } => {
                let t = self.live(*target)?;
                self.dispatch(t, *event, *bubbles)
            }
            Op::MakeWeak { weak, target } => {
                let t = self.live(*target)?;
                let slot = grow(&mut self.weak, weak.index());
                if slot.is_some() {
                    return Err(ErrorKind::WeakIdReused(*weak));
                }
                *slot = Some(t);
                Ok(())
            }
            Op::Forget { node } => {
                if *node == NodeId::DOCUMENT {
                    return Err(ErrorKind::ForgetDocument);
                }
                self.live(*node)?;
                self.ids[node.index()] = Slot::Forgotten;
                Ok(())
            }
            Op::Collect => {
                self.stats.collections += 1;
                Ok(())
            }
            Op::Checkpoint => {
                self.stats.collections += 1;
                self.checkpoint();
                Ok(())
            }
        }
    }

    fn ix(&self, id: NodeId) -> Option<Ix> {
        match self.ids.get(id.index()) {
            Some(Slot::Live(ix)) => Some(*ix),
            _ => None,
        }
    }

    fn live(&self, id: NodeId) -> Result<Ix, ErrorKind> {
        match self.ids.get(id.index()) {
            Some(Slot::Live(ix)) => Ok(*ix),
            Some(Slot::Forgotten) => Err(ErrorKind::ForgottenNode(id)),
            Some(Slot::Unused) | None => Err(ErrorKind::UnknownNode(id)),
        }
    }

    /// Id of a node named by an op (every such node has one).
    fn id_of(&self, ix: Ix) -> NodeId {
        self.nodes[ix].id.expect("op-reachable nodes have ids")
    }

    fn create(&mut self, id: NodeId, kind: Kind) -> Result<(), ErrorKind> {
        let ix = self.nodes.len();
        let slot = grow(&mut self.ids, id.index());
        if *slot != Slot::Unused {
            return Err(ErrorKind::NodeIdReused(id));
        }
        *slot = Slot::Live(ix);
        self.nodes.push(Node::new(kind, Some(id)));
        self.stats.nodes_allocated += 1;
        Ok(())
    }

    fn is_inclusive_ancestor_ix(&self, ancestor: Ix, node: Ix) -> bool {
        successors(Some(node), |&n| self.nodes[n].parent).any(|n| n == ancestor)
    }

    /// The DOM's "pre-insert", restricted to this model:
    /// 1. validate the insertion;
    /// 2. if the reference child is `child` itself, use `child`'s next sibling
    ///    instead;
    /// 3. remove `child` from its current parent;
    /// 4. insert it before the reference, or append it if there is none.
    fn insert(&mut self, parent: Ix, child: Ix, reference: Option<Ix>) -> Result<(), ErrorKind> {
        if !self.nodes[parent].can_have_children() {
            return Err(ErrorKind::CannotHaveChildren(self.id_of(parent)));
        }
        if child == DOCUMENT || self.is_inclusive_ancestor_ix(child, parent) {
            return Err(ErrorKind::HierarchyRequest {
                parent: self.id_of(parent),
                child: self.id_of(child),
            });
        }
        let reference = match reference {
            Some(r) if r == child => self.next_sibling(child),
            other => other,
        };
        self.detach(child);
        let children = &mut self.nodes[parent].children;
        let at = reference.map_or(children.len(), |r| {
            children
                .iter()
                .position(|&c| c == r)
                .expect("reference is a child of parent")
        });
        children.insert(at, child);
        self.nodes[child].parent = Some(parent);
        Ok(())
    }

    fn next_sibling(&self, ix: Ix) -> Option<Ix> {
        let parent = self.nodes[ix].parent?;
        let siblings = &self.nodes[parent].children;
        let at = siblings.iter().position(|&c| c == ix)?;
        siblings.get(at + 1).copied()
    }

    fn detach(&mut self, ix: Ix) {
        if let Some(parent) = self.nodes[ix].parent.take() {
            let siblings = &mut self.nodes[parent].children;
            let at = siblings
                .iter()
                .position(|&c| c == ix)
                .expect("a child is in its parent's child list");
            siblings.remove(at);
        }
    }

    fn set_attribute(&mut self, ix: Ix, name: AttrName, value: &str) -> Result<(), ErrorKind> {
        if !matches!(self.nodes[ix].kind, Kind::Element(_)) {
            return Err(ErrorKind::NotAnElement(self.id_of(ix)));
        }
        let attributes = &mut self.nodes[ix].attributes;
        match attributes.iter_mut().find(|(n, _)| *n == name) {
            Some((_, v)) => *v = value.into(),
            None => attributes.push((name, value.into())),
        }
        Ok(())
    }

    fn add_listener(
        &mut self,
        node: NodeId,
        listener: ListenerId,
        spec: &ListenerSpec,
    ) -> Result<(), ErrorKind> {
        let own = self.live(node)?;
        let extra = spec.extra_capture.map(|id| self.live(id)).transpose()?;
        let created = grow(&mut self.listener_created, listener.index());
        if *created {
            return Err(ErrorKind::ListenerIdReused(listener));
        }
        *created = true;
        let lx = self.listeners.len();
        self.listeners.push(Listener {
            id: listener,
            event: spec.event,
            capture: spec.capture,
            removed: false,
            behavior: spec.behavior.clone(),
            own,
            extra,
        });
        self.nodes[own].listeners.push(lx);
        self.stats.listeners_allocated += 1;
        Ok(())
    }

    fn remove_listener(&mut self, node: Ix, listener: ListenerId) {
        let listeners = &mut self.listeners;
        let list = &mut self.nodes[node].listeners;
        if let Some(at) = list.iter().position(|&lx| listeners[lx].id == listener) {
            let lx = list.remove(at);
            listeners[lx].removed = true;
        }
    }

    /// The dispatch algorithm documented on [`Op::Dispatch`].
    fn dispatch(&mut self, target: Ix, event: EventType, bubbles: bool) -> Result<(), ErrorKind> {
        self.stats.dispatches += 1;
        let path: Vec<Ix> = successors(Some(target), |&n| self.nodes[n].parent).collect();
        let mut stop = false;

        for (depth, &node) in path.iter().enumerate().rev() {
            if stop {
                self.stats.stopped_dispatches += 1;
                return Ok(());
            }
            let phase = if depth == 0 {
                Phase::AtTarget
            } else {
                Phase::Capturing
            };
            self.invoke(node, event, true, phase, &mut stop)?;
        }
        for (depth, &node) in path.iter().enumerate() {
            let phase = match depth {
                0 => Phase::AtTarget,
                _ if bubbles => Phase::Bubbling,
                _ => continue,
            };
            if stop {
                self.stats.stopped_dispatches += 1;
                return Ok(());
            }
            self.invoke(node, event, false, phase, &mut stop)?;
        }
        Ok(())
    }

    /// Invoke the matching listeners of one node in one pass, over a snapshot of
    /// its listener list.
    fn invoke(
        &mut self,
        node: Ix,
        event: EventType,
        capture: bool,
        phase: Phase,
        stop: &mut bool,
    ) -> Result<(), ErrorKind> {
        let current_target = self.id_of(node);
        let snapshot = self.nodes[node].listeners.clone();
        for lx in snapshot {
            let listener = &self.listeners[lx];
            if listener.event != event || listener.capture != capture {
                continue;
            }
            if listener.removed {
                self.stats.skipped_removed += 1;
                continue;
            }
            let (id, own, behavior) = (listener.id, listener.own, listener.behavior.clone());
            self.stats.invocations += 1;
            self.outcome.log.push(LogEntry {
                listener: id,
                current_target,
                phase,
            });
            self.run_behavior(&behavior, own, stop)?;
        }
        Ok(())
    }

    fn run_behavior(
        &mut self,
        behavior: &Behavior,
        own: Ix,
        stop: &mut bool,
    ) -> Result<(), ErrorKind> {
        match behavior {
            Behavior::LogOnly => {}
            Behavior::SetAttribute { name, value } => self.set_attribute(own, *name, value)?,
            Behavior::AppendFresh { tag, count } => {
                if !self.nodes[own].can_have_children() {
                    return Err(ErrorKind::CannotHaveChildren(self.id_of(own)));
                }
                for _ in 0..*count {
                    let child = self.nodes.len();
                    let mut fresh = Node::new(Kind::Element(*tag), None);
                    fresh.parent = Some(own);
                    self.nodes.push(fresh);
                    self.nodes[own].children.push(child);
                    self.stats.nodes_allocated += 1;
                }
            }
            Behavior::AllocGarbage { count } => {
                self.stats.nodes_allocated += u64::from(*count);
                self.stats.garbage_allocated += u64::from(*count);
            }
            Behavior::RemoveSelf => self.detach(own),
            Behavior::RemoveListener { listener } => self.remove_listener(own, *listener),
            Behavior::StopPropagation => *stop = true,
        }
        Ok(())
    }

    // ----- Checkpoints -----

    fn checkpoint(&mut self) {
        let (alive, live_closures) = self.reachability();
        let live_nodes = alive.iter().filter(|&&a| a).count();
        let weak_alive = self.weak.iter().map(|w| w.map(|ix| alive[ix])).collect();
        self.stats.max_live_nodes = self.stats.max_live_nodes.max(live_nodes);
        let tree_digest = self.tree_digest();
        self.outcome.checkpoints.push(CheckpointRecord {
            tree_digest,
            live_nodes,
            live_closures,
            weak_alive,
        });
    }

    /// Nodes reachable from the roots (the document and every live id), and
    /// the number of reachable closures.
    fn reachability(&self) -> (Vec<bool>, usize) {
        let mut node_seen = vec![false; self.nodes.len()];
        let mut listener_seen = vec![false; self.listeners.len()];
        let mut live_closures = 0;
        // ids[0] is the document's slot, so the roots include it.
        let mut stack: Vec<Ix> = self
            .ids
            .iter()
            .filter_map(|slot| match slot {
                Slot::Live(ix) => Some(*ix),
                Slot::Unused | Slot::Forgotten => None,
            })
            .collect();
        while let Some(ix) = stack.pop() {
            if core::mem::replace(&mut node_seen[ix], true) {
                continue;
            }
            let node = &self.nodes[ix];
            stack.extend(node.parent);
            stack.extend(&node.children);
            for &lx in &node.listeners {
                if !core::mem::replace(&mut listener_seen[lx], true) {
                    live_closures += 1;
                    let listener = &self.listeners[lx];
                    stack.push(listener.own);
                    stack.extend(listener.extra);
                }
            }
        }
        (node_seen, live_closures)
    }

    /// Canonical pre-order digest of the tree under the document (iterative, so
    /// deep trees cannot overflow the stack).
    fn tree_digest(&self) -> u64 {
        enum Step {
            Enter(Ix),
            Exit,
        }
        let mut digest = TreeDigest::new();
        let mut stack = vec![Step::Enter(DOCUMENT)];
        while let Some(step) = stack.pop() {
            let ix = match step {
                Step::Exit => {
                    digest.end_element();
                    continue;
                }
                Step::Enter(ix) => ix,
            };
            let node = &self.nodes[ix];
            let tag = match &node.kind {
                Kind::Text(data) => {
                    digest.text(node.id, data);
                    continue;
                }
                Kind::Document => Tag::Document,
                Kind::Element(tag) => *tag,
            };
            digest.begin_element(tag, node.id);
            for (name, value) in &node.attributes {
                digest.attribute(*name, value);
            }
            stack.push(Step::Exit);
            stack.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
        }
        digest.finish()
    }
}

/// `&mut v[i]`, growing `v` with defaults as needed.
fn grow<T: Default>(v: &mut Vec<T>, i: usize) -> &mut T {
    if v.len() <= i {
        v.resize_with(i + 1, T::default);
    }
    &mut v[i]
}

#[cfg(test)]
mod tests;

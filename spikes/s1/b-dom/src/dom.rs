//! The DOM object model and operations, in Nova style.

use core::iter::successors;

use s1_b_gc::{Forwarding, GcScope, Handle, Marker, Pacing, Roots, Trace, WeakHandle, heap};
use s1_scenarios::counters::{Closures, Live, Nodes, live};
use s1_scenarios::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, LogEntry, NodeId, Phase, Tag,
    TreeDigest,
};

pub type NodeRef<'a> = Handle<'a, NodeData>;
pub type ListenerRef<'a> = Handle<'a, ListenerData>;
pub type ClosureRef<'a> = Handle<'a, ClosureData>;

/// Bytes of heap per node. The pressure test uses it to turn the collection
/// budget into a node count.
pub const NODE_BYTES: usize = size_of::<NodeData>();

#[derive(Debug)]
pub enum NodeKind {
    Document,
    Element(Tag),
    Text(Box<str>),
}

#[derive(Debug)]
pub struct NodeData {
    pub id: Option<NodeId>,
    pub kind: NodeKind,
    pub parent: Option<NodeRef<'static>>,
    pub first_child: Option<NodeRef<'static>>,
    pub last_child: Option<NodeRef<'static>>,
    pub prev_sibling: Option<NodeRef<'static>>,
    pub next_sibling: Option<NodeRef<'static>>,
    pub attributes: Vec<(AttrName, Box<str>)>,
    pub listeners: Vec<ListenerRef<'static>>,
    _live: Live<Nodes>,
}

#[derive(Debug)]
pub struct ListenerData {
    pub id: ListenerId,
    pub event: EventType,
    pub capture: bool,
    pub removed: bool,
    pub closure: ClosureRef<'static>,
}

/// A listener's closure: its "code" ([`Behavior`]) and its captures.
#[derive(Debug)]
pub struct ClosureData {
    pub behavior: Behavior,
    /// The node the listener was registered on (a node ⇄ closure cycle).
    pub own: NodeRef<'static>,
    pub extra: Option<NodeRef<'static>>,
    _live: Live<Closures>,
}

heap! {
    /// The DOM heap: one arena per object kind.
    #[derive(Debug)]
    pub struct DomHeap {
        nodes: NodeData,
        listeners: ListenerData,
        closures: ClosureData,
    }
}

impl NodeData {
    fn new(kind: NodeKind, id: Option<NodeId>) -> Self {
        Self {
            id,
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            attributes: Vec::new(),
            listeners: Vec::new(),
            _live: Live::new(),
        }
    }
}

impl Trace for NodeData {
    fn mark(&self, marker: &mut Marker) {
        marker.mark_opt(self.parent);
        marker.mark_opt(self.first_child);
        marker.mark_opt(self.last_child);
        marker.mark_opt(self.prev_sibling);
        marker.mark_opt(self.next_sibling);
        for &listener in &self.listeners {
            marker.mark(listener);
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        forwarding.relocate_opt(&mut self.parent);
        forwarding.relocate_opt(&mut self.first_child);
        forwarding.relocate_opt(&mut self.last_child);
        forwarding.relocate_opt(&mut self.prev_sibling);
        forwarding.relocate_opt(&mut self.next_sibling);
        for listener in &mut self.listeners {
            forwarding.relocate(listener);
        }
    }
}

impl Trace for ListenerData {
    fn mark(&self, marker: &mut Marker) {
        marker.mark(self.closure);
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        forwarding.relocate(&mut self.closure);
    }
}

impl Trace for ClosureData {
    fn mark(&self, marker: &mut Marker) {
        marker.mark(self.own);
        marker.mark_opt(self.extra);
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        forwarding.relocate(&mut self.own);
        forwarding.relocate_opt(&mut self.extra);
    }
}

/// The driver's roots: the document, the id table, and the weak slots.
#[derive(Debug)]
pub struct Globals {
    pub document: NodeRef<'static>,
    pub ids: Vec<Option<NodeRef<'static>>>,
    /// `None` means not created. `Some(None)` means the target died.
    pub weak: Vec<Option<Option<WeakHandle<NodeData>>>>,
    pub weak_high: usize,
}

impl Roots for Globals {
    fn mark(&self, marker: &mut Marker) {
        marker.mark(self.document);
        for &id in self.ids.iter().flatten() {
            marker.mark(id);
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        forwarding.relocate(&mut self.document);
        for id in self.ids.iter_mut().flatten() {
            forwarding.relocate(id);
        }
        for slot in self.weak.iter_mut().flatten() {
            forwarding.relocate_weak(slot);
        }
    }
}

/// The heap, the driver's roots, and the outcome log. This is everything a
/// collection may touch.
#[derive(Debug)]
pub struct Agent {
    pub heap: DomHeap,
    pub globals: Globals,
    pub log: Vec<LogEntry>,
    base_nodes: usize,
    pub peak_live_nodes: usize,
}

impl Agent {
    #[must_use]
    pub fn new(node_slots: usize, weak_slots: usize, pacing: Pacing) -> Self {
        let mut heap = DomHeap::new(pacing);
        let base_nodes = live::<Nodes>();
        let document = heap.alloc(NodeData::new(NodeKind::Document, Some(NodeId::DOCUMENT)));
        let mut ids = vec![None; node_slots.max(1)];
        ids[NodeId::DOCUMENT.index()] = Some(document);
        Self {
            heap,
            globals: Globals {
                document,
                ids,
                weak: vec![None; weak_slots],
                weak_high: 0,
            },
            log: Vec::new(),
            base_nodes,
            peak_live_nodes: 0,
        }
    }

    /// Record the live-node count. Nodes die only in collections, so calling
    /// this before every collection and at the end gives the exact peak.
    pub fn note_peak(&mut self) {
        self.peak_live_nodes = self.peak_live_nodes.max(live::<Nodes>() - self.base_nodes);
    }

    /// A full collection.
    pub fn collect(&mut self) {
        self.note_peak();
        self.heap.collect(&mut self.globals);
    }

    /// A safepoint: collect if the allocation budget is spent. Taking a
    /// `GcScope` makes every caller give up its bound handles.
    pub fn safepoint(&mut self, _gc: GcScope<'_>) {
        if self.heap.core.wants_collection() {
            self.collect();
        }
    }

    /// Create a node, after a safepoint.
    pub fn create_node<'a>(
        &mut self,
        kind: NodeKind,
        id: Option<NodeId>,
        mut gc: GcScope<'a>,
    ) -> NodeRef<'a> {
        self.safepoint(gc.reborrow());
        self.heap
            .alloc(NodeData::new(kind, id))
            .bind(gc.into_nogc())
    }
}

/// Detach `node` from its parent. A no-op if it has none. Allocates nothing,
/// so it cannot collect and needs no scope.
pub fn remove(heap: &mut DomHeap, node: NodeRef<'_>) {
    let node = node.unbind();
    let Some(parent) = heap.nodes[node].parent else {
        return;
    };
    let (prev, next) = (heap.nodes[node].prev_sibling, heap.nodes[node].next_sibling);
    match prev {
        Some(prev) => heap.nodes[prev].next_sibling = next,
        None => heap.nodes[parent].first_child = next,
    }
    match next {
        Some(next) => heap.nodes[next].prev_sibling = prev,
        None => heap.nodes[parent].last_child = prev,
    }
    let data = &mut heap.nodes[node];
    data.parent = None;
    data.prev_sibling = None;
    data.next_sibling = None;
}

/// The DOM's "pre-insert": insert `child` into `parent` before `reference`,
/// or append it if `reference` is `None`. Validation is the script's job (CP2).
pub fn insert_before(
    heap: &mut DomHeap,
    parent: NodeRef<'_>,
    child: NodeRef<'_>,
    reference: Option<NodeRef<'_>>,
) {
    let (parent, child) = (parent.unbind(), child.unbind());
    // "If referenceChild is node, set it to node's next sibling."
    let reference = match reference.map(Handle::unbind) {
        Some(r) if r == child => heap.nodes[child].next_sibling,
        other => other,
    };
    remove(heap, child);
    let prev = match reference {
        Some(r) => heap.nodes[r].prev_sibling,
        None => heap.nodes[parent].last_child,
    };
    let data = &mut heap.nodes[child];
    data.parent = Some(parent);
    data.prev_sibling = prev;
    data.next_sibling = reference;
    match prev {
        Some(prev) => heap.nodes[prev].next_sibling = Some(child),
        None => heap.nodes[parent].first_child = Some(child),
    }
    match reference {
        Some(r) => heap.nodes[r].prev_sibling = Some(child),
        None => heap.nodes[parent].last_child = Some(child),
    }
}

/// An existing attribute keeps its position and gets the new value; a new one
/// is appended.
pub fn set_attribute(node: &mut NodeData, name: AttrName, value: &str) {
    match node.attributes.iter_mut().find(|(n, _)| *n == name) {
        Some((_, v)) => *v = value.into(),
        None => node.attributes.push((name, value.into())),
    }
}

/// Register a listener with a new closure that captures `node` (and `extra`).
///
/// Two allocations follow one safepoint. `node` and `extra` are rooted across
/// the safepoint, because compaction may move them.
pub fn add_listener(
    agent: &mut Agent,
    node: NodeRef<'_>,
    id: ListenerId,
    spec: &ListenerSpec,
    extra: Option<NodeRef<'_>>,
    mut gc: GcScope<'_>,
) {
    let frame = agent.heap.core.frame();
    let node = node.scope(&frame);
    let extra = extra.map(|extra| extra.scope(&frame));
    agent.safepoint(gc.reborrow());

    // No safepoint from here on: plain allocations never collect.
    let node = node.get().bind(gc.nogc());
    let closure = agent.heap.alloc(ClosureData {
        behavior: spec.behavior.clone(),
        own: node.unbind(),
        extra: extra.map(|extra| extra.get()),
        _live: Live::new(),
    });
    let listener = agent.heap.alloc(ListenerData {
        id,
        event: spec.event,
        capture: spec.capture,
        removed: false,
        closure,
    });
    agent.heap.nodes[node].listeners.push(listener);
}

/// Remove `id` from `node`'s listeners, if present, and set its removed flag.
pub fn remove_listener(heap: &mut DomHeap, node: NodeRef<'_>, id: ListenerId) {
    let listeners = &mut heap.nodes[node].listeners;
    if let Some(at) = listeners.iter().position(|&l| heap.listeners[l].id == id) {
        let removed = listeners.remove(at);
        heap.listeners[removed].removed = true;
    }
}

/// Dispatch an event, with exactly the semantics of
/// `s1_scenarios::Op::Dispatch`.
///
/// Listener code may allocate, which means it may collect, and compaction
/// moves every object. So the event path and each listener snapshot are
/// rooted in frames, and handles are re-read (`.get()`) after every listener
/// call. The borrow checker enforces this: a handle bound to `gc.nogc()`
/// cannot be used after `gc.reborrow()`.
pub fn dispatch(
    agent: &mut Agent,
    target: NodeRef<'_>,
    event: EventType,
    bubbles: bool,
    mut gc: GcScope<'_>,
) {
    let target = target.unbind();
    let path_frame = agent.heap.core.frame();
    // Sized first: one allocation instead of repeated growth (CP7 profile).
    let depth = successors(Some(target), |&node| agent.heap.nodes[node].parent).count();
    let mut path = Vec::with_capacity(depth);
    let mut cursor = Some(target);
    while let Some(node) = cursor {
        path.push(node.scope(&path_frame));
        cursor = agent.heap.nodes[node].parent;
    }

    // One frame and one buffer serve every visit's snapshot, so a visit
    // allocates nothing once the buffer has grown. Snapshot roots are
    // released when the dispatch returns. That is invisible to scripts:
    // collections inside a dispatch are not observable.
    let snapshot_frame = agent.heap.core.frame();
    let mut snapshot = Vec::new();
    let n = path.len();
    let visits = n + if bubbles { n } else { 1 };
    let mut stop = false;
    for visit in 0..visits {
        if stop {
            return;
        }
        let (index, capture, phase) = if visit < n {
            let i = n - 1 - visit;
            (
                i,
                true,
                if i == 0 {
                    Phase::AtTarget
                } else {
                    Phase::Capturing
                },
            )
        } else {
            let i = visit - n;
            (
                i,
                false,
                if i == 0 {
                    Phase::AtTarget
                } else {
                    Phase::Bubbling
                },
            )
        };
        let node = path[index].get().bind(gc.nogc());
        let current_target = agent.heap.nodes[node]
            .id
            .expect("listeners live on nodes with ids");

        snapshot.clear();
        snapshot.extend(
            agent.heap.nodes[node]
                .listeners
                .iter()
                .map(|listener| listener.scope(&snapshot_frame)),
        );
        for listener in &snapshot {
            let listener = &agent.heap.listeners[listener.get().bind(gc.nogc())];
            if listener.event != event || listener.capture != capture || listener.removed {
                continue;
            }
            agent.log.push(LogEntry {
                listener: listener.id,
                current_target,
                phase,
            });
            let closure = listener.closure;
            run_closure(agent, closure, &mut stop, gc.reborrow());
        }
    }
}

/// The closure's "native code". Allocating behaviours hit a safepoint on
/// every node they create.
fn run_closure(agent: &mut Agent, closure: ClosureRef<'_>, stop: &mut bool, mut gc: GcScope<'_>) {
    let closure = closure.bind(gc.nogc());
    let data = &agent.heap.closures[closure];
    let own = data.own;
    match &data.behavior {
        Behavior::LogOnly => {}
        Behavior::SetAttribute { name, value } => {
            set_attribute(&mut agent.heap.nodes[own], *name, value);
        }
        &Behavior::AppendFresh { tag, count } => {
            let frame = agent.heap.core.frame();
            let own = own.scope(&frame);
            for _ in 0..count {
                let fresh = agent
                    .create_node(NodeKind::Element(tag), None, gc.reborrow())
                    .unbind();
                insert_before(&mut agent.heap, own.get(), fresh, None);
            }
        }
        &Behavior::AllocGarbage { count } => {
            for _ in 0..count {
                agent.create_node(NodeKind::Element(Tag::Div), None, gc.reborrow());
            }
        }
        Behavior::RemoveSelf => remove(&mut agent.heap, own),
        &Behavior::RemoveListener { listener } => remove_listener(&mut agent.heap, own, listener),
        Behavior::StopPropagation => *stop = true,
    }
}

/// The canonical pre-order digest of the tree under `document`. It is
/// iterative, and it allocates nothing.
pub fn tree_digest(heap: &DomHeap, document: NodeRef<'_>) -> u64 {
    enum Step {
        Enter(NodeRef<'static>),
        Exit,
    }
    let mut digest = TreeDigest::new();
    let mut stack = vec![Step::Enter(document.unbind())];
    while let Some(step) = stack.pop() {
        let node = match step {
            Step::Exit => {
                digest.end_element();
                continue;
            }
            Step::Enter(node) => &heap.nodes[node],
        };
        let tag = match &node.kind {
            NodeKind::Text(data) => {
                digest.text(node.id, data);
                continue;
            }
            NodeKind::Document => Tag::Document,
            NodeKind::Element(tag) => *tag,
        };
        digest.begin_element(tag, node.id);
        for (name, value) in &node.attributes {
            digest.attribute(*name, value);
        }
        stack.push(Step::Exit);
        let mut children = Vec::new();
        let mut cursor = node.first_child;
        while let Some(child) = cursor {
            children.push(child);
            cursor = heap.nodes[child].next_sibling;
        }
        stack.extend(children.into_iter().rev().map(Step::Enter));
    }
    digest.finish()
}

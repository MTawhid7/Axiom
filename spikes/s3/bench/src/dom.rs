//! S1's Pattern B DOM (`s1_b_dom::dom`), ported to `s3-gc`: generic over the
//! root stack (S3-Q1) and the write barrier (S3-Q5).
//!
//! The operations are S1's, line for line, with two changes:
//! - **Handle-valued fields are private to their type** (ADR-0008 SH9). They
//!   are read through getters and written only through setters that
//!   [`s3_gc::setters!`] generates, or the listener-list methods below, each
//!   of which calls the heap's barrier. Fields set once, at construction,
//!   are set by constructors.
//! - The root stack is borrowed for `'r` rather than shared through an `Rc`.

use core::iter::successors;

use s1_scenarios::counters::{Nodes, live};
use s1_scenarios::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, LogEntry, NodeId, Phase, Tag,
    TreeDigest,
};
use s3_gc::{Barrier, Forwarding, GcScope, Handle, Marker, Pacing, RootStack, Roots, WeakHandle};

pub use node::{ClosureData, DomHeap, ListenerData, NodeData, NodeKind};

pub type NodeRef<'a> = Handle<'a, NodeData>;
pub type ListenerRef<'a> = Handle<'a, ListenerData>;
pub type ClosureRef<'a> = Handle<'a, ClosureData>;

/// The object model. Handle-valued fields are private to this module.
mod node {
    use s1_scenarios::counters::{Closures, Live, Nodes};
    use s1_scenarios::{AttrName, Behavior, EventType, ListenerId, NodeId, Tag};
    use s3_gc::{Allocated, Barrier, Forwarding, Marker, RootStack, Trace, heap, setters};

    use super::{ClosureRef, ListenerRef, NodeRef};

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
        parent: Option<NodeRef<'static>>,
        first_child: Option<NodeRef<'static>>,
        last_child: Option<NodeRef<'static>>,
        prev_sibling: Option<NodeRef<'static>>,
        next_sibling: Option<NodeRef<'static>>,
        pub attributes: Vec<(AttrName, Box<str>)>,
        listeners: Vec<ListenerRef<'static>>,
        _live: Live<Nodes>,
    }

    #[derive(Debug)]
    pub struct ListenerData {
        pub id: ListenerId,
        pub event: EventType,
        pub capture: bool,
        pub removed: bool,
        closure: ClosureRef<'static>,
    }

    /// A listener's closure: its "code" ([`Behavior`]) and its captures.
    #[derive(Debug)]
    pub struct ClosureData {
        pub behavior: Behavior,
        /// The node the listener was registered on (a node ⇄ closure cycle).
        own: NodeRef<'static>,
        extra: Option<NodeRef<'static>>,
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

    setters! {
        DomHeap.nodes: NodeData {
            set_parent => parent: Option<NodeRef<'static>>,
            set_first_child => first_child: Option<NodeRef<'static>>,
            set_last_child => last_child: Option<NodeRef<'static>>,
            set_prev_sibling => prev_sibling: Option<NodeRef<'static>>,
            set_next_sibling => next_sibling: Option<NodeRef<'static>>,
        }
    }

    impl<S: RootStack, B: Barrier> DomHeap<'_, S, B> {
        /// Append a listener to a node's list, through the barrier.
        pub fn push_listener(&mut self, node: NodeRef<'_>, listener: ListenerRef<'_>) {
            let node = node.unbind();
            self.nodes[node].listeners.push(listener.unbind());
            self.core.barrier.post_write(NodeData::ARENA, node.index());
        }

        /// Remove the listener at `at` from a node's list, through the barrier.
        pub fn remove_listener_at(&mut self, node: NodeRef<'_>, at: usize) -> ListenerRef<'static> {
            let node = node.unbind();
            let removed = self.nodes[node].listeners.remove(at);
            self.core.barrier.post_write(NodeData::ARENA, node.index());
            removed
        }
    }

    impl NodeData {
        pub(super) fn new(kind: NodeKind, id: Option<NodeId>) -> Self {
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

        pub fn parent(&self) -> Option<NodeRef<'static>> {
            self.parent
        }

        pub fn first_child(&self) -> Option<NodeRef<'static>> {
            self.first_child
        }

        pub fn last_child(&self) -> Option<NodeRef<'static>> {
            self.last_child
        }

        pub fn prev_sibling(&self) -> Option<NodeRef<'static>> {
            self.prev_sibling
        }

        pub fn next_sibling(&self) -> Option<NodeRef<'static>> {
            self.next_sibling
        }

        pub fn listeners(&self) -> &[ListenerRef<'static>] {
            &self.listeners
        }
    }

    impl ListenerData {
        pub(super) fn new(
            id: ListenerId,
            event: EventType,
            capture: bool,
            closure: ClosureRef<'_>,
        ) -> Self {
            Self {
                id,
                event,
                capture,
                removed: false,
                closure: closure.unbind(),
            }
        }

        pub fn closure(&self) -> ClosureRef<'static> {
            self.closure
        }
    }

    impl ClosureData {
        pub(super) fn new(
            behavior: Behavior,
            own: NodeRef<'_>,
            extra: Option<NodeRef<'_>>,
        ) -> Self {
            Self {
                behavior,
                own: own.unbind(),
                extra: extra.map(NodeRef::unbind),
                _live: Live::new(),
            }
        }

        pub fn own(&self) -> NodeRef<'static> {
            self.own
        }

        pub fn extra(&self) -> Option<NodeRef<'static>> {
            self.extra
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
pub struct Agent<'r, S: RootStack, B: Barrier = s3_gc::NoBarrier> {
    pub heap: DomHeap<'r, S, B>,
    pub globals: Globals,
    pub log: Vec<LogEntry>,
    base_nodes: usize,
    pub peak_live_nodes: usize,
}

impl<'r, S: RootStack, B: Barrier> Agent<'r, S, B> {
    #[must_use]
    pub fn new(roots: &'r S, node_slots: usize, pacing: Pacing) -> Self {
        let mut heap = DomHeap::new(roots, pacing);
        let base_nodes = live::<Nodes>();
        let document = heap.alloc(NodeData::new(NodeKind::Document, Some(NodeId::DOCUMENT)));
        let mut ids = vec![None; node_slots.max(1)];
        ids[NodeId::DOCUMENT.index()] = Some(document);
        Self {
            heap,
            globals: Globals {
                document,
                ids,
                weak: Vec::new(),
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
pub fn remove<S: RootStack, B: Barrier>(heap: &mut DomHeap<'_, S, B>, node: NodeRef<'_>) {
    let node = node.unbind();
    let Some(parent) = heap.nodes[node].parent() else {
        return;
    };
    let (prev, next) = (
        heap.nodes[node].prev_sibling(),
        heap.nodes[node].next_sibling(),
    );
    match prev {
        Some(prev) => heap.set_next_sibling(prev, next),
        None => heap.set_first_child(parent, next),
    }
    match next {
        Some(next) => heap.set_prev_sibling(next, prev),
        None => heap.set_last_child(parent, prev),
    }
    heap.set_parent(node, None);
    heap.set_prev_sibling(node, None);
    heap.set_next_sibling(node, None);
}

/// The DOM's "pre-insert": insert `child` into `parent` before `reference`,
/// or append it if `reference` is `None`. Validation is the script's job.
pub fn insert_before<S: RootStack, B: Barrier>(
    heap: &mut DomHeap<'_, S, B>,
    parent: NodeRef<'_>,
    child: NodeRef<'_>,
    reference: Option<NodeRef<'_>>,
) {
    let (parent, child) = (parent.unbind(), child.unbind());
    // "If referenceChild is node, set it to node's next sibling."
    let reference = match reference.map(Handle::unbind) {
        Some(r) if r == child => heap.nodes[child].next_sibling(),
        other => other,
    };
    remove(heap, child);
    let prev = match reference {
        Some(r) => heap.nodes[r].prev_sibling(),
        None => heap.nodes[parent].last_child(),
    };
    heap.set_parent(child, Some(parent));
    heap.set_prev_sibling(child, prev);
    heap.set_next_sibling(child, reference);
    match prev {
        Some(prev) => heap.set_next_sibling(prev, Some(child)),
        None => heap.set_first_child(parent, Some(child)),
    }
    match reference {
        Some(r) => heap.set_prev_sibling(r, Some(child)),
        None => heap.set_last_child(parent, Some(child)),
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
pub fn add_listener<S: RootStack, B: Barrier>(
    agent: &mut Agent<'_, S, B>,
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
    let closure = agent.heap.alloc(ClosureData::new(
        spec.behavior.clone(),
        node,
        extra.map(|extra| extra.get()),
    ));
    let listener = agent
        .heap
        .alloc(ListenerData::new(id, spec.event, spec.capture, closure));
    agent.heap.push_listener(node, listener);
}

/// Remove `id` from `node`'s listeners, if present, and set its removed flag.
pub fn remove_listener<S: RootStack, B: Barrier>(
    heap: &mut DomHeap<'_, S, B>,
    node: NodeRef<'_>,
    id: ListenerId,
) {
    let listeners = heap.nodes[node].listeners();
    if let Some(at) = listeners.iter().position(|&l| heap.listeners[l].id == id) {
        let removed = heap.remove_listener_at(node, at);
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
pub fn dispatch<S: RootStack, B: Barrier>(
    agent: &mut Agent<'_, S, B>,
    target: NodeRef<'_>,
    event: EventType,
    bubbles: bool,
    mut gc: GcScope<'_>,
) {
    let target = target.unbind();
    let path_frame = agent.heap.core.frame();
    // Sized first: one allocation instead of repeated growth (S1's CP7 profile).
    let depth = successors(Some(target), |&node| agent.heap.nodes[node].parent()).count();
    let mut path = Vec::with_capacity(depth);
    let mut cursor = Some(target);
    while let Some(node) = cursor {
        path.push(node.scope(&path_frame));
        cursor = agent.heap.nodes[node].parent();
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
                .listeners()
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
            let closure = listener.closure();
            run_closure(agent, closure, &mut stop, gc.reborrow());
        }
    }
}

/// The closure's "native code". Allocating behaviours hit a safepoint on
/// every node they create.
fn run_closure<S: RootStack, B: Barrier>(
    agent: &mut Agent<'_, S, B>,
    closure: ClosureRef<'_>,
    stop: &mut bool,
    mut gc: GcScope<'_>,
) {
    let closure = closure.bind(gc.nogc());
    let data = &agent.heap.closures[closure];
    let own = data.own();
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
pub fn tree_digest<S: RootStack, B: Barrier>(
    heap: &DomHeap<'_, S, B>,
    document: NodeRef<'_>,
) -> u64 {
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
        let mut cursor = node.first_child();
        while let Some(child) = cursor {
            children.push(child);
            cursor = heap.nodes[child].next_sibling();
        }
        stack.extend(children.into_iter().rev().map(Step::Enter));
    }
    digest.finish()
}

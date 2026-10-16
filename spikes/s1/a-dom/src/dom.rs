//! The DOM object model: nodes with intrusive tree links, listeners and
//! closures, all allocated in the garbage-collected heap.

use core::cell::{Cell, RefCell};
use core::fmt;
use core::iter::successors;

use s1_a_gc::{BOX_HEADER_BYTES, Gc, Lock, Mutation, RefLock, Trace, unlock};
use s1_scenarios::counters::{Closures, Live, Nodes};
use s1_scenarios::{AttrName, Behavior, EventType, ListenerId, NodeId, Tag, TreeDigest};

pub type NodeRef<'gc> = Gc<'gc, Node<'gc>>;
pub type ListenerRef<'gc> = Gc<'gc, Listener<'gc>>;

/// Heap bytes per node box. The pressure test uses it to turn the collection
/// budget into a node count.
pub const NODE_BOX_BYTES: usize = BOX_HEADER_BYTES + size_of::<Node<'static>>();

/// What a node is. This is `'static` data, so it is stored untraced.
#[derive(Debug)]
pub enum NodeKind {
    Document,
    Element(Tag),
    Text(Box<str>),
}

#[derive(Trace)]
pub struct Node<'gc> {
    #[trace(require_static)]
    pub id: Option<NodeId>,
    #[trace(require_static)]
    pub kind: NodeKind,
    pub parent: Lock<Option<NodeRef<'gc>>>,
    pub first_child: Lock<Option<NodeRef<'gc>>>,
    pub last_child: Lock<Option<NodeRef<'gc>>>,
    pub prev_sibling: Lock<Option<NodeRef<'gc>>>,
    pub next_sibling: Lock<Option<NodeRef<'gc>>>,
    /// Holds no `Gc`, so a plain `RefCell` is allowed. It is `Trace` only
    /// because it is `'static`, and needs no write barrier.
    pub attributes: RefCell<Vec<(AttrName, Box<str>)>>,
    pub listeners: RefLock<Vec<ListenerRef<'gc>>>,
    #[trace(require_static)]
    live: Live<Nodes>,
}

#[derive(Trace)]
pub struct Listener<'gc> {
    #[trace(require_static)]
    pub id: ListenerId,
    #[trace(require_static)]
    pub event: EventType,
    pub capture: bool,
    pub removed: Cell<bool>,
    pub closure: Gc<'gc, Closure<'gc>>,
}

/// A listener's closure: its "code" ([`Behavior`]) and its captures.
#[derive(Trace)]
pub struct Closure<'gc> {
    #[trace(require_static)]
    pub behavior: Behavior,
    /// The node the listener was registered on (a node ⇄ closure cycle).
    pub own: NodeRef<'gc>,
    pub extra: Option<NodeRef<'gc>>,
    #[trace(require_static)]
    live: Live<Closures>,
}

impl<'gc> Node<'gc> {
    pub fn alloc(mc: &Mutation<'gc>, kind: NodeKind, id: Option<NodeId>) -> NodeRef<'gc> {
        Gc::new(
            mc,
            Self {
                id,
                kind,
                parent: Lock::new(None),
                first_child: Lock::new(None),
                last_child: Lock::new(None),
                prev_sibling: Lock::new(None),
                next_sibling: Lock::new(None),
                attributes: RefCell::default(),
                listeners: RefLock::default(),
                live: Live::new(),
            },
        )
    }

    /// Children, in order.
    pub fn children(&self) -> impl Iterator<Item = NodeRef<'gc>> {
        successors(self.first_child.get(), |child| child.next_sibling.get())
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for Listener<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listener")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for Closure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Closure")
            .field("behavior", &self.behavior)
            .finish_non_exhaustive()
    }
}

/// Detach `node` from its parent. A no-op if it has none.
pub fn remove<'gc>(mc: &Mutation<'gc>, node: NodeRef<'gc>) {
    let Some(parent) = node.parent.get() else {
        return;
    };
    let (prev, next) = (node.prev_sibling.get(), node.next_sibling.get());
    match prev {
        Some(prev) => unlock!(Gc::write(mc, prev), Node, next_sibling).set(next),
        None => unlock!(Gc::write(mc, parent), Node, first_child).set(next),
    }
    match next {
        Some(next) => unlock!(Gc::write(mc, next), Node, prev_sibling).set(prev),
        None => unlock!(Gc::write(mc, parent), Node, last_child).set(prev),
    }
    let node = Gc::write(mc, node);
    unlock!(node, Node, parent).set(None);
    unlock!(node, Node, prev_sibling).set(None);
    unlock!(node, Node, next_sibling).set(None);
}

/// The DOM's "pre-insert": insert `child` into `parent` before `reference`,
/// or append it if `reference` is `None`. Validation is the script's job (CP2).
pub fn insert_before<'gc>(
    mc: &Mutation<'gc>,
    parent: NodeRef<'gc>,
    child: NodeRef<'gc>,
    reference: Option<NodeRef<'gc>>,
) {
    // "If referenceChild is node, set it to node's next sibling."
    let reference = match reference {
        Some(r) if Gc::ptr_eq(r, child) => child.next_sibling.get(),
        other => other,
    };
    remove(mc, child);
    let prev = match reference {
        Some(r) => r.prev_sibling.get(),
        None => parent.last_child.get(),
    };
    let written = Gc::write(mc, child);
    unlock!(written, Node, parent).set(Some(parent));
    unlock!(written, Node, prev_sibling).set(prev);
    unlock!(written, Node, next_sibling).set(reference);
    match prev {
        Some(prev) => unlock!(Gc::write(mc, prev), Node, next_sibling).set(Some(child)),
        None => unlock!(Gc::write(mc, parent), Node, first_child).set(Some(child)),
    }
    match reference {
        Some(r) => unlock!(Gc::write(mc, r), Node, prev_sibling).set(Some(child)),
        None => unlock!(Gc::write(mc, parent), Node, last_child).set(Some(child)),
    }
}

/// An existing attribute keeps its position and gets the new value; a new one
/// is appended.
pub fn set_attribute(node: &Node<'_>, name: AttrName, value: &str) {
    let mut attributes = node.attributes.borrow_mut();
    match attributes.iter_mut().find(|(n, _)| *n == name) {
        Some((_, v)) => *v = value.into(),
        None => attributes.push((name, value.into())),
    }
}

/// Register a listener with a new closure that captures `node` (and `extra`).
pub fn add_listener<'gc>(
    mc: &Mutation<'gc>,
    node: NodeRef<'gc>,
    id: ListenerId,
    spec: &s1_scenarios::ListenerSpec,
    extra: Option<NodeRef<'gc>>,
) {
    let closure = Gc::new(
        mc,
        Closure {
            behavior: spec.behavior.clone(),
            own: node,
            extra,
            live: Live::new(),
        },
    );
    let listener = Gc::new(
        mc,
        Listener {
            id,
            event: spec.event,
            capture: spec.capture,
            removed: Cell::new(false),
            closure,
        },
    );
    unlock!(Gc::write(mc, node), Node, listeners)
        .borrow_mut()
        .push(listener);
}

/// Remove `id` from `node`'s listeners, if present, and set its removed flag.
pub fn remove_listener<'gc>(mc: &Mutation<'gc>, node: NodeRef<'gc>, id: ListenerId) {
    let mut listeners = unlock!(Gc::write(mc, node), Node, listeners).borrow_mut();
    if let Some(at) = listeners.iter().position(|l| l.id == id) {
        listeners.remove(at).removed.set(true);
    }
}

/// The canonical pre-order digest of the tree under `document`. It is
/// iterative, so deep trees cannot overflow the stack.
pub fn tree_digest(document: NodeRef<'_>) -> u64 {
    enum Step<'gc> {
        Enter(NodeRef<'gc>),
        Exit,
    }
    let mut digest = TreeDigest::new();
    let mut stack = vec![Step::Enter(document)];
    while let Some(step) = stack.pop() {
        let node = match step {
            Step::Exit => {
                digest.end_element();
                continue;
            }
            Step::Enter(node) => node,
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
        for (name, value) in node.attributes.borrow().iter() {
            digest.attribute(*name, value);
        }
        stack.push(Step::Exit);
        let children: Vec<_> = node.children().collect();
        stack.extend(children.into_iter().rev().map(Step::Enter));
    }
    digest.finish()
}

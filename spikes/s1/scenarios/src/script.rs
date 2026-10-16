//! The script language: pattern-independent DOM operations and their semantics.
//!
//! This module is the contract that the reference interpreter (CP2), Pattern A
//! (CP4) and Pattern B (CP6) all implement. The semantics are a deliberately small
//! subset of the DOM Standard, chosen to exercise exactly what stresses a GC
//! design:
//! - cyclic object graphs;
//! - allocation during callbacks;
//! - state (the event path, listener snapshots) that must survive a collection
//!   in the middle of a dispatch.
//!
//! # Roots and reachability
//!
//! A driver holds exactly two kinds of strong roots:
//! - the document node, [`NodeId::DOCUMENT`], which never dies;
//! - its id table, with one strong reference per created and not-yet-forgotten
//!   [`NodeId`].
//!
//! Every other object is alive if and only if it is reachable from these roots
//! through strong edges:
//! - **parent ⇄ child:** both directions are strong. The tree links are
//!   ordinary traced references, so a detached subtree keeps itself alive as a
//!   cycle until nothing reaches it.
//! - **node → listener → closure:** a node strongly owns its listeners, and a
//!   listener owns its closure.
//! - **closure → captured nodes:** a closure captures its own node (the node it
//!   was registered on) and optionally one extra node.
//!
//! Weak references ([`Op::MakeWeak`]) never keep their target alive.

use core::fmt;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl $name {
            /// The id as a slot index into a driver's id table.
            #[inline]
            #[must_use]
            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    };
}

id_type!(
    /// Scenario-level node identity, assigned by [`ScriptBuilder`].
    NodeId
);
id_type!(
    /// Identity of a listener, so that it can be removed and appear in logs.
    ListenerId
);
id_type!(
    /// Slot of a weak reference held by the driver.
    WeakId
);

impl NodeId {
    /// The document node. Drivers create it before the first op, and it is
    /// never forgotten.
    pub const DOCUMENT: Self = Self(0);
}

/// Element tag names (a fixed, small vocabulary keeps scripts compact).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tag {
    /// The document node's tag in digests.
    Document,
    Html,
    Body,
    Div,
    Span,
    P,
    A,
    Ul,
    Li,
    Section,
    Button,
}

impl Tag {
    /// Tags a generator may create (everything except [`Tag::Document`]).
    pub const CREATABLE: [Self; 10] = [
        Self::Html,
        Self::Body,
        Self::Div,
        Self::Span,
        Self::P,
        Self::A,
        Self::Ul,
        Self::Li,
        Self::Section,
        Self::Button,
    ];

    /// The tag's name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "#document",
            Self::Html => "html",
            Self::Body => "body",
            Self::Div => "div",
            Self::Span => "span",
            Self::P => "p",
            Self::A => "a",
            Self::Ul => "ul",
            Self::Li => "li",
            Self::Section => "section",
            Self::Button => "button",
        }
    }
}

/// Attribute names (fixed vocabulary).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttrName {
    Id,
    Class,
    Title,
    Href,
    Role,
    Lang,
    DataA,
    DataB,
}

impl AttrName {
    /// Every attribute name, in a stable order.
    pub const ALL: [Self; 8] = [
        Self::Id,
        Self::Class,
        Self::Title,
        Self::Href,
        Self::Role,
        Self::Lang,
        Self::DataA,
        Self::DataB,
    ];

    /// The attribute's name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Class => "class",
            Self::Title => "title",
            Self::Href => "href",
            Self::Role => "role",
            Self::Lang => "lang",
            Self::DataA => "data-a",
            Self::DataB => "data-b",
        }
    }
}

/// Event types (fixed vocabulary).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventType {
    Click,
    Input,
    Focus,
    Custom,
}

impl EventType {
    /// Every event type, in a stable order.
    pub const ALL: [Self; 4] = [Self::Click, Self::Input, Self::Focus, Self::Custom];
}

/// What a listener's closure does when invoked: the "native code" of the closure.
///
/// Every invocation first appends a [`LogEntry`](crate::LogEntry); then the
/// behaviour runs. "Own node" means the closure's first capture, which is the
/// node the listener was registered on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Behavior {
    /// Only the log entry.
    LogOnly,
    /// Set an attribute on the own node (same semantics as [`Op::SetAttribute`]).
    SetAttribute { name: AttrName, value: Box<str> },
    /// Create `count` fresh, id-less elements with tag `tag`, and append each one
    /// to the own node.
    AppendFresh { tag: Tag, count: u16 },
    /// Allocate `count` fresh, id-less elements and never attach them. They are
    /// garbage the moment the closure returns. This is GC pressure inside a dispatch.
    AllocGarbage { count: u16 },
    /// Detach the own node from its parent (a no-op if it has none). The event
    /// path does not change, because it was computed when the dispatch began.
    RemoveSelf,
    /// Set the event's stop-propagation flag.
    StopPropagation,
}

/// Registration parameters for [`Op::AddListener`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListenerSpec {
    /// Only events of this type invoke the listener.
    pub event: EventType,
    /// `true`: invoked in the capture pass; `false`: in the bubble pass.
    pub capture: bool,
    /// What the closure does.
    pub behavior: Behavior,
    /// An optional second node captured by the closure: a strong edge that
    /// can join otherwise-separate subtrees into one cycle.
    pub extra_capture: Option<NodeId>,
}

/// One DOM operation.
///
/// Scripts produced by [`ScriptBuilder`] (and later by the generators) are
/// **valid**:
/// - ids are created before use and never used after [`Op::Forget`];
/// - tree operations never create a cycle in the tree;
/// - an `InsertBefore` reference is a child of the parent.
///
/// The reference interpreter checks validity (CP2). Implementations may assume it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// Create an element and bind it to `id`. The driver then holds it strongly.
    CreateElement { id: NodeId, tag: Tag },
    /// Create a text node holding `data` and bind it to `id`.
    CreateText { id: NodeId, data: Box<str> },
    /// Append `child` as the last child of `parent`, first removing it from its
    /// current parent if it has one (the DOM's move semantics).
    AppendChild { parent: NodeId, child: NodeId },
    /// Insert `child` into `parent` immediately before `reference`, a child of
    /// `parent`. Like `AppendChild`, `child` is first removed from its current
    /// parent. Inserting a node before itself is a no-op.
    InsertBefore {
        parent: NodeId,
        child: NodeId,
        reference: NodeId,
    },
    /// Detach `node` from its parent. A no-op if it has none.
    Remove { node: NodeId },
    /// Set an attribute on an element. An existing attribute keeps its position
    /// and gets the new value; a new attribute is appended.
    SetAttribute {
        node: NodeId,
        name: AttrName,
        value: Box<str>,
    },
    /// Register a new listener (with its own new closure) on `node`. It is
    /// appended to the node's listener list.
    AddListener {
        node: NodeId,
        listener: ListenerId,
        spec: ListenerSpec,
    },
    /// Remove `listener` from `node`'s list and set its *removed* flag. A dispatch
    /// in progress that has it in a snapshot skips it from then on.
    RemoveListener { node: NodeId, listener: ListenerId },
    /// Dispatch an event at `target`.
    ///
    /// 1. The path is `target` followed by its ancestors, fixed when the dispatch
    ///    starts.
    /// 2. **Capture pass:** walk the path from the outermost ancestor to `target`.
    ///    The phase is `Capturing` for ancestors and `AtTarget` for the target.
    /// 3. **Bubble pass:** walk from `target` outward. The phase is `AtTarget` for
    ///    the target. Ancestors are visited, with phase `Bubbling`, only if
    ///    `bubbles` is set.
    /// 4. **At each visit:**
    ///    - first, if the stop-propagation flag is set, the dispatch ends;
    ///    - otherwise, the node's listener list is snapshotted;
    ///    - each listener in the snapshot is invoked if it is not removed, its
    ///      event type matches, and its `capture` flag matches the pass.
    Dispatch {
        target: NodeId,
        event: EventType,
        bubbles: bool,
    },
    /// Create a weak reference to `target` in slot `weak`.
    MakeWeak { weak: WeakId, target: NodeId },
    /// Drop the driver's strong reference for `node`. The id is never used again.
    Forget { node: NodeId },
    /// Run a full collection (no observable effect, except on timing and memory).
    Collect,
    /// Run a full collection, then record a
    /// [`CheckpointRecord`](crate::CheckpointRecord).
    Checkpoint,
}

/// A complete script, plus the id-table sizes a driver must allocate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Script {
    /// The operations, in execution order.
    pub ops: Vec<Op>,
    /// Number of node-id slots, including the document's slot 0.
    pub node_slots: u32,
    /// Number of listener-id slots.
    pub listener_slots: u32,
    /// Number of weak-reference slots.
    pub weak_slots: u32,
}

/// Builds a [`Script`], allocating ids sequentially.
///
/// The builder does not validate tree operations; the reference interpreter
/// does (CP2).
#[derive(Debug)]
pub struct ScriptBuilder {
    ops: Vec<Op>,
    next_node: u32,
    next_listener: u32,
    next_weak: u32,
}

impl Default for ScriptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBuilder {
    /// An empty script. Node id 0 is reserved for the document.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ops: Vec::new(),
            next_node: 1,
            next_listener: 0,
            next_weak: 0,
        }
    }

    /// Ops recorded so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Whether no ops have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    fn fresh_node(&mut self) -> NodeId {
        let id = NodeId(self.next_node);
        self.next_node = self.next_node.checked_add(1).expect("node ids exhausted");
        id
    }

    /// `CreateElement` with a fresh id.
    pub fn element(&mut self, tag: Tag) -> NodeId {
        let id = self.fresh_node();
        self.ops.push(Op::CreateElement { id, tag });
        id
    }

    /// `CreateText` with a fresh id.
    pub fn text(&mut self, data: impl Into<Box<str>>) -> NodeId {
        let id = self.fresh_node();
        self.ops.push(Op::CreateText {
            id,
            data: data.into(),
        });
        id
    }

    /// `AppendChild`.
    pub fn append(&mut self, parent: NodeId, child: NodeId) -> &mut Self {
        self.ops.push(Op::AppendChild { parent, child });
        self
    }

    /// `InsertBefore`.
    pub fn insert_before(&mut self, parent: NodeId, child: NodeId, reference: NodeId) -> &mut Self {
        self.ops.push(Op::InsertBefore {
            parent,
            child,
            reference,
        });
        self
    }

    /// `Remove`.
    pub fn remove(&mut self, node: NodeId) -> &mut Self {
        self.ops.push(Op::Remove { node });
        self
    }

    /// `SetAttribute`.
    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: AttrName,
        value: impl Into<Box<str>>,
    ) -> &mut Self {
        self.ops.push(Op::SetAttribute {
            node,
            name,
            value: value.into(),
        });
        self
    }

    /// `AddListener` with a fresh listener id.
    pub fn add_listener(&mut self, node: NodeId, spec: ListenerSpec) -> ListenerId {
        let listener = ListenerId(self.next_listener);
        self.next_listener = self
            .next_listener
            .checked_add(1)
            .expect("listener ids exhausted");
        self.ops.push(Op::AddListener {
            node,
            listener,
            spec,
        });
        listener
    }

    /// `RemoveListener`.
    pub fn remove_listener(&mut self, node: NodeId, listener: ListenerId) -> &mut Self {
        self.ops.push(Op::RemoveListener { node, listener });
        self
    }

    /// `Dispatch`.
    pub fn dispatch(&mut self, target: NodeId, event: EventType, bubbles: bool) -> &mut Self {
        self.ops.push(Op::Dispatch {
            target,
            event,
            bubbles,
        });
        self
    }

    /// `MakeWeak` with a fresh weak slot.
    pub fn make_weak(&mut self, target: NodeId) -> WeakId {
        let weak = WeakId(self.next_weak);
        self.next_weak = self.next_weak.checked_add(1).expect("weak ids exhausted");
        self.ops.push(Op::MakeWeak { weak, target });
        weak
    }

    /// `Forget`.
    pub fn forget(&mut self, node: NodeId) -> &mut Self {
        self.ops.push(Op::Forget { node });
        self
    }

    /// `Collect`.
    pub fn collect(&mut self) -> &mut Self {
        self.ops.push(Op::Collect);
        self
    }

    /// `Checkpoint`.
    pub fn checkpoint(&mut self) -> &mut Self {
        self.ops.push(Op::Checkpoint);
        self
    }

    /// Finishes the script.
    #[must_use]
    pub fn build(self) -> Script {
        Script {
            ops: self.ops,
            node_slots: self.next_node,
            listener_slots: self.next_listener,
            weak_slots: self.next_weak,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_assigns_sequential_ids_and_reserves_the_document() {
        let mut b = ScriptBuilder::new();
        let div = b.element(Tag::Div);
        let text = b.text("hi");
        b.append(NodeId::DOCUMENT, div).append(div, text);
        let l = b.add_listener(
            div,
            ListenerSpec {
                event: EventType::Click,
                capture: false,
                behavior: Behavior::LogOnly,
                extra_capture: Some(text),
            },
        );
        let w = b.make_weak(text);
        b.dispatch(text, EventType::Click, true).checkpoint();
        let script = b.build();

        assert_eq!(div, NodeId(1));
        assert_eq!(text, NodeId(2));
        assert_eq!(l, ListenerId(0));
        assert_eq!(w, WeakId(0));
        assert_eq!(script.node_slots, 3, "document + 2 nodes");
        assert_eq!(script.listener_slots, 1);
        assert_eq!(script.weak_slots, 1);
        assert_eq!(script.ops.len(), 8);
        assert_eq!(
            script.ops[0],
            Op::CreateElement {
                id: div,
                tag: Tag::Div
            }
        );
        assert_eq!(script.ops.last(), Some(&Op::Checkpoint));
    }

    #[test]
    fn vocabularies_have_unique_names() {
        let mut tags: Vec<_> = Tag::CREATABLE.iter().map(|t| t.as_str()).collect();
        tags.push(Tag::Document.as_str());
        let n = tags.len();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), n);

        let mut attrs: Vec<_> = AttrName::ALL.iter().map(|a| a.as_str()).collect();
        attrs.sort_unstable();
        attrs.dedup();
        assert_eq!(attrs.len(), AttrName::ALL.len());
    }
}

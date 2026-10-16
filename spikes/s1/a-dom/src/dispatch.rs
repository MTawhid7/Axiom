//! Event dispatch as a resumable state machine.
//!
//! **This is Pattern A's central ergonomic cost.** A collection can only run
//! *between* mutation scopes, but a dispatch runs arbitrary listener code that
//! may allocate without bound.
//!
//! So a dispatch can't be one function call holding `Gc`s on the Rust stack.
//! All of its state lives in the arena root, where the collector can trace it:
//! - the event path;
//! - the current listener snapshot;
//! - the cursor;
//! - the stop flag.
//!
//! [`Dispatch::step`] returns [`Step::Yielded`] whenever the heap asks for a
//! collection. The driver then collects and calls `step` again. A real JS
//! engine on this pattern would need its *interpreter* to be stackless in the
//! same way (as piccolo's is), not just its dispatcher.
//!
//! The semantics are exactly those of `s1_scenarios::Op::Dispatch`. The two
//! passes are flattened into one sequence of *visits*: the capture pass from the
//! outermost ancestor to the target, then the bubble pass from the target
//! outward. When `bubbles` is false, the bubble pass has only the target.

use core::iter::successors;

use s1_a_gc::{Gc, Mutation, Trace};
use s1_scenarios::{Behavior, EventType, LogEntry, Phase, Tag};

use crate::dom::{
    Closure, ListenerRef, Node, NodeKind, NodeRef, insert_before, remove, remove_listener,
    set_attribute,
};

/// What [`Dispatch::step`] stopped for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The heap wants a collection. Collect, then call `step` again.
    Yielded,
    Finished,
}

/// The state of one dispatch in progress.
#[derive(Debug, Trace)]
pub struct Dispatch<'gc> {
    #[trace(require_static)]
    event: EventType,
    bubbles: bool,
    /// The target first, then its ancestors. Fixed when the dispatch begins.
    path: Vec<NodeRef<'gc>>,
    /// Index of the next visit in the flattened sequence.
    visit: usize,
    current_node: Option<NodeRef<'gc>>,
    current_capture: bool,
    #[trace(require_static)]
    current_phase: Phase,
    /// The current node's listeners, as they were when its visit began.
    snapshot: Vec<ListenerRef<'gc>>,
    next: usize,
    stop: bool,
}

impl<'gc> Dispatch<'gc> {
    pub fn begin(target: NodeRef<'gc>, event: EventType, bubbles: bool) -> Self {
        Self {
            event,
            bubbles,
            path: successors(Some(target), |n| n.parent.get()).collect(),
            visit: 0,
            current_node: None,
            current_capture: true,
            current_phase: Phase::AtTarget,
            snapshot: Vec::new(),
            next: 0,
            stop: false,
        }
    }

    fn visit_count(&self) -> usize {
        let n = self.path.len();
        n + if self.bubbles { n } else { 1 }
    }

    /// (node, capture, phase) of visit `v`.
    fn visit_at(&self, v: usize) -> (NodeRef<'gc>, bool, Phase) {
        let n = self.path.len();
        if v < n {
            let i = n - 1 - v;
            let phase = if i == 0 {
                Phase::AtTarget
            } else {
                Phase::Capturing
            };
            (self.path[i], true, phase)
        } else {
            let i = v - n;
            let phase = if i == 0 {
                Phase::AtTarget
            } else {
                Phase::Bubbling
            };
            (self.path[i], false, phase)
        }
    }

    /// Run listeners until the dispatch finishes or the heap wants a collection.
    pub fn step(&mut self, mc: &Mutation<'gc>, log: &mut Vec<LogEntry>) -> Step {
        loop {
            if self.next == self.snapshot.len() {
                if self.stop || self.visit == self.visit_count() {
                    return Step::Finished;
                }
                let (node, capture, phase) = self.visit_at(self.visit);
                self.visit += 1;
                self.snapshot.clear();
                self.snapshot
                    .extend(node.listeners.borrow().iter().copied());
                self.next = 0;
                self.current_node = Some(node);
                self.current_capture = capture;
                self.current_phase = phase;
                continue;
            }
            let listener = self.snapshot[self.next];
            self.next += 1;
            if listener.event != self.event
                || listener.capture != self.current_capture
                || listener.removed.get()
            {
                continue;
            }
            let node = self.current_node.expect("a visit is in progress");
            log.push(LogEntry {
                listener: listener.id,
                current_target: node.id.expect("listeners live on nodes with ids"),
                phase: self.current_phase,
            });
            self.run(mc, listener.closure);
            if mc.wants_collection() {
                return Step::Yielded;
            }
        }
    }

    /// The closure's "native code".
    fn run(&mut self, mc: &Mutation<'gc>, closure: Gc<'gc, Closure<'gc>>) {
        let own = closure.own;
        match &closure.behavior {
            Behavior::LogOnly => {}
            Behavior::SetAttribute { name, value } => set_attribute(&own, *name, value),
            Behavior::AppendFresh { tag, count } => {
                for _ in 0..*count {
                    let fresh = Node::alloc(mc, NodeKind::Element(*tag), None);
                    insert_before(mc, own, fresh, None);
                }
            }
            Behavior::AllocGarbage { count } => {
                for _ in 0..*count {
                    Node::alloc(mc, NodeKind::Element(Tag::Div), None);
                }
            }
            Behavior::RemoveSelf => remove(mc, own),
            Behavior::RemoveListener { listener } => remove_listener(mc, own, *listener),
            Behavior::StopPropagation => self.stop = true,
        }
    }
}

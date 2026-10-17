//! A corpus of small scripts, each aimed at one rarely hit semantic path.
//!
//! The scenarios mirror the reference's hand-checked golden tests
//! (`reference/tests.rs`), but the roles differ:
//! - **The golden tests** validate the *reference* against hand-written
//!   expectations.
//! - **This corpus** lets every *pattern* be compared with the validated
//!   reference on exactly those paths. Seeded workloads hit them only by chance.
#![allow(
    clippy::many_single_char_names,
    reason = "nodes are named like the DOM Standard's examples (a, t, p, x, y, z)"
)]

use crate::script::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, NodeId, Script, ScriptBuilder, Tag,
};

const DOC: NodeId = NodeId::DOCUMENT;

fn listener(event: EventType, capture: bool, behavior: Behavior) -> ListenerSpec {
    ListenerSpec {
        event,
        capture,
        behavior,
        extra_capture: None,
    }
}

fn log(capture: bool) -> ListenerSpec {
    listener(EventType::Click, capture, Behavior::LogOnly)
}

/// `(name, script)` for every edge case.
#[must_use]
pub fn all() -> Vec<(&'static str, Script)> {
    vec![
        ("phases-and-non-bubbling", phases()),
        ("stop-mid-path", stop_mid_path()),
        ("stop-at-target-capture", stop_at_target_capture()),
        ("removal-pair", removal_pair()),
        ("self-cycle-weak-forget", self_cycle()),
        ("extra-capture-keeps-detached", extra_capture()),
        ("fixed-path-and-fresh-children", fixed_path()),
        ("garbage-in-dispatch", garbage()),
        ("dom-insertion-semantics", tree_ops()),
        ("deep-nesting", deep_nesting()),
    ]
}

fn phases() -> Script {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let t = b.element(Tag::Span);
    b.append(DOC, a).append(a, t);
    for (node, capture) in [
        (DOC, true),
        (a, true),
        (t, true),
        (t, false),
        (a, false),
        (DOC, false),
    ] {
        b.add_listener(node, log(capture));
    }
    b.dispatch(t, EventType::Click, true)
        .dispatch(t, EventType::Click, false)
        .dispatch(t, EventType::Input, true)
        .checkpoint();
    b.build()
}

fn stop_mid_path() -> Script {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let t = b.element(Tag::Span);
    b.append(DOC, a).append(a, t);
    b.add_listener(DOC, log(true));
    b.add_listener(
        a,
        listener(EventType::Click, true, Behavior::StopPropagation),
    );
    b.add_listener(a, log(true));
    b.add_listener(t, log(false));
    b.dispatch(t, EventType::Click, true).checkpoint();
    b.build()
}

fn stop_at_target_capture() -> Script {
    let mut b = ScriptBuilder::new();
    let t = b.element(Tag::Div);
    b.append(DOC, t);
    b.add_listener(
        t,
        listener(EventType::Click, true, Behavior::StopPropagation),
    );
    b.add_listener(t, log(false));
    b.dispatch(t, EventType::Click, true).checkpoint();
    b.build()
}

fn removal_pair() -> Script {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.append(DOC, a);
    let victim = ListenerId(b.peek_next_listener().0 + 1);
    b.add_listener(
        a,
        listener(
            EventType::Click,
            false,
            Behavior::RemoveListener { listener: victim },
        ),
    );
    b.add_listener(a, log(false));
    b.add_listener(a, log(false));
    b.dispatch(a, EventType::Click, true)
        .dispatch(a, EventType::Click, true)
        .checkpoint();
    b.build()
}

fn self_cycle() -> Script {
    let mut b = ScriptBuilder::new();
    let div = b.element(Tag::Div);
    b.append(DOC, div);
    b.add_listener(div, log(false));
    b.make_weak(div);
    b.checkpoint();
    b.remove(div).checkpoint();
    b.forget(div).checkpoint();
    b.build()
}

fn extra_capture() -> Script {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let c = b.element(Tag::Span);
    b.append(DOC, a);
    let l = b.add_listener(
        a,
        ListenerSpec {
            extra_capture: Some(c),
            ..log(false)
        },
    );
    b.make_weak(c);
    b.forget(c).checkpoint();
    b.remove_listener(a, l).checkpoint();
    b.build()
}

fn fixed_path() -> Script {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let t = b.element(Tag::Span);
    b.append(DOC, a).append(a, t);
    b.add_listener(t, listener(EventType::Click, false, Behavior::RemoveSelf));
    b.add_listener(
        a,
        listener(
            EventType::Click,
            false,
            Behavior::AppendFresh {
                tag: Tag::Span,
                count: 2,
            },
        ),
    );
    b.add_listener(DOC, log(false));
    b.dispatch(t, EventType::Click, true).checkpoint();
    b.forget(t).checkpoint();
    b.build()
}

fn garbage() -> Script {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.append(DOC, a);
    b.add_listener(
        a,
        listener(EventType::Click, false, Behavior::AllocGarbage { count: 7 }),
    );
    b.dispatch(a, EventType::Click, true)
        .dispatch(a, EventType::Click, true)
        .checkpoint();
    b.build()
}

fn tree_ops() -> Script {
    let mut b = ScriptBuilder::new();
    let p = b.element(Tag::Div);
    let (x, y, z) = (
        b.element(Tag::Span),
        b.element(Tag::Span),
        b.element(Tag::Span),
    );
    b.append(DOC, p).append(p, x).append(p, y).append(p, z);
    b.insert_before(p, z, x).checkpoint(); // z x y
    b.insert_before(p, y, y).checkpoint(); // before itself: no-op
    b.insert_before(p, z, z).checkpoint(); // before itself, at the front
    b.append(p, z).checkpoint(); // append moves z to the end: x y z
    let q = b.element(Tag::Div);
    b.append(DOC, q).append(q, y); // a move across parents
    b.set_attribute(p, AttrName::Class, "a")
        .set_attribute(p, AttrName::Id, "i")
        .set_attribute(p, AttrName::Class, "b");
    let t = b.text("hello");
    b.append(x, t).checkpoint();
    b.remove(x).remove(x).checkpoint(); // removing a detached node is a no-op
    b.build()
}

fn deep_nesting() -> Script {
    let mut b = ScriptBuilder::new();
    let mut parent = DOC;
    let mut chain = Vec::new();
    // Shallower under Miri: with collect-always pacing every op runs a full collection.
    let depth = if cfg!(miri) { 64 } else { 512 };
    for _ in 0..depth {
        let child = b.element(Tag::Div);
        b.append(parent, child);
        chain.push(child);
        parent = child;
    }
    b.add_listener(chain[0], log(false));
    b.dispatch(parent, EventType::Click, true).checkpoint();
    b.remove(chain[0]);
    for id in chain {
        b.forget(id);
    }
    b.checkpoint(); // the whole chain is now garbage
    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference;

    #[test]
    fn every_edge_case_is_valid_and_checkpointed() {
        for (name, script) in all() {
            let (outcome, _) = reference::run(&script).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!outcome.checkpoints.is_empty(), "{name}");
        }
    }
}

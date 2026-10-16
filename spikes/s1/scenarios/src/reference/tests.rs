//! Golden tests: small hand-written scripts with hand-checked outcomes.
#![allow(
    clippy::many_single_char_names,
    reason = "nodes are named like the DOM Standard's examples (a, t, p, x, y, z)"
)]

use super::*;
use crate::outcome::Phase::{AtTarget, Bubbling, Capturing};
use crate::script::{Behavior::*, EventType::*, ScriptBuilder};

const DOC: NodeId = NodeId::DOCUMENT;

fn listener(event: EventType, capture: bool, behavior: Behavior) -> ListenerSpec {
    ListenerSpec {
        event,
        capture,
        behavior,
        extra_capture: None,
    }
}

fn entry(listener: ListenerId, current_target: NodeId, phase: Phase) -> LogEntry {
    LogEntry {
        listener,
        current_target,
        phase,
    }
}

fn run_ok(b: ScriptBuilder) -> (Outcome, Stats) {
    run(&b.build()).expect("golden scripts are valid")
}

fn run_err(b: ScriptBuilder) -> ScriptError {
    run(&b.build()).expect_err("script must be rejected")
}

/// Digest of `document { <children written by build> }`.
fn digest(build: impl FnOnce(&mut TreeDigest)) -> u64 {
    let mut d = TreeDigest::new();
    d.begin_element(Tag::Document, Some(DOC));
    build(&mut d);
    d.end_element();
    d.finish()
}

#[test]
fn dispatch_visits_capture_target_and_bubble_phases_in_order() {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let t = b.element(Tag::Span);
    b.append(DOC, a).append(a, t);
    let l0 = b.add_listener(DOC, listener(Click, true, LogOnly));
    let l1 = b.add_listener(a, listener(Click, true, LogOnly));
    let l2 = b.add_listener(t, listener(Click, true, LogOnly));
    let l3 = b.add_listener(t, listener(Click, false, LogOnly));
    let l4 = b.add_listener(a, listener(Click, false, LogOnly));
    let l5 = b.add_listener(DOC, listener(Click, false, LogOnly));
    b.dispatch(t, Click, true)
        .dispatch(t, Click, false)
        .dispatch(t, Input, true);
    let (outcome, stats) = run_ok(b);

    assert_eq!(
        outcome.log,
        [
            entry(l0, DOC, Capturing),
            entry(l1, a, Capturing),
            entry(l2, t, AtTarget),
            entry(l3, t, AtTarget),
            entry(l4, a, Bubbling),
            entry(l5, DOC, Bubbling),
            // bubbles = false: the target's non-capture listeners still run;
            // the ancestors' do not.
            entry(l0, DOC, Capturing),
            entry(l1, a, Capturing),
            entry(l2, t, AtTarget),
            entry(l3, t, AtTarget),
            // The Input dispatch matches no listener.
        ]
    );
    assert_eq!((stats.dispatches, stats.invocations), (3, 10));
}

#[test]
fn stop_propagation_finishes_the_current_node_then_stops() {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let t = b.element(Tag::Span);
    b.append(DOC, a).append(a, t);
    let l0 = b.add_listener(DOC, listener(Click, true, LogOnly));
    let l1 = b.add_listener(a, listener(Click, true, StopPropagation));
    let l2 = b.add_listener(a, listener(Click, true, LogOnly));
    let _never = b.add_listener(t, listener(Click, false, LogOnly));
    b.dispatch(t, Click, true);
    let (outcome, stats) = run_ok(b);

    assert_eq!(
        outcome.log,
        [
            entry(l0, DOC, Capturing),
            entry(l1, a, Capturing),
            // The same node's remaining listeners still run (no stopImmediatePropagation).
            entry(l2, a, Capturing),
        ]
    );
    assert_eq!(stats.stopped_dispatches, 1);
}

#[test]
fn stop_propagation_during_capture_at_target_skips_the_bubble_pass_at_target() {
    let mut b = ScriptBuilder::new();
    let t = b.element(Tag::Div);
    b.append(DOC, t);
    let l0 = b.add_listener(t, listener(Click, true, StopPropagation));
    let _l1 = b.add_listener(t, listener(Click, false, LogOnly));
    b.dispatch(t, Click, true);
    let (outcome, stats) = run_ok(b);

    assert_eq!(outcome.log, [entry(l0, t, AtTarget)]);
    assert_eq!(stats.stopped_dispatches, 1);
}

#[test]
fn a_listener_removed_mid_dispatch_is_skipped_even_though_it_is_in_the_snapshot() {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.append(DOC, a);
    // The remover names the id the *next* add_listener will assign.
    let victim = ListenerId(b.peek_next_listener().0 + 1);
    let x = b.add_listener(
        a,
        listener(Click, false, RemoveListener { listener: victim }),
    );
    let y = b.add_listener(a, listener(Click, false, LogOnly));
    assert_eq!(y, victim);
    let z = b.add_listener(a, listener(Click, false, LogOnly));
    b.dispatch(a, Click, true)
        .dispatch(a, Click, true)
        .checkpoint();
    let (outcome, stats) = run_ok(b);

    assert_eq!(
        outcome.log,
        [
            entry(x, a, AtTarget),
            entry(z, a, AtTarget),
            entry(x, a, AtTarget), // removing y again is a no-op
            entry(z, a, AtTarget),
        ]
    );
    assert_eq!(stats.skipped_removed, 1, "y is skipped once, by its flag");
    let checkpoint = &outcome.checkpoints[0];
    assert_eq!(
        (checkpoint.live_nodes, checkpoint.live_closures),
        (2, 2),
        "y's closure is unreachable once y is removed"
    );
}

#[test]
fn a_self_capturing_cycle_lives_while_rooted_and_dies_when_unreachable() {
    let mut b = ScriptBuilder::new();
    let div = b.element(Tag::Div);
    b.append(DOC, div);
    // The closure captures `div`, which owns the listener: a div ⇄ closure cycle.
    b.add_listener(div, listener(Click, false, LogOnly));
    let weak = b.make_weak(div);
    b.checkpoint(); // attached and held by the driver
    b.remove(div).checkpoint(); // detached, but still held by the driver's id table
    b.forget(div).checkpoint(); // an unreachable cycle: it must die
    let (outcome, _) = run_ok(b);

    assert_eq!(weak, WeakId(0));
    let with_div = digest(|d| {
        d.begin_element(Tag::Div, Some(div));
        d.end_element();
    });
    let empty = digest(|_| {});
    let record = |tree_digest, live_nodes, live_closures, alive| CheckpointRecord {
        tree_digest,
        live_nodes,
        live_closures,
        weak_alive: vec![Some(alive)],
    };
    assert_eq!(
        outcome.checkpoints,
        [
            record(with_div, 2, 1, true),
            record(empty, 2, 1, true),
            record(empty, 1, 0, false),
        ]
    );
}

#[test]
fn an_extra_capture_keeps_a_forgotten_detached_node_alive_until_the_listener_goes() {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let c = b.element(Tag::Span); // never attached
    b.append(DOC, a);
    let l = b.add_listener(
        a,
        ListenerSpec {
            extra_capture: Some(c),
            ..listener(Click, false, LogOnly)
        },
    );
    b.forget(c).checkpoint();
    b.remove_listener(a, l).checkpoint();
    let (outcome, _) = run_ok(b);

    let live: Vec<_> = outcome
        .checkpoints
        .iter()
        .map(|c| (c.live_nodes, c.live_closures))
        .collect();
    assert_eq!(live, [(3, 1), (2, 0)]);
}

#[test]
fn the_event_path_is_fixed_at_dispatch_start_and_fresh_children_join_the_tree() {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    let t = b.element(Tag::Span);
    b.append(DOC, a).append(a, t);
    let l0 = b.add_listener(t, listener(Click, false, RemoveSelf));
    let l1 = b.add_listener(
        a,
        listener(
            Click,
            false,
            AppendFresh {
                tag: Tag::Span,
                count: 2,
            },
        ),
    );
    let l2 = b.add_listener(DOC, listener(Click, false, LogOnly));
    b.dispatch(t, Click, true).checkpoint();
    let (outcome, stats) = run_ok(b);

    // `t` detaches itself first, yet its former ancestors are still visited.
    assert_eq!(
        outcome.log,
        [
            entry(l0, t, AtTarget),
            entry(l1, a, Bubbling),
            entry(l2, DOC, Bubbling),
        ]
    );
    let expected_tree = digest(|d| {
        d.begin_element(Tag::Div, Some(a));
        for _ in 0..2 {
            d.begin_element(Tag::Span, None);
            d.end_element();
        }
        d.end_element();
    });
    let checkpoint = &outcome.checkpoints[0];
    assert_eq!(checkpoint.tree_digest, expected_tree);
    assert_eq!(
        (checkpoint.live_nodes, checkpoint.live_closures),
        (5, 3),
        "document, a, t (detached but held), and the two fresh spans"
    );
    assert_eq!(stats.nodes_allocated, 5);
}

#[test]
fn garbage_is_counted_but_never_observable() {
    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.append(DOC, a);
    b.add_listener(a, listener(Click, false, AllocGarbage { count: 7 }));
    b.dispatch(a, Click, true)
        .dispatch(a, Click, true)
        .checkpoint();
    let (outcome, stats) = run_ok(b);

    assert_eq!(stats.garbage_allocated, 14);
    assert_eq!(stats.nodes_allocated, 2 + 14);
    assert_eq!(outcome.checkpoints[0].live_nodes, 2);
}

#[test]
fn tree_ops_follow_dom_insertion_semantics() {
    let mut b = ScriptBuilder::new();
    let p = b.element(Tag::Div);
    let (x, y, z) = (
        b.element(Tag::Span),
        b.element(Tag::Span),
        b.element(Tag::Span),
    );
    b.append(DOC, p).append(p, x).append(p, y).append(p, z);
    b.insert_before(p, z, x).checkpoint(); // z x y
    b.insert_before(p, y, y).checkpoint(); // before itself: no-op, so still z x y
    b.append(p, z).checkpoint(); // append moves z to the end: x y z
    let q = b.element(Tag::Div);
    b.append(DOC, q).append(q, y); // a move across parents. p: x z, q: y
    b.set_attribute(p, AttrName::Class, "a")
        .set_attribute(p, AttrName::Id, "i")
        .set_attribute(p, AttrName::Class, "b"); // replaced in place
    let t = b.text("hello");
    b.append(x, t).checkpoint();
    let (outcome, _) = run_ok(b);

    let order = |kids: &[NodeId]| {
        digest(|d| {
            d.begin_element(Tag::Div, Some(p));
            for &k in kids {
                d.begin_element(Tag::Span, Some(k));
                d.end_element();
            }
            d.end_element();
        })
    };
    let final_tree = digest(|d| {
        d.begin_element(Tag::Div, Some(p));
        d.attribute(AttrName::Class, "b");
        d.attribute(AttrName::Id, "i");
        d.begin_element(Tag::Span, Some(x));
        d.text(Some(t), "hello");
        d.end_element();
        d.begin_element(Tag::Span, Some(z));
        d.end_element();
        d.end_element();
        d.begin_element(Tag::Div, Some(q));
        d.begin_element(Tag::Span, Some(y));
        d.end_element();
        d.end_element();
    });
    let digests: Vec<u64> = outcome.checkpoints.iter().map(|c| c.tree_digest).collect();
    assert_eq!(
        digests,
        [
            order(&[z, x, y]),
            order(&[z, x, y]),
            order(&[x, y, z]),
            final_tree
        ]
    );
}

#[test]
fn invalid_ops_are_rejected_with_their_position_and_reason() {
    use ErrorKind::*;

    // An ancestor inserted into its own descendant. Ops: create, create, append, append, append.
    let mut b = ScriptBuilder::new();
    let (a, c) = (b.element(Tag::Div), b.element(Tag::Div));
    b.append(DOC, a).append(a, c).append(c, a);
    assert_eq!(
        run_err(b),
        ScriptError {
            op_index: 4,
            kind: HierarchyRequest {
                parent: c,
                child: a
            }
        }
    );

    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.append(a, DOC);
    assert_eq!(
        run_err(b).kind,
        HierarchyRequest {
            parent: a,
            child: DOC
        }
    );

    let mut b = ScriptBuilder::new();
    let (t, a) = (b.text("x"), b.element(Tag::Div));
    b.append(t, a);
    assert_eq!(run_err(b).kind, CannotHaveChildren(t));

    let mut b = ScriptBuilder::new();
    let (a, c, x) = (
        b.element(Tag::Div),
        b.element(Tag::Div),
        b.element(Tag::Div),
    );
    b.insert_before(a, c, x);
    assert_eq!(
        run_err(b).kind,
        NotAChild {
            parent: a,
            reference: x
        }
    );

    let mut b = ScriptBuilder::new();
    let t = b.text("x");
    b.set_attribute(t, AttrName::Id, "i");
    assert_eq!(run_err(b).kind, NotAnElement(t));

    let mut b = ScriptBuilder::new();
    b.forget(DOC);
    assert_eq!(run_err(b).kind, ForgetDocument);

    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.forget(a).append(DOC, a);
    assert_eq!(
        run_err(b),
        ScriptError {
            op_index: 2,
            kind: ForgottenNode(a)
        }
    );

    let mut b = ScriptBuilder::new();
    b.append(DOC, NodeId(99));
    assert_eq!(run_err(b).kind, UnknownNode(NodeId(99)));

    let mut b = ScriptBuilder::new();
    let a = b.element(Tag::Div);
    b.remove_listener(a, ListenerId(5));
    assert_eq!(run_err(b).kind, UnknownListener(ListenerId(5)));

    let reused = Script {
        ops: vec![
            Op::CreateElement {
                id: NodeId(1),
                tag: Tag::Div,
            },
            Op::CreateElement {
                id: NodeId(1),
                tag: Tag::Span,
            },
        ],
        ..Script::default()
    };
    assert_eq!(run(&reused).unwrap_err().kind, NodeIdReused(NodeId(1)));
}

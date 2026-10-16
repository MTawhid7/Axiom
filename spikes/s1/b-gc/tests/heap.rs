//! Behavioural tests of the Pattern B heap, using only its public API.
#![forbid(unsafe_code)]

use s1_b_gc::{Forwarding, Handle, Marker, Pacing, Roots, Trace, WeakHandle, heap};
use s1_scenarios::Rng;
use s1_scenarios::counters::{Live, Nodes, live};

struct Node {
    id: u32,
    edges: Vec<Handle<'static, Node>>,
    weak: Option<WeakHandle<Node>>,
    _live: Live<Nodes>,
}

heap! {
    struct Heap {
        nodes: Node,
    }
}

impl Trace for Node {
    fn mark(&self, marker: &mut Marker) {
        for &edge in &self.edges {
            marker.mark(edge);
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        for edge in &mut self.edges {
            forwarding.relocate(edge);
        }
        forwarding.relocate_weak(&mut self.weak);
    }
}

#[derive(Default)]
struct Globals {
    roots: Vec<Handle<'static, Node>>,
}

impl Roots for Globals {
    fn mark(&self, marker: &mut Marker) {
        for &root in &self.roots {
            marker.mark(root);
        }
    }

    fn relocate(&mut self, forwarding: &Forwarding) {
        for root in &mut self.roots {
            forwarding.relocate(root);
        }
    }
}

fn node(heap: &mut Heap, id: u32) -> Handle<'static, Node> {
    heap.alloc(Node {
        id,
        edges: Vec::new(),
        weak: None,
        _live: Live::new(),
    })
}

#[test]
fn unreachable_cycles_are_collected_and_dropped() {
    let base = live::<Nodes>();
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();
    let (a, b, c) = (node(&mut heap, 0), node(&mut heap, 1), node(&mut heap, 2));
    heap.nodes[a].edges.push(b);
    heap.nodes[b].edges.push(a); // a ⇄ b, rooted
    heap.nodes[c].edges.push(c); // a self-loop, unrooted
    globals.roots.push(a);

    heap.collect(&mut globals);
    assert_eq!(live::<Nodes>() - base, 2, "c is unreachable");
    assert_eq!(heap.nodes.len(), 2);

    globals.roots.clear();
    heap.collect(&mut globals);
    assert_eq!(live::<Nodes>(), base);
    assert!(heap.nodes.is_empty());
}

#[test]
fn compaction_moves_objects_and_relocates_every_handle() {
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();
    // Garbage interleaved with a chain, so compaction moves every survivor.
    let mut chain = Vec::new();
    for id in 0..10 {
        node(&mut heap, 1000 + id);
        chain.push(node(&mut heap, id));
    }
    for pair in chain.windows(2) {
        heap.nodes[pair[0]].edges.push(pair[1]);
    }
    globals.roots.push(chain[0]);

    for _ in 0..3 {
        heap.collect(&mut globals);
    }
    assert_eq!(heap.nodes.len(), 10);
    let mut ids = Vec::new();
    let mut cursor = Some(globals.roots[0]);
    while let Some(n) = cursor {
        ids.push(heap.nodes[n].id);
        cursor = heap.nodes[n].edges.first().copied();
    }
    assert_eq!(ids, (0..10).collect::<Vec<_>>());
}

#[test]
fn weak_handles_are_cleared_when_the_target_dies_and_relocated_otherwise() {
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();
    let (a, b, c) = (node(&mut heap, 0), node(&mut heap, 1), node(&mut heap, 2));
    heap.nodes[a].weak = Some(b.downgrade()); // b is unrooted: it will die
    heap.nodes[c].weak = Some(a.downgrade()); // a is rooted: it will survive
    globals.roots.extend([a, c]);

    heap.collect(&mut globals);
    let [a, c] = [globals.roots[0], globals.roots[1]];
    assert!(heap.nodes[a].weak.is_none(), "dead target: cleared");
    let target = heap.nodes[c].weak.expect("live target: kept").upgrade();
    assert_eq!(target, a, "and relocated");
    assert_eq!(heap.nodes.len(), 2);
}

#[test]
fn scoped_roots_survive_collection_and_are_released_with_their_frame() {
    let base = live::<Nodes>();
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();
    node(&mut heap, 99); // garbage in front, so the survivor moves
    let kept = node(&mut heap, 7);
    {
        let frame = heap.core.frame();
        let rooted = kept.scope(&frame);
        assert_eq!(heap.core.rooted(), 1);
        heap.collect(&mut globals);
        let relocated = rooted.get();
        assert_ne!(relocated.index(), kept.index(), "compaction moved it");
        assert_eq!(heap.nodes[relocated].id, 7);
    }
    assert_eq!(heap.core.rooted(), 0, "dropping the frame popped the root");
    heap.collect(&mut globals);
    assert_eq!(live::<Nodes>(), base);
}

/// Ground truth for the randomized test: ids, strong edges, weak edges, roots.
#[derive(Default)]
struct Model {
    edges: Vec<Vec<u32>>,
    weak: Vec<Option<u32>>,
    roots: Vec<u32>,
}

impl Model {
    fn reachable(&self) -> Vec<bool> {
        let mut seen = vec![false; self.edges.len()];
        let mut stack = self.roots.clone();
        while let Some(id) = stack.pop() {
            if !std::mem::replace(&mut seen[id as usize], true) {
                stack.extend(&self.edges[id as usize]);
            }
        }
        seen
    }
}

/// The Pattern A oracle test, transposed. It also checks that compaction
/// rewrites every handle correctly: edges are verified by id after objects
/// have moved.
#[test]
fn random_graphs_match_a_reachability_oracle() {
    let new_per_round: u32 = if cfg!(miri) { 12 } else { 400 };
    let rounds = if cfg!(miri) { 3 } else { 15 };
    let base = live::<Nodes>();
    let mut rng = Rng::seed_from_u64(0x00B0_B0B0);
    let mut model = Model::default();
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();

    for _ in 0..rounds {
        let mut by_id: Vec<Option<Handle<'static, Node>>> = vec![None; model.edges.len()];
        let mut stack = globals.roots.clone();
        while let Some(n) = stack.pop() {
            if by_id[heap.nodes[n].id as usize].replace(n).is_none() {
                stack.extend(heap.nodes[n].edges.iter().copied());
            }
        }
        let first_new = u32::try_from(model.edges.len()).expect("small");
        for id in first_new..first_new + new_per_round {
            by_id.push(Some(node(&mut heap, id)));
            model.edges.push(Vec::new());
            model.weak.push(None);
        }
        let candidates: Vec<u32> = (0..u32::try_from(by_id.len()).expect("small"))
            .filter(|&id| by_id[id as usize].is_some())
            .collect();
        let handle = |id: u32| by_id[id as usize].expect("candidate");

        for _ in 0..new_per_round * 2 {
            let from = if rng.ratio(3, 4) {
                first_new + rng.below(new_per_round)
            } else {
                *rng.pick(&candidates)
            };
            let to = *rng.pick(&candidates);
            heap.nodes[handle(from)].edges.push(handle(to));
            model.edges[from as usize].push(to);
        }
        for _ in 0..new_per_round / 3 {
            let from = first_new + rng.below(new_per_round);
            let to = *rng.pick(&candidates);
            heap.nodes[handle(from)].weak = Some(handle(to).downgrade());
            model.weak[from as usize] = Some(to);
        }
        globals.roots.clear();
        model.roots.clear();
        for &id in &candidates {
            if rng.ratio(1, 12) {
                globals.roots.push(handle(id));
                model.roots.push(id);
            }
        }

        heap.collect(&mut globals);

        let reachable = model.reachable();
        let reachable_count = reachable.iter().filter(|&&r| r).count();
        assert_eq!(live::<Nodes>() - base, reachable_count, "live values");
        assert_eq!(heap.nodes.len(), reachable_count, "compacted arena");

        let mut seen = vec![false; model.edges.len()];
        let mut stack = globals.roots.clone();
        while let Some(n) = stack.pop() {
            let id = heap.nodes[n].id as usize;
            if std::mem::replace(&mut seen[id], true) {
                continue;
            }
            assert!(reachable[id]);
            let edges: Vec<u32> = heap.nodes[n]
                .edges
                .iter()
                .map(|&e| heap.nodes[e].id)
                .collect();
            assert_eq!(edges, model.edges[id], "edges of node {id}");
            let weak_target = heap.nodes[n].weak.map(|w| heap.nodes[w.upgrade()].id);
            let expected = model.weak[id].filter(|&t| reachable[t as usize]);
            assert_eq!(weak_target, expected, "weak edge of node {id}");
            stack.extend(heap.nodes[n].edges.iter().copied());
        }
        assert_eq!(seen, reachable, "traversal reaches exactly the model's set");
    }
}

#[test]
fn pacing_reports_when_the_budget_is_spent() {
    let mut heap = Heap::new(Pacing {
        min_bytes: 4096,
        growth_percent: 100,
    });
    let mut globals = Globals::default();
    assert!(!heap.core.wants_collection());
    let mut garbage = 0;
    while !heap.core.wants_collection() {
        node(&mut heap, 0);
        garbage += 1;
    }
    assert!(garbage > 0);
    heap.collect(&mut globals);
    assert!(
        !heap.core.wants_collection(),
        "the budget resets after a collection"
    );
    assert_eq!(heap.core.metrics().collections, 1);
}

#[test]
fn deep_chains_neither_overflow_the_stack_nor_leak() {
    let depth: u32 = if cfg!(miri) { 200 } else { 200_000 };
    let base = live::<Nodes>();
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();
    let first = node(&mut heap, 0);
    let mut tail = first;
    for id in 1..depth {
        let next = node(&mut heap, id);
        heap.nodes[tail].edges.push(next);
        tail = next;
    }
    globals.roots.push(first);
    heap.collect(&mut globals); // marking uses a worklist
    assert_eq!(live::<Nodes>() - base, depth as usize);
    drop(heap);
    assert_eq!(live::<Nodes>(), base);
}

/// Debug builds turn stale-handle use into a deterministic panic, where a
/// release build would silently read whatever object now occupies the slot.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "stale handle")]
fn using_a_handle_after_a_collection_panics_in_debug_builds() {
    let mut heap = Heap::new(Pacing::default());
    let mut globals = Globals::default();
    let held = node(&mut heap, 1); // an unbound handle, held across a collection
    globals.roots.push(held);
    heap.collect(&mut globals);
    let _ = heap.nodes[held].id; // not re-read from the roots: stale
}

mod buggy {
    use super::{Forwarding, Handle, Live, Marker, Nodes, Pacing, Roots, Trace, heap};

    /// A `Trace` impl that forgets to mark its edge.
    pub(super) struct Leaky {
        pub(super) edge: Option<Handle<'static, Leaky>>,
        pub(super) _live: Live<Nodes>,
    }

    heap! {
        pub(super) struct LeakyHeap {
            nodes: Leaky,
        }
    }

    impl Trace for Leaky {
        fn mark(&self, _: &mut Marker) {} // bug: `edge` is not marked
        fn relocate(&mut self, forwarding: &Forwarding) {
            forwarding.relocate_opt(&mut self.edge);
        }
    }

    pub(super) struct Root(pub(super) Handle<'static, Leaky>);

    impl Roots for Root {
        fn mark(&self, marker: &mut Marker) {
            marker.mark(self.0);
        }
        fn relocate(&mut self, forwarding: &Forwarding) {
            forwarding.relocate(&mut self.0);
        }
    }

    pub(super) fn heap() -> LeakyHeap {
        LeakyHeap::new(Pacing::default())
    }
}

/// A missed mark is a *logic* error in Pattern B. Debug builds catch it the
/// moment the collector tries to relocate a handle to the dead object.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "some Trace::mark missed it")]
fn a_missed_mark_is_caught_in_debug_builds() {
    let mut heap = buggy::heap();
    let child = heap.alloc(buggy::Leaky {
        edge: None,
        _live: Live::new(),
    });
    let parent = heap.alloc(buggy::Leaky {
        edge: Some(child),
        _live: Live::new(),
    });
    heap.collect(&mut buggy::Root(parent));
}

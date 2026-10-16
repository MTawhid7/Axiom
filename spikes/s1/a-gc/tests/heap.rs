//! Behavioural tests of the Pattern A heap, using only its safe public API.
//!
//! Sizes shrink under Miri, which runs every test here with the unsafe core
//! checked for undefined behaviour.
#![forbid(unsafe_code)]

use s1_a_gc::{Arena, Gc, GcWeak, Lock, Mutation, Pacing, RefLock, Rootable, Trace, unlock};
use s1_scenarios::Rng;
use s1_scenarios::counters::{Live, Nodes, live};

#[derive(Trace)]
struct Node<'gc> {
    id: u32,
    edges: RefLock<Vec<Gc<'gc, Node<'gc>>>>,
    weak: Lock<Option<GcWeak<'gc, Node<'gc>>>>,
    #[trace(require_static)]
    live: Live<Nodes>,
}

fn node<'gc>(mc: &Mutation<'gc>, id: u32) -> Gc<'gc, Node<'gc>> {
    Gc::new(
        mc,
        Node {
            id,
            edges: RefLock::new(Vec::new()),
            weak: Lock::new(None),
            live: Live::new(),
        },
    )
}

fn link<'gc>(mc: &Mutation<'gc>, from: Gc<'gc, Node<'gc>>, to: Gc<'gc, Node<'gc>>) {
    unlock!(Gc::write(mc, from), Node, edges)
        .borrow_mut()
        .push(to);
}

fn set_weak<'gc>(mc: &Mutation<'gc>, from: Gc<'gc, Node<'gc>>, to: Option<Gc<'gc, Node<'gc>>>) {
    unlock!(Gc::write(mc, from), Node, weak).set(to.map(Gc::downgrade));
}

struct Roots;
impl Rootable for Roots {
    type Root<'gc> = Vec<Gc<'gc, Node<'gc>>>;
}

#[test]
fn unreachable_cycles_are_collected_and_dropped() {
    let base = live::<Nodes>();
    let mut arena = Arena::<Roots>::new(|_| Vec::new());
    arena.mutate_root(|mc, roots| {
        let (a, b, c) = (node(mc, 0), node(mc, 1), node(mc, 2));
        link(mc, a, b);
        link(mc, b, a); // a ⇄ b, rooted
        link(mc, c, c); // a self-loop, unrooted
        roots.push(a);
    });
    assert_eq!(live::<Nodes>() - base, 3);

    arena.collect_all();
    assert_eq!(live::<Nodes>() - base, 2, "c is unreachable");
    assert_eq!(arena.metrics().objects, 2);

    arena.mutate_root(|_, roots| roots.clear());
    arena.collect_all();
    assert_eq!(live::<Nodes>(), base, "the a ⇄ b cycle is unreachable");
    assert_eq!(arena.metrics().objects, 0);
}

#[test]
fn reachable_data_survives_collection_intact() {
    let mut arena = Arena::<Roots>::new(|_| Vec::new());
    arena.mutate_root(|mc, roots| {
        let nodes: Vec<_> = (0..10).map(|i| node(mc, i)).collect();
        for pair in nodes.windows(2) {
            link(mc, pair[0], pair[1]);
        }
        roots.push(nodes[0]);
    });
    for _ in 0..3 {
        arena.collect_all();
    }
    arena.mutate(|_, roots| {
        let mut ids = Vec::new();
        let mut cursor = Some(roots[0]);
        while let Some(n) = cursor {
            ids.push(n.id);
            cursor = n.edges.borrow().first().copied();
        }
        assert_eq!(ids, (0..10).collect::<Vec<_>>());
    });
}

#[test]
fn a_dead_weak_target_keeps_its_memory_until_the_weak_reference_goes() {
    let base = live::<Nodes>();
    let mut arena = Arena::<Roots>::new(|_| Vec::new());
    arena.mutate_root(|mc, roots| {
        let (a, b) = (node(mc, 0), node(mc, 1));
        set_weak(mc, a, Some(b)); // only `a` is rooted
        roots.push(a);
    });
    arena.collect_all();
    assert_eq!(live::<Nodes>() - base, 1, "b's value was dropped");
    assert_eq!(
        arena.metrics().objects,
        2,
        "b's box is kept for the weak reference"
    );

    arena.mutate(|mc, roots| {
        let weak = roots[0].weak.get().expect("weak set");
        assert!(weak.is_dead());
        assert!(weak.upgrade(mc).is_none());
        set_weak(mc, roots[0], None);
    });
    arena.collect_all();
    assert_eq!(
        arena.metrics().objects,
        1,
        "no weak reference remains: memory freed"
    );
}

#[test]
fn a_weak_reference_to_a_reachable_object_upgrades() {
    let mut arena = Arena::<Roots>::new(|_| Vec::new());
    arena.mutate_root(|mc, roots| {
        let (a, b) = (node(mc, 0), node(mc, 1));
        set_weak(mc, a, Some(b));
        roots.extend([a, b]);
    });
    arena.collect_all();
    arena.mutate(|mc, roots| {
        let upgraded = roots[0]
            .weak
            .get()
            .and_then(|w| w.upgrade(mc))
            .expect("alive");
        assert!(Gc::ptr_eq(upgraded, roots[1]));
    });
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
            let i = id as usize;
            if !std::mem::replace(&mut seen[i], true) {
                stack.extend(&self.edges[i]);
            }
        }
        seen
    }
}

/// Repeatedly grows a random graph that is wired into the surviving nodes,
/// collects, and checks the heap against `Model`:
/// - live values equal the reachable set;
/// - live boxes equal the reachable set plus the weak targets it keeps;
/// - every surviving edge is intact;
/// - every weak reference upgrades exactly when its target is reachable.
#[test]
fn random_graphs_match_a_reachability_oracle() {
    let new_per_round: u32 = if cfg!(miri) { 12 } else { 400 };
    let rounds = if cfg!(miri) { 3 } else { 15 };
    let base = live::<Nodes>();
    let mut rng = Rng::seed_from_u64(0x00A1_1CE5);
    let mut model = Model::default();
    let mut arena = Arena::<Roots>::new(|_| Vec::new());

    for _ in 0..rounds {
        arena.mutate_root(|mc, roots| {
            // Handles to every reachable node, indexed by id.
            let mut by_id: Vec<Option<Gc<'_, Node<'_>>>> = vec![None; model.edges.len()];
            let mut stack: Vec<_> = roots.clone();
            while let Some(n) = stack.pop() {
                if by_id[n.id as usize].replace(n).is_none() {
                    stack.extend(n.edges.borrow().iter().copied());
                }
            }
            let first_new = u32::try_from(model.edges.len()).expect("small");
            for id in first_new..first_new + new_per_round {
                by_id.push(Some(node(mc, id)));
                model.edges.push(Vec::new());
                model.weak.push(None);
            }
            let candidates: Vec<u32> = (0..u32::try_from(by_id.len()).expect("small"))
                .filter(|&id| by_id[id as usize].is_some())
                .collect();

            // Random strong edges out of new nodes, plus some from old ones.
            for _ in 0..new_per_round * 2 {
                let from = if rng.ratio(3, 4) {
                    first_new + rng.below(new_per_round)
                } else {
                    *rng.pick(&candidates)
                };
                let to = *rng.pick(&candidates);
                link(
                    mc,
                    by_id[from as usize].unwrap(),
                    by_id[to as usize].unwrap(),
                );
                model.edges[from as usize].push(to);
            }
            // Random weak edges out of new nodes.
            for _ in 0..new_per_round / 3 {
                let from = first_new + rng.below(new_per_round);
                let to = *rng.pick(&candidates);
                set_weak(mc, by_id[from as usize].unwrap(), by_id[to as usize]);
                model.weak[from as usize] = Some(to);
            }
            // A fresh random root set.
            roots.clear();
            model.roots.clear();
            for &id in &candidates {
                if rng.ratio(1, 12) {
                    roots.push(by_id[id as usize].unwrap());
                    model.roots.push(id);
                }
            }
        });

        arena.collect_all();

        let reachable = model.reachable();
        let reachable_count = reachable.iter().filter(|&&r| r).count();
        let mut kept = reachable.clone();
        for (id, &weak) in model.weak.iter().enumerate() {
            if let (true, Some(target)) = (reachable[id], weak) {
                kept[target as usize] = true;
            }
        }
        assert_eq!(live::<Nodes>() - base, reachable_count, "live values");
        assert_eq!(
            arena.metrics().objects,
            kept.iter().filter(|&&k| k).count(),
            "live boxes"
        );

        arena.mutate(|mc, roots| {
            let mut stack: Vec<_> = roots.clone();
            let mut seen = vec![false; model.edges.len()];
            while let Some(n) = stack.pop() {
                let id = n.id as usize;
                if std::mem::replace(&mut seen[id], true) {
                    continue;
                }
                assert!(reachable[id]);
                let edges: Vec<u32> = n.edges.borrow().iter().map(|e| e.id).collect();
                assert_eq!(edges, model.edges[id], "edges of node {id}");
                let weak_alive = n.weak.get().map(|w| w.upgrade(mc).is_some());
                assert_eq!(
                    weak_alive,
                    model.weak[id].map(|t| reachable[t as usize]),
                    "weak edge of node {id}"
                );
                stack.extend(n.edges.borrow().iter().copied());
            }
            assert_eq!(seen, reachable, "traversal reaches exactly the model's set");
        });
    }
}

#[test]
fn pacing_triggers_a_collection_once_the_budget_is_spent() {
    let pacing = Pacing {
        min_bytes: 4096,
        growth_percent: 100,
    };
    let mut arena = Arena::<Roots>::with_pacing(pacing, |_| Vec::new());
    assert!(!arena.collect_debt(), "nothing allocated yet");

    let mut garbage = 0;
    while !arena.mutate(|mc, _| {
        node(mc, 0);
        mc.wants_collection()
    }) {
        garbage += 1;
    }
    assert!(garbage > 0);
    assert!(arena.collect_debt());
    let metrics = arena.metrics();
    assert_eq!(metrics.collections, 1);
    assert_eq!(metrics.objects, 0);
    assert!(
        !arena.collect_debt(),
        "the budget resets after a collection"
    );
}

#[test]
fn barrier_conveniences_write_and_are_counted() {
    struct Pair;
    impl Rootable for Pair {
        type Root<'gc> = (Gc<'gc, Lock<u32>>, Gc<'gc, RefLock<Vec<u32>>>);
    }
    let arena = Arena::<Pair>::new(|mc| {
        (
            Gc::new(mc, Lock::new(1)),
            Gc::new(mc, RefLock::new(Vec::new())),
        )
    });
    arena.mutate(|mc, (counter, list)| {
        counter.set(mc, counter.get() + 1);
        list.borrow_mut(mc).push(7);
        assert_eq!(counter.get(), 2);
        assert_eq!(*list.borrow(), [7]);
    });
    assert_eq!(arena.metrics().barriers, 2);
}

#[test]
fn deep_chains_neither_overflow_the_stack_nor_leak() {
    let depth: u32 = if cfg!(miri) { 200 } else { 200_000 };
    let base = live::<Nodes>();
    let mut arena = Arena::<Roots>::new(|_| Vec::new());
    arena.mutate_root(|mc, roots| {
        let head = node(mc, 0);
        let mut tail = head;
        for id in 1..depth {
            let next = node(mc, id);
            link(mc, tail, next);
            tail = next;
        }
        roots.push(head);
    });
    arena.collect_all(); // marking is iterative
    assert_eq!(live::<Nodes>() - base, depth as usize);
    drop(arena); // and so is freeing
    assert_eq!(live::<Nodes>(), base);
}

#[test]
fn dropping_the_arena_drops_every_value() {
    let base = live::<Nodes>();
    let arena = Arena::<Roots>::new(|mc| {
        let a = node(mc, 0);
        link(mc, a, a);
        node(mc, 1); // garbage, never collected
        vec![a]
    });
    assert_eq!(live::<Nodes>() - base, 2);
    drop(arena);
    assert_eq!(live::<Nodes>(), base);
}

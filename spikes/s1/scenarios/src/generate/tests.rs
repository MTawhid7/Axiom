use super::*;
use crate::reference;
use crate::script::Op;

const SEEDS: u64 = 100;

#[test]
fn every_small_workload_is_valid_and_reproduces_its_expectation_for_100_seeds() {
    for workload in Workload::ALL {
        for seed in 0..SEEDS {
            let g = workload.generate(Size::Small, seed);
            let (outcome, stats) = reference::run(&g.script)
                .unwrap_or_else(|e| panic!("{} seed {seed}: {e}", workload.name()));
            assert_eq!(outcome, g.expected, "{} seed {seed}", workload.name());
            assert_eq!(stats, g.stats, "{} seed {seed}", workload.name());
            assert!(!outcome.checkpoints.is_empty());
        }
    }
}

#[test]
fn generation_is_deterministic_and_seed_sensitive() {
    for workload in Workload::ALL {
        let a = workload.generate(Size::Small, 7);
        let b = workload.generate(Size::Small, 7);
        let c = workload.generate(Size::Small, 8);
        assert_eq!(a.script, b.script, "{}", workload.name());
        assert_eq!(
            a.expected.digest(),
            b.expected.digest(),
            "{}",
            workload.name()
        );
        assert_ne!(a.script, c.script, "{}", workload.name());
    }
}

#[test]
fn medium_workloads_are_valid() {
    for workload in Workload::ALL {
        let g = workload.generate(Size::Medium, 1);
        let (outcome, _) = reference::run(&g.script).expect("valid script");
        assert_eq!(outcome, g.expected, "{}", workload.name());
    }
}

/// The workloads must actually exercise the semantics that tell a correct
/// implementation from an incorrect one. Otherwise "outcome equality" would be
/// a weak oracle.
#[test]
fn small_workloads_cover_the_distinguishing_semantics() {
    let mut invocations = 0;
    let mut stopped = 0;
    let mut skipped_removed = 0;
    let mut phases_seen = [false; 3];
    let mut weak_alive_seen = false;
    let mut weak_dead_seen = false;
    let mut cycles_collected = false;
    for workload in [Workload::Listeners, Workload::Churn] {
        for seed in 0..SEEDS {
            let g = workload.generate(Size::Small, seed);
            invocations += g.stats.invocations;
            stopped += g.stats.stopped_dispatches;
            skipped_removed += g.stats.skipped_removed;
            for entry in &g.expected.log {
                phases_seen[entry.phase as usize - 1] = true;
            }
            for checkpoint in &g.expected.checkpoints {
                weak_alive_seen |= checkpoint.weak_alive.contains(&Some(true));
                weak_dead_seen |= checkpoint.weak_alive.contains(&Some(false));
            }
            // Non-garbage nodes that died: unreachable structures (often
            // cycles) that a correct collector must reclaim.
            let last = g
                .expected
                .checkpoints
                .last()
                .expect("ends with a checkpoint");
            let materialized = g.stats.nodes_allocated - g.stats.garbage_allocated;
            cycles_collected |= (last.live_nodes as u64) < materialized;
        }
    }
    assert!(invocations > 1_000, "only {invocations} invocations");
    assert!(stopped > 0, "stopPropagation never cut a dispatch short");
    assert!(
        skipped_removed > 0,
        "the snapshot's removed flag was never exercised"
    );
    assert_eq!(phases_seen, [true; 3], "every event phase must occur");
    assert!(
        weak_alive_seen && weak_dead_seen,
        "weak references must both survive and die"
    );
    assert!(
        cycles_collected,
        "some materialized nodes must become garbage"
    );
}

#[test]
fn the_tree_workload_keeps_every_node_alive_through_the_document() {
    for seed in 0..SEEDS {
        let g = Workload::Tree.generate(Size::Small, seed);
        let last = g.expected.checkpoints.last().expect("one checkpoint");
        assert_eq!(
            last.live_nodes as u64, g.stats.nodes_allocated,
            "seed {seed}"
        );
        assert_eq!(last.live_closures, 0);
    }
}

#[test]
fn the_tree_shape_matches_its_parameters() {
    let params = TreeParams::preset(Size::Medium);
    let g = tree(&params, 3);
    let elements = g
        .script
        .ops
        .iter()
        .filter(|op| matches!(op, Op::CreateElement { .. }))
        .count();
    assert_eq!(elements, params.elements as usize);

    // In this workload each node is appended exactly once, after its parent.
    let mut depth = vec![0_u32; g.script.node_slots as usize];
    let mut max_depth = 0;
    for op in &g.script.ops {
        if let Op::AppendChild { parent, child } = op {
            depth[child.index()] = depth[parent.index()] + 1;
            max_depth = max_depth.max(depth[child.index()]);
        }
    }
    // Elements sit at most `max_depth + 1` below the document; text adds one level.
    assert!(max_depth <= params.max_depth + 2, "too deep: {max_depth}");
    assert!(max_depth >= 6, "unrealistically shallow: {max_depth}");
}

#[test]
fn pressure_keeps_the_reachable_set_constant_and_allocates_the_expected_garbage() {
    for size in [Size::Small, Size::Medium] {
        let params = PressureParams::preset(size);
        for seed in 0..10 {
            let g = pressure(&params, seed);
            let [before, after] = g.expected.checkpoints.as_slice() else {
                panic!("pressure takes exactly two checkpoints");
            };
            assert_eq!(
                before, after,
                "dispatching must not change what is reachable"
            );
            assert_eq!(g.stats.garbage_allocated, params.total_garbage());
            assert_eq!(
                g.stats.invocations,
                u64::from(params.dispatches * params.path_depth * params.listeners_per_node)
            );
        }
    }
}

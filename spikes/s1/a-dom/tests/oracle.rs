//! Pattern A must reproduce the reference interpreter's outcome exactly:
//! the listener log, tree digests, live node and closure counts after every
//! full collection, and weak-reference liveness.
#![forbid(unsafe_code)]

use s1_a_dom::dom::NODE_BOX_BYTES;
use s1_a_dom::{run, run_with};
use s1_a_gc::Pacing;
use s1_scenarios::Script;
use s1_scenarios::generate::{PressureParams, Size, Workload, pressure};

const SEEDS: u64 = if cfg!(miri) { 2 } else { 100 };

/// A budget of one byte: the heap asks for a collection after every
/// allocation. Collections then fall at almost every op and listener boundary,
/// which maximizes the number of collector/mutator interleavings.
const COLLECT_ALWAYS: Pacing = Pacing {
    min_bytes: 1,
    growth_percent: 0,
};

fn assert_matches(
    name: &str,
    script: &Script,
    expected: &s1_scenarios::Outcome,
    pacing: Pacing,
) -> s1_a_dom::RunReport {
    let report = run_with(script, pacing);
    if let Some(difference) = report.outcome.first_difference(expected) {
        panic!("{name}: {difference}");
    }
    report
}

#[test]
fn edge_cases_match_the_reference_at_both_pacings() {
    for (name, script) in s1_scenarios::edge_cases::all() {
        let (expected, _) = s1_scenarios::reference::run(&script).expect("valid edge case");
        assert_matches(name, &script, &expected, Pacing::default());
        assert_matches(
            &format!("{name}, collect-always"),
            &script,
            &expected,
            COLLECT_ALWAYS,
        );
    }
}

#[test]
fn small_workloads_match_the_reference() {
    for workload in Workload::ALL {
        for seed in 0..SEEDS {
            let g = workload.generate(Size::Small, seed);
            assert_matches(
                &format!("{} seed {seed}", workload.name()),
                &g.script,
                &g.expected,
                Pacing::default(),
            );
        }
    }
}

#[test]
fn small_workloads_match_the_reference_when_collecting_constantly() {
    let mut pressure_yields = 0;
    for workload in Workload::ALL {
        for seed in 0..SEEDS {
            let g = workload.generate(Size::Small, seed);
            let report = assert_matches(
                &format!("{} seed {seed}, collect-always", workload.name()),
                &g.script,
                &g.expected,
                COLLECT_ALWAYS,
            );
            if workload == Workload::Pressure {
                pressure_yields += report.dispatch_yields;
            }
        }
    }
    // So the resumable dispatch, and collection in the middle of one, are
    // exercised here, including under Miri.
    assert!(pressure_yields > 0, "pressure dispatches must yield");
}

#[test]
#[cfg_attr(
    miri,
    ignore = "minutes under Miri; the Small tests cover the same code paths"
)]
fn medium_workloads_match_the_reference() {
    for workload in Workload::ALL {
        for seed in 1..=3 {
            let g = workload.generate(Size::Medium, seed);
            assert_matches(
                &format!("{} medium seed {seed}", workload.name()),
                &g.script,
                &g.expected,
                Pacing::default(),
            );
        }
    }
}

/// The pressure bound from the Step 2b plan (§8, CP2): with collections
/// *during* a dispatch, peak live nodes ≤ L + B + s.
/// - L: the live nodes at the checkpoints;
/// - B: the allocation budget, expressed in node boxes;
/// - s: one listener's garbage.
#[test]
#[cfg_attr(
    miri,
    ignore = "131k allocations; the Small tests cover the yield path under Miri"
)]
fn pressure_peak_stays_within_the_collection_bound() {
    let params = PressureParams::preset(Size::Medium);
    let g = pressure(&params, 1);
    let pacing = Pacing::default();
    let report = run(&g.script);
    assert_eq!(report.outcome, g.expected);

    let live = g.expected.checkpoints[0].live_nodes;
    let budget_nodes = pacing.budget(report.metrics.bytes_after_collection) / NODE_BOX_BYTES;
    let step = usize::from(params.garbage_per_invocation);
    let bound = live + budget_nodes + step + 1;
    let without_mid_dispatch_gc = live + usize::try_from(params.total_garbage()).expect("fits");

    assert!(
        report.dispatch_yields > 0,
        "dispatches must yield to the collector"
    );
    assert!(
        report.peak_live_nodes <= bound,
        "peak {} exceeds bound {bound} (L = {live}, B = {budget_nodes} nodes, s = {step})",
        report.peak_live_nodes
    );
    assert!(
        bound * 10 < without_mid_dispatch_gc,
        "the workload must separate the regimes: bound {bound} vs {without_mid_dispatch_gc}"
    );
    eprintln!(
        "pressure (Pattern A): peak {} ≤ bound {bound}; without mid-dispatch GC ≈ {without_mid_dispatch_gc}; \
         {} yields, {} collections, max pause {:?}",
        report.peak_live_nodes,
        report.dispatch_yields,
        report.metrics.collections,
        report.metrics.max_pause,
    );
}

//! Pattern B must reproduce the reference interpreter's outcome exactly, and
//! agree with Pattern A.
#![forbid(unsafe_code)]

use s1_b_dom::dom::NODE_BYTES;
use s1_b_dom::{RunReport, run, run_with};
use s1_b_gc::Pacing;
use s1_scenarios::generate::{PressureParams, Size, Workload, pressure};
use s1_scenarios::{Outcome, Script};

const SEEDS: u64 = if cfg!(miri) { 2 } else { 100 };

/// A budget of one byte: every allocation safepoint collects.
const COLLECT_ALWAYS: Pacing = Pacing {
    min_bytes: 1,
    growth_percent: 0,
};

fn assert_matches(name: &str, script: &Script, expected: &Outcome, pacing: Pacing) -> RunReport {
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
    let mut pressure_collections = 0;
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
                pressure_collections += report.metrics.collections;
            }
        }
    }
    // Collections inside dispatches (at allocation safepoints) are exercised
    // here, including under Miri.
    assert!(pressure_collections > 0);
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

/// The pressure bound (Step 2b plan §8, CP2). Pattern B reaches a safepoint at
/// every allocation, so the overshoot past the budget is one node, not one
/// listener's garbage: peak ≤ L + B + 1.
#[test]
#[cfg_attr(
    miri,
    ignore = "131k allocations; the Small tests cover the safepoint path under Miri"
)]
fn pressure_peak_stays_within_the_collection_bound() {
    let params = PressureParams::preset(Size::Medium);
    let g = pressure(&params, 1);
    let pacing = Pacing::default();
    let report = run(&g.script);
    assert_eq!(report.outcome, g.expected);

    let live = g.expected.checkpoints[0].live_nodes;
    let budget_nodes = pacing.budget(report.metrics.bytes_after_collection) / NODE_BYTES;
    let bound = live + budget_nodes + 1;
    let without_mid_dispatch_gc = live + usize::try_from(params.total_garbage()).expect("fits");
    assert!(
        report.peak_live_nodes <= bound,
        "peak {} exceeds bound {bound} (L = {live}, B = {budget_nodes} nodes)",
        report.peak_live_nodes
    );
    assert!(bound * 10 < without_mid_dispatch_gc);
    eprintln!(
        "pressure (Pattern B): peak {} ≤ bound {bound}; without mid-dispatch GC ≈ {without_mid_dispatch_gc}; \
         {} collections, max pause {:?}",
        report.peak_live_nodes, report.metrics.collections, report.metrics.max_pause,
    );
}

/// A ≡ B, directly. It is already implied by both matching the reference, but
/// asserting it here makes the differential claim explicit, including on the
/// Medium scripts.
#[test]
#[cfg_attr(miri, ignore = "runs both patterns at Medium size")]
fn pattern_a_and_pattern_b_agree() {
    for workload in Workload::ALL {
        for size in [Size::Small, Size::Medium] {
            let g = workload.generate(size, 7);
            let a = s1_a_dom::run(&g.script).outcome;
            let b = run(&g.script).outcome;
            assert_eq!(a, b, "{} {size:?}", workload.name());
        }
    }
}

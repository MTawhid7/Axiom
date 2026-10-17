//! The quick pass of every suite. Each case verifies both patterns against
//! the reference (and against each other) before it times anything, so this
//! keeps the benchmarks honest under `cargo test`. In debug builds, Pattern
//! B's epoch checks also guard the benchmark glue against stale handles.
#![forbid(unsafe_code)]

use s1_bench::suites::{self, Options, SUITES};
use s1_bench::{Dom, Pattern, PatternA, PatternB};

#[test]
fn every_suite_verifies_and_reports_both_patterns() {
    let mut pairs = Vec::new();
    suites::run(
        &Options {
            quick: true,
            filter: None,
        },
        &mut |pair| pairs.push(pair),
    );
    for suite in SUITES {
        assert!(
            pairs.iter().any(|[a, _]| a.suite == suite),
            "suite {suite} reported nothing"
        );
    }
    for [a, b] in &pairs {
        assert_eq!((a.pattern, b.pattern), (Pattern::A, Pattern::B));
        assert_eq!((&a.suite, &a.case), (&b.suite, &b.case));
        assert_eq!(
            a.items, b.items,
            "{}/{}: both did the same work",
            a.suite, a.case
        );
        assert!(a.items > 0, "{}/{} measured no work", a.suite, a.case);
    }
}

#[test]
fn the_filter_selects_cases_by_suite_and_name() {
    let mut cases = Vec::new();
    suites::run(
        &Options {
            quick: true,
            filter: Some("collect/live".to_owned()),
        },
        &mut |[a, _]| cases.push(format!("{}/{}", a.suite, a.case)),
    );
    assert_eq!(cases, ["collect/live-200"]);
}

#[test]
fn profile_loops_reject_unknown_cases() {
    assert!(suites::profile::<PatternA>("nonsense", 0.0).is_err());
    assert!(suites::profile::<PatternB>("scenario-nonsense", 0.0).is_err());
    assert_eq!(PatternA::PATTERN, Pattern::A);
}

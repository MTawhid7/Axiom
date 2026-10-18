//! The quick pass of every suite: small inputs, one sample, and every
//! verification the full runs make.
#![forbid(unsafe_code)]

use s3_bench::suites::{self, Options};

#[test]
fn every_suite_verifies_and_reports() {
    let mut cases = Vec::new();
    suites::run(
        &Options {
            quick: true,
            filter: None,
        },
        &mut |row: &[s3_bench::harness::Measurement]| {
            assert!(!row.is_empty());
            for m in row {
                assert!(m.items > 0 && m.samples == 1, "{m:?}");
            }
            cases.push(format!("{}/{}", row[0].suite, row[0].case));
        },
    );
    for suite in suites::SUITES {
        assert!(
            cases.iter().any(|c| c.starts_with(suite)),
            "{suite} reported nothing: {cases:?}"
        );
    }
}

#[test]
fn counts_cover_every_workload() {
    let rows = suites::counts(s3_vm::workloads::Size::Test);
    assert_eq!(rows.len(), 4 * s3_vm::Technique::ALL.len());
}

#[test]
fn the_profile_loops_run() {
    assert!(suites::profile("w1/match/raw", 0.0).is_ok());
    assert!(suites::profile("nonsense", 0.0).is_err());
}

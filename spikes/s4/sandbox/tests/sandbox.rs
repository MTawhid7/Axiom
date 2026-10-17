//! S4-Q1 and S4-Q2 as tests: the broker launches the real helper binary,
//! which applies the renderer profile (or, for the control run, none) and
//! probes what it can do.
//!
//! These binaries are signed only by the linker (ad hoc, without the hardened
//! runtime). `tools/s4-sign.sh` runs the same scenarios against copies signed
//! with `codesign -s - -o runtime`, which is the configuration S4-Q1 names.
#![forbid(unsafe_code)]

use std::path::Path;

use s4_sandbox::platform::process::Exit;
use s4_sandbox::probe::Outcome;
use s4_sandbox::profile::{Method, Profile};
use s4_sandbox::scenario::{self, ProbeReport, ProbeRun};

const HELPER: &str = env!("CARGO_BIN_EXE_s4-helper");

fn run(profile: Option<(Profile, Method)>) -> ProbeReport {
    let work_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("s4");
    let report = scenario::run_probe(&ProbeRun {
        helper: Path::new(HELPER),
        profile,
        work_dir: &work_dir,
    })
    .expect("the scenario runs");
    let mismatches = report.mismatches();
    assert!(
        mismatches.is_empty(),
        "unexpected results:\n{}\nfull report: {report:#?}",
        mismatches
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    report
}

/// Without a profile, every probe succeeds, so each probe is able to observe
/// the access it tests.
#[test]
fn the_control_run_allows_every_probe() {
    let report = run(None);
    assert_eq!(report.state("sandboxed"), Some("false"));
    assert!(report.results.iter().all(|r| r.outcome == Outcome::Allowed));
}

#[test]
fn the_renderer_profile_applies_with_sandbox_init_with_parameters() {
    let report = run(Some((Profile::Renderer, Method::InitWithParameters)));
    assert_eq!(report.state("sandboxed"), Some("true"));
}

#[test]
fn the_renderer_profile_applies_with_compile_and_apply() {
    let report = run(Some((Profile::Renderer, Method::CompileAndApply)));
    assert_eq!(report.state("sandboxed"), Some("true"));
}

/// The browser's way (Chromium's): the broker compiles the profile and the
/// helper only applies the bytecode, so it pays neither the compilation time
/// nor libsandbox's compiler memory.
#[test]
fn the_renderer_profile_applies_as_bytecode_compiled_by_the_broker() {
    let report = run(Some((Profile::Renderer, Method::Precompiled)));
    assert_eq!(report.state("sandboxed"), Some("true"));
    assert!(report.state("bytecode-bytes").is_some_and(|n| n != "0"));
}

/// The SDK 27.0 man page for `sandbox_init`: the named profiles "must not be
/// used when building against macOS SDK >= 27.0. Processes opting into them
/// will be killed on attempt to do so."
///
/// Ignored by default: each kill leaves a crash report in
/// `~/Library/Logs/DiagnosticReports`. Run it with
/// `cargo test -p s4-sandbox -- --ignored`.
#[test]
#[ignore = "each run leaves a crash report in ~/Library/Logs/DiagnosticReports"]
fn a_named_profile_kills_a_process_built_against_sdk_27() {
    let (exit, output) = scenario::run_named_profile(Path::new(HELPER)).expect("runs");
    assert!(
        matches!(exit, Exit::Signal(_)),
        "expected the helper to be killed; it ended with {exit:?} and printed {output:?}"
    );
}

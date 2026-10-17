//! The S4 broker: a stand-in for Axiom's browser process.
//!
//! Usage: `s4-broker probe [--helper PATH] [--work-dir DIR] [--named-profile]`
//!
//! Runs the S4a scenarios against the helper:
//! 1. the control run, with no profile: every probe must be allowed;
//! 2. the renderer profile through `sandbox_init_with_parameters`;
//! 3. the renderer profile through `sandbox_compile_string` + `sandbox_apply`;
//! 4. with `--named-profile` only: the built-in named profile through
//!    `sandbox_init` (S4-Q1's SDK 27 rule). The kill it causes leaves a crash
//!    report in `~/Library/Logs/DiagnosticReports`.
//!
//! Prints a table per scenario and exits 0 only if every expectation holds.
//! `--helper` defaults to `s4-helper` next to this executable, so a signed
//! copy of both runs as a pair (`tools/s4-sign.sh`).
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use s4_sandbox::platform::process::Exit;
use s4_sandbox::probe;
use s4_sandbox::profile::{Method, Profile};
use s4_sandbox::scenario::{self, ProbeRun};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("probe") {
        eprintln!("usage: s4-broker probe [--helper PATH] [--work-dir DIR] [--named-profile]");
        return ExitCode::from(2);
    }
    let mut helper = None;
    let mut work_dir = scenario::default_work_dir();
    let mut named_profile = false;
    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        if flag == "--named-profile" {
            named_profile = true;
            continue;
        }
        let Some(value) = rest.next() else {
            eprintln!("s4-broker: {flag} needs a value");
            return ExitCode::from(2);
        };
        match flag.as_str() {
            "--helper" => helper = Some(PathBuf::from(value)),
            "--work-dir" => work_dir = PathBuf::from(value),
            other => {
                eprintln!("s4-broker: unknown flag {other}");
                return ExitCode::from(2);
            }
        }
    }
    let helper = match helper.map_or_else(sibling_helper, Ok) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("s4-broker: {message}");
            return ExitCode::from(2);
        }
    };
    println!("helper: {}", helper.display());

    let mut failures = 0;
    let runs = [
        ("control (no profile)", None),
        (
            "renderer, sandbox_init_with_parameters",
            Some((Profile::Renderer, Method::InitWithParameters)),
        ),
        (
            "renderer, sandbox_compile_string + sandbox_apply",
            Some((Profile::Renderer, Method::CompileAndApply)),
        ),
    ];
    for (title, profile) in runs {
        let run = ProbeRun {
            helper: &helper,
            profile,
            work_dir: &work_dir,
        };
        match scenario::run_probe(&run) {
            Ok(report) => {
                failures += print_report(title, &report);
            }
            Err(error) => {
                println!("\n== {title}: could not run: {error}");
                failures += 1;
            }
        }
    }
    if named_profile {
        failures += named_profile_scenario(&helper);
    } else {
        println!("\n== named profile: skipped (pass --named-profile; it leaves a crash report)");
    }
    println!(
        "\n{}",
        if failures == 0 {
            "ALL PASS"
        } else {
            "FAILURES"
        }
    );
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The built-in named profile must kill a helper built against SDK 27.
/// Returns 1 if it didn't, else 0.
fn named_profile_scenario(helper: &std::path::Path) -> usize {
    println!("\n== named profile (kSBXProfilePureComputation), sandbox_init");
    match scenario::run_named_profile(helper) {
        Ok((exit, output)) => {
            let killed = matches!(exit, Exit::Signal(_));
            println!("helper ended with {exit:?}; expected: killed by a signal");
            if !output.trim().is_empty() {
                println!("helper printed: {}", output.trim());
            }
            println!("result: {}", if killed { "PASS" } else { "FAIL" });
            usize::from(!killed)
        }
        Err(error) => {
            println!("could not run: {error}");
            1
        }
    }
}

/// Print one scenario's table. Returns 1 if it failed, else 0.
fn print_report(title: &str, report: &scenario::ProbeReport) -> usize {
    println!("\n== {title}");
    for (key, value) in &report.states {
        println!("state  {key:<22} {value}");
    }
    println!("{:<46} {:<8} {:<8} detail", "probe", "expected", "observed");
    for (name, kind) in probe::catalogue() {
        let expected = probe::expected(kind, report.sandboxed_run);
        match report.result(&name) {
            Some(r) => println!("{name:<46} {expected:<8} {:<8} {}", r.outcome, r.detail),
            None => println!("{name:<46} {expected:<8} {:<8}", "-"),
        }
    }
    match report.broker.mach_hello {
        Some(hello) => println!(
            "broker: Mach hello from pid {} (audit trailer), claiming {}; the child is {}",
            hello.sender_pid, hello.claimed_pid, report.child_pid
        ),
        None => println!("broker: no Mach hello"),
    }
    println!(
        "broker: socket hello {}; TCP connection {}; UDP datagram {}; helper ended with {:?}",
        yes(report.broker.socket_greeted),
        yes(report.broker.tcp_accepted),
        yes(report.broker.udp_received),
        report.exit
    );
    let mismatches = report.mismatches();
    for mismatch in &mismatches {
        println!("MISMATCH: {mismatch}");
    }
    println!(
        "result: {} ({}/{} probes as expected)",
        if mismatches.is_empty() {
            "PASS"
        } else {
            "FAIL"
        },
        report.probes_as_expected(),
        probe::catalogue().len()
    );
    usize::from(!mismatches.is_empty())
}

fn yes(value: bool) -> &'static str {
    if value { "seen" } else { "not seen" }
}

fn sibling_helper() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(exe.with_file_name("s4-helper"))
}

//! The S4 broker: a stand-in for Axiom's browser process.
//!
//! Usage: `s4-broker COMMAND [--helper PATH] [options]`
//!
//! - `probe [--work-dir DIR] [--named-profile]`: the S4a scenarios (S4-Q1,
//!   S4-Q2): the control run, then the renderer profile applied by each
//!   method. With `--named-profile`, also checks that the built-in named
//!   profile kills the helper; each kill leaves a crash report in
//!   `~/Library/Logs/DiagnosticReports`.
//! - `startup [--runs N]`: spawn-to-ready time, footprint and profile
//!   timings of a helper with and without the profile (S4-Q3).
//! - `ipc [--batches N]`: echo latency and 1 MiB transfer time over a socket
//!   pair and over Mach messages, with and without the profile (S4-Q3).
//! - `text [--text PATH] [--runs N]`: CoreText inside the text profile
//!   (S4-Q5): compares sandboxed and unsandboxed rasterization, then measures
//!   the start-up and footprint of a helper that has drawn text.
//! - `jit [--dir DIR]`: the JIT matrix over the signed helper variants in
//!   `DIR` (S4-Q4): `s4-helper`, `s4-helper-jit`, `s4-helper-jit1` and
//!   `s4-helper-plain`, each under four profile configurations.
//!
//! `--helper` and `--dir` default to this executable's directory, so a signed
//! set made by `tools/s4-sign.sh` runs together. Every command exits 0 only
//! if every check it makes holds.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use s4_sandbox::bench::{self, IpcPlan, Series, StartupSample};
use s4_sandbox::platform::process::Exit;
use s4_sandbox::platform::sandbox;
use s4_sandbox::probe;
use s4_sandbox::profile::{Method, Profile, RENDEZVOUS_PARAM};
use s4_sandbox::scenario::{self, ProbeRun};

const USAGE: &str = "usage: s4-broker probe|startup|ipc|text|jit [--helper PATH] [--text PATH] \
                     [--dir DIR] [--work-dir DIR] [--runs N] [--batches N] [--named-profile]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let options = match Options::parse(&args[1..]) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("s4-broker: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let failures = match command.as_str() {
        "probe" => probe_command(&options),
        "startup" => startup_command(&options),
        "ipc" => ipc_command(&options),
        "jit" => jit_command(&options),
        "text" => text_command(&options),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
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

struct Options {
    helper: PathBuf,
    text: PathBuf,
    dir: PathBuf,
    work_dir: PathBuf,
    runs: usize,
    batches: usize,
    named_profile: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let here = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .map(Path::to_path_buf)
            .ok_or("no executable directory")?;
        let mut options = Self {
            helper: here.join("s4-helper"),
            text: here.join("s4-text"),
            dir: here,
            work_dir: scenario::default_work_dir(),
            runs: 30,
            batches: 5,
            named_profile: false,
        };
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
            if flag == "--named-profile" {
                options.named_profile = true;
                continue;
            }
            let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
            let count = || value.parse::<usize>().map_err(|e| format!("{flag}: {e}"));
            match flag.as_str() {
                "--helper" => options.helper = PathBuf::from(value),
                "--text" => options.text = PathBuf::from(value),
                "--dir" => options.dir = PathBuf::from(value),
                "--work-dir" => options.work_dir = PathBuf::from(value),
                "--runs" => options.runs = count()?.max(1),
                "--batches" => options.batches = count()?.max(1),
                other => return Err(format!("unknown flag {other}")),
            }
        }
        Ok(options)
    }
}

/// Profile flags for a helper.
fn profile_args(profile: Option<(Profile, Method)>) -> Vec<String> {
    match profile {
        Some((profile, method)) => vec![
            "--profile".into(),
            profile.name().into(),
            "--method".into(),
            method.name().into(),
        ],
        None => Vec::new(),
    }
}

const CONFIGURATIONS: [(&str, Option<(Profile, Method)>); 3] = [
    ("no profile", None),
    (
        "renderer, sandbox_init_with_parameters",
        Some((Profile::Renderer, Method::InitWithParameters)),
    ),
    (
        "renderer, sandbox_compile_string + sandbox_apply",
        Some((Profile::Renderer, Method::CompileAndApply)),
    ),
];

// ----- probe ---------------------------------------------------------------

fn probe_command(options: &Options) -> usize {
    println!("helper: {}", options.helper.display());
    let mut failures = 0;
    let precompiled = (
        "renderer, bytecode compiled by the broker",
        Some((Profile::Renderer, Method::Precompiled)),
    );
    for (title, profile) in CONFIGURATIONS.iter().copied().chain([precompiled]) {
        let run = ProbeRun {
            helper: &options.helper,
            profile,
            work_dir: &options.work_dir,
        };
        match scenario::run_probe(&run) {
            Ok(report) => failures += print_probe_report(title, &report),
            Err(error) => {
                println!("\n== {title}: could not run: {error}");
                failures += 1;
            }
        }
    }
    if options.named_profile {
        failures += named_profile_scenario(&options.helper);
    } else {
        println!("\n== named profile: skipped (pass --named-profile; it leaves a crash report)");
    }
    failures
}

/// Print one scenario's table. Returns 1 if it failed, else 0.
fn print_probe_report(title: &str, report: &scenario::ProbeReport) -> usize {
    println!("\n== {title}");
    for (key, value) in &report.states {
        println!("state  {key:<22} {value}");
    }
    println!("{:<52} {:<8} {:<8} detail", "probe", "expected", "observed");
    for (name, kind) in probe::catalogue() {
        let expected = probe::expected(kind, report.sandboxed_run);
        match report.result(&name) {
            Some(r) => println!("{name:<52} {expected:<8} {:<8} {}", r.outcome, r.detail),
            None => println!("{name:<52} {expected:<8} {:<8}", "-"),
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
        seen(report.broker.socket_greeted),
        seen(report.broker.tcp_accepted),
        seen(report.broker.udp_received),
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

fn seen(value: bool) -> &'static str {
    if value { "seen" } else { "not seen" }
}

/// The built-in named profile must kill a helper built against SDK 27.
/// Returns 1 if it didn't, else 0.
fn named_profile_scenario(helper: &Path) -> usize {
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

// ----- startup -------------------------------------------------------------

fn startup_command(options: &Options) -> usize {
    println!(
        "helper: {}; {} runs per configuration",
        options.helper.display(),
        options.runs
    );
    // The precompiled method: this process compiles the profile once, with
    // the helper's parameters, and sends the bytecode to each helper.
    let params = [(RENDEZVOUS_PARAM, "org.axiom.s4.unused")];
    let bytecode =
        match sandbox::compile(Profile::Renderer.source(), &params).and_then(|c| c.bytecode()) {
            Ok(bytecode) => bytecode,
            Err(error) => {
                println!("could not compile the renderer profile: {error}");
                return 1;
            }
        };
    println!("renderer profile bytecode: {} bytes", bytecode.len());
    let precompiled = Some((Profile::Renderer, Method::Precompiled));
    let mut failures = 0;
    for (title, profile) in CONFIGURATIONS
        .iter()
        .copied()
        .chain([("renderer, bytecode compiled by the broker", precompiled)])
    {
        let mut args = vec!["ready".to_owned()];
        args.extend(profile_args(profile));
        let code = (profile == precompiled).then_some(bytecode.as_slice());
        let mut samples = Vec::with_capacity(options.runs);
        for _ in 0..options.runs {
            match bench::measure_startup(&options.helper, &args, code) {
                Ok(sample) => samples.push(sample),
                Err(error) => {
                    println!("\n== {title}: a run failed: {error}");
                    failures += 1;
                    break;
                }
            }
        }
        failures += print_startup(title, profile.is_some(), &samples);
    }
    failures
}

fn print_startup(title: &str, sandboxed: bool, samples: &[StartupSample]) -> usize {
    println!("\n== {title}");
    if samples.is_empty() {
        return 1;
    }
    let bad: Vec<&StartupSample> = samples
        .iter()
        .filter(|s| {
            let state = |k: &str| {
                s.states
                    .iter()
                    .find(|(key, _)| key == k)
                    .map(|(_, v)| v.as_str())
            };
            s.exit != Exit::Code(0)
                || state("enter-error").is_some()
                || state("sandboxed") != Some(if sandboxed { "true" } else { "false" })
        })
        .collect();
    let ready: Vec<Duration> = samples.iter().map(|s| s.spawn_to_ready).collect();
    let footprint: Vec<u64> = samples.iter().map(|s| s.footprint.current).collect();
    let lifetime: Vec<u64> = samples.iter().map(|s| s.footprint.lifetime_max).collect();
    println!(
        "spawn to ready      median {:>8.2} ms   min {:>8.2}   max {:>8.2}",
        ms(median(&ready)),
        ms(*ready.iter().min().expect("samples")),
        ms(*ready.iter().max().expect("samples"))
    );
    println!(
        "footprint (ready)   median {:>8.2} MiB  min {:>8.2}   max {:>8.2}",
        mib(median(&footprint)),
        mib(*footprint.iter().min().expect("samples")),
        mib(*footprint.iter().max().expect("samples"))
    );
    println!(
        "footprint (peak)    median {:>8.2} MiB",
        mib(median(&lifetime))
    );
    for key in ["compile-microseconds", "apply-microseconds"] {
        let values: Vec<u64> = samples
            .iter()
            .filter_map(|s| s.states.iter().find(|(k, _)| k == key))
            .filter_map(|(_, v)| v.parse().ok())
            .collect();
        if !values.is_empty() {
            println!(
                "{:<19} median {:>8} µs   min {:>8}   max {:>8}",
                key.trim_end_matches("-microseconds"),
                median(&values),
                values.iter().min().expect("values"),
                values.iter().max().expect("values")
            );
        }
    }
    println!(
        "result: {} ({} of {} runs as expected)",
        if bad.is_empty() { "PASS" } else { "FAIL" },
        samples.len() - bad.len(),
        samples.len()
    );
    usize::from(!bad.is_empty())
}

// ----- ipc -----------------------------------------------------------------

fn ipc_command(options: &Options) -> usize {
    let plan = IpcPlan {
        batches: options.batches,
        ..IpcPlan::default()
    };
    println!(
        "helper: {}; echo {} B × {} per batch; bulk {} KiB × {} per batch; {} batches after a warm-up",
        options.helper.display(),
        plan.echo_bytes,
        plan.echo_rounds,
        plan.bulk_bytes / 1024,
        plan.bulk_rounds,
        plan.batches
    );
    let renderer = Some((Profile::Renderer, Method::InitWithParameters));
    let runs = [
        ("no profile; default socket buffers", None, None),
        ("renderer profile; default socket buffers", renderer, None),
        (
            "renderer profile; 1 MiB socket buffers",
            renderer,
            Some(1 << 20),
        ),
    ];
    let mut failures = 0;
    for (title, profile, socket_buffer) in runs {
        println!("\n== {title}");
        let plan = IpcPlan {
            socket_buffer,
            ..plan
        };
        match bench::measure_ipc(&options.helper, &profile_args(profile), plan) {
            Ok(report) => {
                print_series("socket echo, round trip", &report.socket_echo, None);
                print_series("Mach echo, round trip", &report.mach_echo, None);
                print_series(
                    "socket 1 MiB, round trip",
                    &report.socket_bulk,
                    Some(plan.bulk_bytes),
                );
                print_series(
                    "Mach 1 MiB out of line",
                    &report.mach_bulk,
                    Some(plan.bulk_bytes),
                );
                let ok = report.hello_from_child && report.exit == Exit::Code(0);
                println!(
                    "hello from the child (audit trailer): {}; helper ended with {:?}",
                    report.hello_from_child, report.exit
                );
                println!("result: {}", if ok { "PASS" } else { "FAIL" });
                failures += usize::from(!ok);
            }
            Err(error) => {
                println!("could not run: {error}");
                failures += 1;
            }
        }
    }
    failures
}

fn print_series(title: &str, series: &Series, bytes: Option<usize>) {
    let (median, min, max) = (series.median(), series.min(), series.max());
    match bytes {
        None => println!(
            "{title:<28} median {:>8.2} µs   min {:>8.2}   max {:>8.2}",
            us(median),
            us(min),
            us(max)
        ),
        Some(bytes) => println!(
            "{title:<28} median {:>8.1} µs   min {:>8.1}   max {:>8.1}   ({:.1} GB/s at the median)",
            us(median),
            us(min),
            us(max),
            gbps(bytes, median)
        ),
    }
}

// ----- jit -----------------------------------------------------------------

fn jit_command(options: &Options) -> usize {
    let variants = [
        ("s4-helper", "hardened, no entitlement"),
        ("s4-helper-jit", "hardened + allow-jit"),
        ("s4-helper-jit1", "hardened + allow-jit + single-jit"),
        ("s4-helper-plain", "no hardened runtime"),
    ];
    let profiles: [(&str, Option<(Profile, Method)>); 4] = [
        ("no profile", None),
        (
            "bare: default deny only",
            Some((Profile::Bare, Method::InitWithParameters)),
        ),
        (
            "renderer: explicit deny",
            Some((Profile::Renderer, Method::InitWithParameters)),
        ),
        (
            "renderer-jit: explicit allow",
            Some((Profile::RendererJit, Method::InitWithParameters)),
        ),
    ];
    println!("helpers in {}", options.dir.display());
    println!(
        "\n{:<50} {:<26} {:<30} {:<4} {:<30}",
        "executable", "profile", "MAP_JIT", "ran", "second MAP_JIT region"
    );
    let mut failures = 0;
    for (file, description) in variants {
        let helper = options.dir.join(file);
        for (profile_title, profile) in profiles {
            match bench::run_jit(&helper, &profile_args(profile)) {
                Ok((exit, states)) => {
                    let state = |key: &str| {
                        states
                            .iter()
                            .find(|(k, _)| k == key)
                            .map_or("-", |(_, v)| v.as_str())
                    };
                    let entered = state("enter-error") == "-";
                    println!(
                        "{:<50} {:<26} {:<30} {:<4} {:<30}",
                        format!("{file} ({description})"),
                        profile_title,
                        state("jit-map"),
                        state("jit-ran"),
                        state("jit-second-map")
                    );
                    let ran_right = matches!(state("jit-ran"), "-" | "42");
                    if exit != Exit::Code(0) || !entered || !ran_right {
                        println!(
                            "  FAIL: exit {exit:?}, enter-error {}",
                            state("enter-error")
                        );
                        failures += 1;
                    }
                }
                Err(error) => {
                    println!("{file:<34} {profile_title:<28} could not run: {error}");
                    failures += 1;
                }
            }
        }
    }
    failures
}

// ----- text ----------------------------------------------------------------

fn text_command(options: &Options) -> usize {
    println!("text helper: {}", options.text.display());
    let render = |profile: Option<&str>| {
        let mut args = vec!["render".to_owned()];
        if let Some(profile) = profile {
            args.extend(["--profile".to_owned(), profile.to_owned()]);
        }
        bench::run_report(&options.text, &args)
    };
    let lines = |states: &[(String, String)]| -> Vec<String> {
        states
            .iter()
            .filter(|(k, _)| k.starts_with("line"))
            .map(|(_, v)| v.clone())
            .collect()
    };
    // A line's identity: everything but its timing.
    let identity = |line: &str| {
        line.rsplit_once(", ")
            .map_or(line, |(head, _)| head)
            .to_owned()
    };
    let mut failures = 0;
    let reference = match render(None) {
        Ok((Exit::Code(0), states)) => lines(&states),
        other => {
            println!("the unsandboxed render failed: {other:?}");
            return 1;
        }
    };
    println!("\n== rasterization");
    for line in &reference {
        println!("unsandboxed              {line}");
    }
    for (profile, should_match) in [("text", true), ("renderer", false)] {
        match render(Some(profile)) {
            Ok((exit, states)) => {
                let got = lines(&states);
                let same = got.len() == reference.len()
                    && got
                        .iter()
                        .zip(&reference)
                        .all(|(a, b)| identity(a) == identity(b));
                for line in &got {
                    println!("{profile:<24} {line}");
                }
                let error = states.iter().find(|(k, _)| k == "enter-error");
                let ok = exit == Exit::Code(0) && error.is_none() && same == should_match;
                println!(
                    "{profile}: pixel-identical to unsandboxed: {same} (expected {should_match}): {}",
                    if ok { "PASS" } else { "FAIL" }
                );
                failures += usize::from(!ok);
            }
            Err(error) => {
                println!("{profile}: could not run: {error}");
                failures += 1;
            }
        }
    }
    for (title, profile) in [
        ("text helper, no profile", None),
        ("text helper, text profile", Some("text")),
    ] {
        let mut args = vec!["ready".to_owned()];
        if let Some(profile) = profile {
            args.extend(["--profile".to_owned(), profile.to_owned()]);
        }
        let mut samples = Vec::with_capacity(options.runs);
        for _ in 0..options.runs {
            match bench::measure_startup(&options.text, &args, None) {
                Ok(sample) => samples.push(sample),
                Err(error) => {
                    println!("\n== {title}: a run failed: {error}");
                    failures += 1;
                    break;
                }
            }
        }
        failures += print_startup(title, profile.is_some(), &samples);
    }
    failures
}

// ----- helpers -------------------------------------------------------------

fn median<T: Copy + Ord>(values: &[T]) -> T {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn us(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e6
}

#[expect(
    clippy::cast_precision_loss,
    reason = "footprints are far below 2^52 bytes"
)]
fn mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "transfer sizes are far below 2^52 bytes"
)]
fn gbps(bytes: usize, duration: Duration) -> f64 {
    bytes as f64 / duration.as_secs_f64() / 1e9
}

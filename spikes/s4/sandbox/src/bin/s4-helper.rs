//! The S4 helper: a stand-in for a renderer process.
//!
//! Usage:
//! - `s4-helper probe [--profile NAME --method init|compile] --rendezvous NAME
//!   --dir DIR --tcp PORT --udp PORT --parent PID`
//!
//!   Applies the profile first, if one is given, then runs every probe and
//!   prints its report (`s4_sandbox::probe`). Without a profile, this is the
//!   control run.
//! - `s4-helper named-profile`
//!
//!   Applies the built-in "pure computation" profile with `sandbox_init`,
//!   which the SDK 27.0 man page says kills a process built against that SDK.
//!
//! The broker launches it (`s4-broker`, or the tests).
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use s4_sandbox::platform::sandbox;
use s4_sandbox::probe::{self, Config, Line};
use s4_sandbox::profile::{Method, Profile, RENDEZVOUS_PARAM};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("probe") => probe_mode(&args[1..]),
        Some("named-profile") => named_profile_mode(),
        _ => {
            eprintln!("usage: s4-helper probe … | named-profile");
            ExitCode::from(2)
        }
    }
}

/// Parsed `probe` arguments.
struct ProbeArgs {
    profile: Option<(Profile, Method)>,
    config: Config,
}

fn parse_probe_args(args: &[String]) -> Result<ProbeArgs, String> {
    let mut profile = None;
    let mut method = Method::InitWithParameters;
    let (mut rendezvous, mut dir, mut tcp, mut udp, mut parent) = (None, None, None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--profile" => profile = Some(value.parse::<Profile>()?),
            "--method" => method = value.parse()?,
            "--rendezvous" => rendezvous = Some(value.clone()),
            "--dir" => dir = Some(PathBuf::from(value)),
            "--tcp" => tcp = Some(value.parse::<u16>().map_err(|e| e.to_string())?),
            "--udp" => udp = Some(value.parse::<u16>().map_err(|e| e.to_string())?),
            "--parent" => parent = Some(value.parse::<i32>().map_err(|e| e.to_string())?),
            other => return Err(format!("unknown flag {other}")),
        }
    }
    let missing = |name: &str| format!("missing {name}");
    Ok(ProbeArgs {
        profile: profile.map(|p| (p, method)),
        config: Config {
            dir: dir.ok_or_else(|| missing("--dir"))?,
            tcp_port: tcp.ok_or_else(|| missing("--tcp"))?,
            udp_port: udp.ok_or_else(|| missing("--udp"))?,
            parent_pid: parent.ok_or_else(|| missing("--parent"))?,
            rendezvous: rendezvous.ok_or_else(|| missing("--rendezvous"))?,
        },
    })
}

fn probe_mode(args: &[String]) -> ExitCode {
    // Parsing touches no resources, so it may come before the profile.
    let parsed = match parse_probe_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("s4-helper: {message}");
            return ExitCode::from(2);
        }
    };
    // The profile goes on before the helper opens anything (sandbox(7):
    // restrictions apply when a resource is acquired).
    let mut lines = Vec::new();
    if let Some((profile, method)) = parsed.profile {
        let start = Instant::now();
        let entered = sandbox::enter(
            profile.source(),
            &[(RENDEZVOUS_PARAM, &parsed.config.rendezvous)],
            method,
        );
        let elapsed = start.elapsed();
        lines.push(state("profile", profile.name()));
        lines.push(state("method", method.name()));
        lines.push(state(
            "apply-microseconds",
            &elapsed.as_micros().to_string(),
        ));
        if let Err(error) = entered {
            lines.push(state("enter-error", &error));
        }
    }
    lines.push(state("sandboxed", &sandbox::is_sandboxed().to_string()));

    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in &lines {
        if writeln!(out, "{line}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    match probe::run_all(&parsed.config, &mut out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn named_profile_mode() -> ExitCode {
    let result = sandbox::enter_named_pure_computation();
    // Reached only if the process wasn't killed.
    let line = match result {
        Ok(()) => state("named-profile", "applied"),
        Err(error) => state("named-profile", &format!("error: {error}")),
    };
    println!("{line}");
    ExitCode::SUCCESS
}

fn state(key: &str, value: &str) -> Line {
    Line::State(key.to_owned(), value.to_owned())
}

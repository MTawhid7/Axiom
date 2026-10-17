//! The S4 helper: a stand-in for a renderer process.
//!
//! Every mode applies its profile first, if one is given, before it opens
//! anything (sandbox(7): restrictions apply when a resource is acquired).
//! Profile flags: `--profile renderer|renderer-jit|text --method init|compile
//! --rendezvous NAME` (the profile's rendezvous parameter).
//!
//! Modes:
//! - `probe … --dir DIR --tcp PORT --udp PORT --parent PID`: run every probe
//!   and print the report (`s4_sandbox::probe`). Without a profile, this is
//!   the control run.
//! - `ready …`: report profile timings, say ready on the inherited socket,
//!   and wait for the broker's quit (start-up and footprint, S4-Q3).
//! - `serve …`: answer echo and bulk requests over the inherited socket and a
//!   Mach port (IPC, S4-Q3).
//! - `jit …`: try to map, fill and run JIT code (S4-Q4).
//! - `named-profile`: apply the built-in "pure computation" profile with
//!   `sandbox_init`, which kills a process built against SDK 27.0.
//!
//! The broker launches it (`s4-broker`, or the tests).
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use std::os::unix::net::UnixStream;

use s4_sandbox::bench;
use s4_sandbox::platform::{jit, process, sandbox};
use s4_sandbox::probe::{self, Config, Line, SOCKET_FD};
use s4_sandbox::profile::{Method, Profile, RENDEZVOUS_PARAM};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = args.first().map(String::as_str) else {
        return usage();
    };
    if mode == "named-profile" {
        return named_profile_mode();
    }
    // Parsing touches no resources, so it may come before the profile.
    let flags = match Flags::parse(&args[1..]) {
        Ok(flags) => flags,
        Err(message) => {
            eprintln!("s4-helper: {message}");
            return ExitCode::from(2);
        }
    };
    let (mut lines, socket) = flags.enter_profile();
    lines.push(state("sandboxed", &sandbox::is_sandboxed().to_string()));
    let result = match mode {
        "probe" => flags.config().and_then(|config| {
            report(&lines)?;
            probe::run_all(&config, &mut io::stdout().lock(), socket).map_err(|e| e.to_string())
        }),
        "ready" => report(&lines).and_then(|()| {
            match socket {
                Some(socket) => bench::ready_and_wait_on(socket),
                None => bench::ready_and_wait(),
            }
            .map_err(|e| e.to_string())
        }),
        "serve" => report(&lines).and_then(|()| {
            let rendezvous = flags.rendezvous.as_deref().ok_or("missing --rendezvous")?;
            bench::serve(rendezvous).map_err(|e| e.to_string())
        }),
        "jit" => {
            let outcome = jit::attempt();
            lines.push(state("jit-map", &errno_text(outcome.map)));
            lines.push(state(
                "jit-ran",
                &outcome.ran.map_or("-".to_owned(), |v| v.to_string()),
            ));
            lines.push(state(
                "jit-second-map",
                &outcome.second_map.map_or("-".to_owned(), errno_text),
            ));
            report(&lines)
        }
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("s4-helper: {message}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: s4-helper probe|ready|serve|jit [flags] | named-profile");
    ExitCode::from(2)
}

/// Command-line flags shared by the modes.
#[derive(Default)]
struct Flags {
    profile: Option<Profile>,
    method: Option<Method>,
    rendezvous: Option<String>,
    dir: Option<PathBuf>,
    tcp: Option<u16>,
    udp: Option<u16>,
    parent: Option<i32>,
}

impl Flags {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut flags = Self::default();
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
            let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
            let invalid = |e: std::num::ParseIntError| format!("{flag}: {e}");
            match flag.as_str() {
                "--profile" => flags.profile = Some(value.parse()?),
                "--method" => flags.method = Some(value.parse()?),
                "--rendezvous" => flags.rendezvous = Some(value.clone()),
                "--dir" => flags.dir = Some(PathBuf::from(value)),
                "--tcp" => flags.tcp = Some(value.parse().map_err(invalid)?),
                "--udp" => flags.udp = Some(value.parse().map_err(invalid)?),
                "--parent" => flags.parent = Some(value.parse().map_err(invalid)?),
                other => return Err(format!("unknown flag {other}")),
            }
        }
        Ok(flags)
    }

    /// Apply the profile, if any, and describe what happened. With the
    /// precompiled method, the bytecode arrives on the inherited socket, which
    /// is returned for the mode to keep using.
    fn enter_profile(&self) -> (Vec<Line>, Option<UnixStream>) {
        let mut lines = Vec::new();
        let mut socket = None;
        let Some(profile) = self.profile else {
            return (lines, socket);
        };
        let method = self.method.unwrap_or(Method::InitWithParameters);
        let rendezvous = self.rendezvous.as_deref().unwrap_or("org.axiom.s4.unused");
        let params = [(RENDEZVOUS_PARAM, rendezvous)];
        lines.push(state("profile", profile.name()));
        lines.push(state("method", method.name()));
        let result = match method {
            Method::InitWithParameters => {
                let start = Instant::now();
                let result = sandbox::enter(profile.source(), &params, method);
                lines.push(state("apply-microseconds", &micros(start)));
                result
            }
            Method::CompileAndApply => {
                let start = Instant::now();
                let compiled = sandbox::compile(profile.source(), &params);
                lines.push(state("compile-microseconds", &micros(start)));
                let start = Instant::now();
                let result = compiled.and_then(|c| c.apply());
                lines.push(state("apply-microseconds", &micros(start)));
                result
            }
            Method::Precompiled => {
                let received = process::adopt_inherited_socket(SOCKET_FD)
                    .and_then(|mut s| bench::read_bytecode(&mut s).map(|code| (s, code)))
                    .map_err(|e| e.to_string());
                received.and_then(|(stream, bytecode)| {
                    socket = Some(stream);
                    let start = Instant::now();
                    let result = sandbox::apply_bytecode(&bytecode);
                    lines.push(state("apply-microseconds", &micros(start)));
                    lines.push(state("bytecode-bytes", &bytecode.len().to_string()));
                    result
                })
            }
        };
        if let Err(error) = result {
            lines.push(state("enter-error", &error));
        }
        (lines, socket)
    }

    fn config(&self) -> Result<Config, String> {
        let missing = |name: &str| format!("missing {name}");
        Ok(Config {
            dir: self.dir.clone().ok_or_else(|| missing("--dir"))?,
            tcp_port: self.tcp.ok_or_else(|| missing("--tcp"))?,
            udp_port: self.udp.ok_or_else(|| missing("--udp"))?,
            parent_pid: self.parent.ok_or_else(|| missing("--parent"))?,
            rendezvous: self
                .rendezvous
                .clone()
                .ok_or_else(|| missing("--rendezvous"))?,
        })
    }
}

/// Print report lines to standard output, an inherited descriptor.
fn report(lines: &[Line]) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in lines {
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
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

fn micros(start: Instant) -> String {
    start.elapsed().as_micros().to_string()
}

fn errno_text(result: Result<(), i32>) -> String {
    let name = |errno: i32| match errno {
        1 => "EPERM",
        12 => "ENOMEM",
        13 => "EACCES",
        22 => "EINVAL",
        _ => "",
    };
    match result {
        Ok(()) => "ok".to_owned(),
        Err(errno) => format!("errno {errno} {}", name(errno))
            .trim_end()
            .to_owned(),
    }
}

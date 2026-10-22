//! Spike S16, for the architecture review's Round 4 (docs/research/s16-results.md):
//! what happens to shared memory and IOSurface objects when the process that
//! shared them dies (S16-Q1); port rights in dropped messages, failed sends and
//! lookups of names that aren't surfaces (S16-Q2); and whether rusage's
//! per-process counters can serve as low-noise efficiency metrics (S16-Q3).
//!
//! Usage:
//! - `s16 memory --compositor browser|gpu --end kill|exit [--cpu-touch] [--images N] [--side N] [--out DIR]`:
//!   S16-Q1 (a). A sandboxed renderer shares N images as memory entries; the
//!   compositor, this process or a sandboxed GPU process, samples them in
//!   place; the renderer is then killed or told to exit, and the compositor
//!   later lets go. Samples every process's charge and the host's pages at
//!   each stage.
//! - `s16 surfaces --end kill|exit [--surfaces N] [--side N] [--out DIR]`:
//!   S16-Q1 (b). A sandboxed GPU process makes N tiles and one more that
//!   carries attached values; this process shows the tiles as layer
//!   contents; the GPU process dies; then this process lets go.
//! - `s16 rights [--count N] [--out DIR]`: S16-Q2.
//! - `s16 counters [--runs N] [--load N] [--workloads cpu,alloc,disk,sandboxed-cpu] [--label L] [--out DIR] [-- COMMAND ARGS...]`:
//!   S16-Q3.
//! - `s16 helper renderer|gpu|sender|worker --rendezvous NAME --profile P [--work W]`:
//!   the helpers, which this process launches.
//! - `s16 work cpu|alloc|disk|spin [--dir DIR]`: S16-Q3's workloads.
//!
//! Profiles: S10's `gpu` and `renderer`, and `none`.
#![deny(unsafe_code)]

mod counters;
mod helper;
mod launch;
mod memory;
mod platform;
mod profiles;
mod protocol;
mod rights;
mod sample;
mod surfaces;
mod work;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use memory::{End, Placement};

const USAGE: &str =
    "usage: s16 memory|surfaces|rights|counters [options] | s16 helper ROLE ... | s16 work NAME";

/// The value after `flag`, if present.
fn option<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn number<T: std::str::FromStr>(args: &[String], flag: &str, default: T) -> Result<T, String> {
    option(args, flag).map_or(Ok(default), |v| {
        v.parse().map_err(|_| format!("{flag}: not a number: {v}"))
    })
}

fn end(args: &[String]) -> Result<End, String> {
    match option(args, "--end").unwrap_or("kill") {
        "kill" => Ok(End::Kill),
        "exit" => Ok(End::Exit),
        other => Err(format!("--end: kill or exit, not {other}")),
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let command = args.first().map(String::as_str).ok_or(USAGE)?;
    let out = option(args, "--out").map(PathBuf::from);
    match command {
        "helper" => {
            let role = args.get(1).ok_or("helper: a role is required")?;
            let rendezvous = option(args, "--rendezvous").ok_or("--rendezvous is required")?;
            let profile = option(args, "--profile").unwrap_or("none");
            helper::run(role, rendezvous, profile, option(args, "--work"))
        }
        "work" => {
            let name = args.get(1).ok_or("work: a workload is required")?;
            work::run(name, option(args, "--dir").map(Path::new))
        }
        "memory" => memory::run(&memory::Options {
            placement: match option(args, "--compositor").unwrap_or("browser") {
                "browser" => Placement::Browser,
                "gpu" => Placement::Gpu,
                other => return Err(format!("--compositor: browser or gpu, not {other}")),
            },
            end: end(args)?,
            cpu_touch: args.iter().any(|a| a == "--cpu-touch"),
            images: number(args, "--images", 16)?,
            side: number(args, "--side", 1_024)?,
            out,
        }),
        "surfaces" => surfaces::run(&surfaces::Options {
            end: end(args)?,
            surfaces: number(args, "--surfaces", 32)?,
            side: number(args, "--side", 256)?,
            out,
        }),
        "rights" => rights::run(number(args, "--count", 1_000)?, out),
        "counters" => {
            let split = args.iter().position(|a| a == "--").unwrap_or(args.len());
            counters::run(&counters::Options {
                runs: number(&args[..split], "--runs", 10)?,
                load: number(&args[..split], "--load", 0)?,
                workloads: option(&args[..split], "--workloads")
                    .unwrap_or("cpu,alloc,disk,sandboxed-cpu")
                    .split(',')
                    .filter(|w| !w.is_empty())
                    .map(str::to_owned)
                    .collect(),
                command: args.get(split + 1..).unwrap_or_default().to_vec(),
                label: option(&args[..split], "--label")
                    .unwrap_or("run")
                    .to_owned(),
                out,
            })
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("s16 {}: {error}", args.first().map_or("", String::as_str));
            ExitCode::FAILURE
        }
    }
}

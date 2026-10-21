//! Spike S10: the GPU/compositor process the architecture review's RV1-02
//! adds, between a renderer and the browser (docs/research/s10-results.md).
//!
//! Usage:
//! - `s10 hop [--frames N] [--items N] [--gpu-profile P] [--renderer-profile P] [--out DIR]`:
//!   S10-Q1. The browser offers frames at 60 Hz; the renderer answers each
//!   with a display-list update; the GPU process validates it, renders the
//!   damaged tiles into IOSurface objects and commits them to the browser's layer
//!   tree. Prints each stage's latency and each process's CPU per frame. The
//!   page has `--items` items (default 44,372, S6's page), which the renderer
//!   sends whole once and the GPU process keeps.
//! - `s10 resources --path no-copy|blit|iosurface [--images N] [--side N] [--gpu-profile P] [--out DIR]`:
//!   S10-Q2. The renderer shares N images; the GPU process registers them by
//!   the path, draws them, and reports the time; the browser reads every
//!   process's footprint.
//! - `s10 validate [--sizes A,B,C] [--iterations N] [--out DIR]`: S10-Q3, in
//!   one process.
//! - `s10 gpu|renderer --rendezvous NAME --profile P`: the helpers, which the
//!   browser launches.
//!
//! Profiles: `gpu`, `renderer`, `renderer-iosurface` (the old path's),
//! `report` (everything allowed and logged) and `none`.
#![deny(unsafe_code)]

mod browser;
mod display_list;
mod gpu_process;
mod platform;
mod profiles;
mod protocol;
mod renderer;
mod stats;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

use protocol::{Config, Path};

const USAGE: &str =
    "usage: s10 hop|resources|validate [options] | s10 gpu|renderer --rendezvous NAME --profile P";

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

fn run(args: &[String]) -> Result<(), String> {
    let command = args.first().map(String::as_str).ok_or(USAGE)?;
    let out = option(args, "--out").map(PathBuf::from);
    let gpu_profile = option(args, "--gpu-profile").unwrap_or("gpu").to_owned();
    match command {
        "gpu" | "renderer" => {
            let rendezvous = option(args, "--rendezvous").ok_or("--rendezvous is required")?;
            let profile = option(args, "--profile").unwrap_or("none");
            if command == "gpu" {
                gpu_process::run(rendezvous, profile)
            } else {
                renderer::run(rendezvous, profile)
            }
        }
        "hop" => {
            let config = Config {
                mode: 0,
                path: Path::NoCopy,
                side: 256,
                columns: 10,
                rows: 7,
                items: number(args, "--items", 44_372)?,
                images: 0,
                image_side: 0,
            };
            let options = browser::Options {
                config,
                frames: number(args, "--frames", 600)?,
                gpu_profile,
                renderer_profile: option(args, "--renderer-profile")
                    .unwrap_or("renderer")
                    .to_owned(),
                out,
            };
            browser::hop(&options)
        }
        "resources" => {
            let path = match option(args, "--path").unwrap_or("no-copy") {
                "no-copy" => Path::NoCopy,
                "blit" => Path::Blit,
                "iosurface" => Path::Surface,
                other => return Err(format!("no path named {other}")),
            };
            let config = Config {
                mode: 1,
                path,
                side: 256,
                columns: 10,
                rows: 7,
                items: 0,
                images: number(args, "--images", 16)?,
                image_side: number(args, "--side", 1024)?,
            };
            let options = browser::Options {
                config,
                frames: 0,
                gpu_profile,
                renderer_profile: option(args, "--renderer-profile")
                    .unwrap_or(browser::renderer_profile_for(path))
                    .to_owned(),
                out,
            };
            browser::resources(&options)
        }
        "validate" => {
            let sizes = option(args, "--sizes")
                .unwrap_or("1000,10000,44372")
                .split(',')
                .map(|s| s.parse().map_err(|_| format!("--sizes: not a number: {s}")))
                .collect::<Result<Vec<usize>, String>>()?;
            validate::run(&sizes, number(args, "--iterations", 200)?, out.as_deref())
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("s10 {}: {error}", args.first().map_or("", String::as_str));
            ExitCode::FAILURE
        }
    }
}

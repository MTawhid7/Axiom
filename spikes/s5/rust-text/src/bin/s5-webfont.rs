//! # Spike S5, S5-Q3: web fonts through the Rust path, inside the renderer's sandbox
//!
//! Usage:
//! - `s5-webfont check [--profile renderer|none] [--timeout-ms N] FILE...`:
//!   read every file, then enter S4's renderer profile (unchanged), then run
//!   each file through the Rust path and print one line per file:
//!   `FILE<TAB>accepted<TAB>details` or `FILE<TAB>rejected<TAB>stage: reason`.
//!   A watchdog ends the process with status 124 after the timeout (default
//!   10 s), so a hang shows as a status, not a stuck run.
//! - `s5-webfont mutate --seed N --count N --out DIR FILE...`: write
//!   deterministic malformed variants of each file into `DIR`.
//!
//! This executable links no Apple framework (`otool -L`), so no CoreText call
//! can receive web-font bytes. The files are read before the profile is
//! applied, as a renderer receives fonts from its loader over IPC, never from
//! the file system.
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use s4_sandbox::platform::sandbox;
use s4_sandbox::profile::{Method, RENDERER, RENDEZVOUS_PARAM};
use s5_rust_text::{Limits, mutate, webfont};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("mutate") => mutate(&args[1..]),
        _ => Err("usage: s5-webfont check [--profile renderer|none] [--timeout-ms N] FILE... | mutate --seed N --count N --out DIR FILE...".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("s5-webfont: {message}");
            ExitCode::from(2)
        }
    }
}

fn check(args: &[String]) -> Result<(), String> {
    let mut profile = true;
    let mut timeout = Duration::from_secs(10);
    let mut files = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--profile" => match rest.next().map(String::as_str) {
                Some("renderer") => profile = true,
                Some("none") => profile = false,
                other => return Err(format!("unknown profile {other:?}")),
            },
            "--timeout-ms" => {
                let ms = rest
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--timeout-ms needs a number")?;
                timeout = Duration::from_millis(ms);
            }
            file => files.push(PathBuf::from(file)),
        }
    }
    // Everything is read before the profile goes on.
    let inputs: Vec<(PathBuf, Vec<u8>)> = files
        .into_iter()
        .map(|path| std::fs::read(&path).map(|data| (path, data)))
        .collect::<io::Result<_>>()
        .map_err(|e| e.to_string())?;
    let mut out = io::stdout().lock();
    if profile {
        sandbox::enter(
            RENDERER,
            &[(RENDEZVOUS_PARAM, "org.axiom.s5.unused")],
            Method::InitWithParameters,
        )
        .map_err(|e| format!("the renderer profile did not apply: {e}"))?;
    }
    writeln!(out, "# sandboxed={}", sandbox::is_sandboxed()).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        std::thread::sleep(timeout);
        eprintln!("s5-webfont: timeout");
        std::process::exit(124);
    });
    let limits = Limits::default();
    for (path, data) in &inputs {
        let line = match webfont::check(data, &limits) {
            Ok(r) => format!(
                "accepted\t{} {} font(s), {} bytes decoded; {} text(s), {} glyphs shaped; {} masks, {} glyph errors, ink {}, hash {:016x}; decode {} µs, shape {} µs, raster {} µs",
                r.kind.map_or("?", webfont::Kind::name),
                r.fonts,
                r.decoded_bytes,
                r.texts,
                r.glyphs_shaped,
                r.masks,
                r.glyph_errors,
                r.ink,
                r.hash,
                r.decode_us,
                r.shape_us,
                r.raster_us
            ),
            Err(rejection) => format!("rejected\t{rejection}"),
        };
        writeln!(out, "{}\t{line}", path.display()).map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn mutate(args: &[String]) -> Result<(), String> {
    let (mut seed, mut count, mut out_dir) = (1_u64, 100_usize, None);
    let mut files = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--seed" => seed = value()?.parse().map_err(|_| "--seed needs a number")?,
            "--count" => count = value()?.parse().map_err(|_| "--count needs a number")?,
            "--out" => out_dir = Some(PathBuf::from(value()?)),
            file => files.push(PathBuf::from(file)),
        }
    }
    let out_dir = out_dir.ok_or("--out is required")?;
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let limits = Limits::default();
    for (file_index, path) in files.iter().enumerate() {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let stem = file_stem(path);
        let file_seed = seed.wrapping_add((file_index as u64) << 32);
        for (i, mutant) in mutate::mutants(&data, file_seed, count, &limits)
            .iter()
            .enumerate()
        {
            let name = format!("{stem}.{i:04}.{}.bin", mutant.strategy);
            std::fs::write(out_dir.join(name), &mutant.data).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn file_stem(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("font")
        .to_owned()
}

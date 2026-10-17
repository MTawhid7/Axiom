//! # Spike S4, S4-Q5: CoreText inside a renderer sandbox
//!
//! Can CoreText lay out and rasterize text with system fonts inside a
//! renderer-like Seatbelt profile that denies the window server and the font
//! server, as WebKit's content profile does? Which rules does it need?
//!
//! Usage: `s4-text render|ready [--profile NAME | --profile-file PATH] [--method init|compile]`
//! - `render`: apply the profile first (if one is given), then rasterize the
//!   test lines and print a report: size, ink and a hash per line, or the
//!   error. The broker compares it with an unsandboxed run.
//! - `ready`: the same, then say ready on the inherited socket and wait for
//!   the broker's quit, so the broker can measure start-up and footprint.
//!
//! `--profile-file` reads an SBPL file (before entering it), for growing the
//! text profile from logged denials without rebuilding.
#![deny(unsafe_code)]

mod platform;

use std::process::ExitCode;
use std::time::Instant;

use s4_sandbox::bench;
use s4_sandbox::platform::sandbox;
use s4_sandbox::probe::Line;
use s4_sandbox::profile::{Method, Profile, RENDEZVOUS_PARAM};

use crate::platform::text::rasterize_line;

/// What gets drawn: two system fonts, Latin text with a ligature candidate and
/// punctuation.
const LINES: [(&str, &str); 2] = [
    ("Helvetica", "Axiom S4: CoreText in a sandbox — fi fl"),
    ("Times New Roman", "The quick brown fox, 0123456789"),
];
const SIZE: f64 = 40.0;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = args.first().map(String::as_str) else {
        return usage();
    };
    let (profile, method, profile_file) = match parse_profile(&args[1..]) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("s4-text: {message}");
            return ExitCode::from(2);
        }
    };
    // The profile goes on before CoreText draws anything.
    let mut lines = Vec::new();
    let source = match (profile, &profile_file) {
        (Some(profile), _) => Some((profile.name().to_owned(), profile.source().to_owned())),
        (None, Some(path)) => match std::fs::read_to_string(path) {
            Ok(text) => Some((path.clone(), text)),
            Err(error) => {
                eprintln!("s4-text: {path}: {error}");
                return ExitCode::from(2);
            }
        },
        (None, None) => None,
    };
    if let Some((name, text)) = source {
        let start = Instant::now();
        let result = sandbox::enter(&text, &[(RENDEZVOUS_PARAM, "org.axiom.s4.unused")], method);
        lines.push(state("profile", &name));
        lines.push(state("apply-microseconds", &micros(start)));
        if let Err(error) = result {
            lines.push(state("enter-error", &error));
        }
    }
    lines.push(state("sandboxed", &sandbox::is_sandboxed().to_string()));
    for (index, (font, text)) in LINES.iter().enumerate() {
        let start = Instant::now();
        match rasterize_line(text, font, SIZE) {
            Ok(mask) => {
                let ink = mask.pixels.iter().filter(|&&c| c > 0).count();
                lines.push(state(
                    &format!("line{index}"),
                    &format!(
                        "{font}: {}x{} px, ink {ink}, hash {:016x}, {} µs",
                        mask.width,
                        mask.height,
                        fnv1a(&mask.pixels),
                        start.elapsed().as_micros()
                    ),
                ));
            }
            Err(error) => lines.push(state(
                &format!("line{index}"),
                &format!("{font}: error: {error}"),
            )),
        }
    }
    for line in &lines {
        println!("{line}");
    }
    match mode {
        "render" => ExitCode::SUCCESS,
        "ready" => match bench::ready_and_wait() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("s4-text: {error}");
                ExitCode::FAILURE
            }
        },
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: s4-text render|ready [--profile NAME | --profile-file PATH] [--method init|compile]"
    );
    ExitCode::from(2)
}

type ProfileArgs = (Option<Profile>, Method, Option<String>);

fn parse_profile(args: &[String]) -> Result<ProfileArgs, String> {
    let (mut profile, mut method, mut file) = (None, Method::InitWithParameters, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--profile" => profile = Some(value.parse()?),
            "--profile-file" => file = Some(value.clone()),
            "--method" => method = value.parse()?,
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok((profile, method, file))
}

fn state(key: &str, value: &str) -> Line {
    Line::State(key.to_owned(), value.to_owned())
}

fn micros(start: Instant) -> String {
    start.elapsed().as_micros().to_string()
}

/// FNV-1a, to compare masks between runs.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

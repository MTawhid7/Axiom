//! `cargo run -p codegen [-- --check | --list]`: regenerates Axiom's
//! generated code, or checks that the committed code is fresh (ES19).

#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "ES14: a tool reports to its terminal"
)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p codegen [-- --check | --list | --help]
  (no option)  regenerate every output that differs from its committed file
  --check      regenerate in memory; fail if a committed output differs or is missing
  --list       name each generator with its inputs and outputs";

fn main() -> ExitCode {
    // The repository's root: this crate is crates/codegen.
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let generators = codegen::registry::generators();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => match codegen::write(&root, &generators) {
            Ok(written) => {
                for path in &written {
                    println!("wrote {}", path.display());
                }
                println!(
                    "codegen: {} generator(s), {} file(s) written",
                    generators.len(),
                    written.len()
                );
                ExitCode::SUCCESS
            }
            Err(error) => fail(&error),
        },
        ["--check"] => match codegen::check(&root, &generators) {
            Ok(stale) if stale.is_empty() => {
                println!(
                    "codegen --check: {} generator(s), every output fresh",
                    generators.len()
                );
                ExitCode::SUCCESS
            }
            Ok(stale) => {
                for output in &stale {
                    let why = if output.missing { "missing" } else { "differs" };
                    eprintln!(
                        "stale: {} ({why}); run `cargo run -p codegen`",
                        output.path.display()
                    );
                }
                ExitCode::FAILURE
            }
            Err(error) => fail(&error),
        },
        ["--list"] => match codegen::list(&root, &generators) {
            Ok(lines) => {
                for line in &lines {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(error) => fail(&error),
        },
        ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn fail(error: &codegen::Error) -> ExitCode {
    eprintln!("codegen: {error}");
    ExitCode::FAILURE
}

//! Command-line entry points: the `cargo bench` target and the `s3-probe` and
//! `s3-count` binaries.
//!
//! ```text
//! cargo bench -p s3-bench [-- [--filter <text>] [--out <file.tsv>]]
//!
//! s3-probe bench [--quick] [--filter <text>] [--out <file.tsv>]
//! s3-probe aggregate <run.tsv>...
//! s3-probe roots
//! s3-probe profile <case> <seconds>
//! s3-probe sizes
//! s3-probe footprint <none|cell|raw|slice> <roots>
//! s3-count [--test]
//! ```
//!
//! - `roots` prints the root stacks' unit costs, like `s1-probe roots`.
//! - `footprint` pushes `<roots>` values onto a stack of the default capacity
//!   and exits. Under `/usr/bin/time -l`, its peak memory footprint against
//!   `none` shows how much of the stack the system committed.
//! - `profile` loops one case ([`suites::PROFILE_CASES`]) for a sampling
//!   profiler.
//! - `s3-count`, built with the `count` feature, prints instruction, call and
//!   root-operation counts for every workload and technique (S3-Q2).

use core::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::process::ExitCode;
use std::time::Duration;

use s3_gc::count::Context;
use s3_vm::workloads::Size;

use crate::harness::{Aggregated, Measurement, aggregate, format_case};
use crate::suites::{self, Options};

type CliResult = Result<(), String>;

fn exit(result: CliResult) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The `cargo bench` target. Cargo passes `--bench`; without it (as under
/// `cargo test --benches`), this runs the quick, verifying pass.
pub fn bench_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let quick = !args.iter().any(|a| a == "--bench");
    exit(bench(&args, quick))
}

/// The `s3-probe` binary.
pub fn probe_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        return exit(Err(usage()));
    };
    exit(match command.as_str() {
        "bench" => bench(rest, false),
        "aggregate" => aggregate_files(rest),
        "roots" => {
            roots();
            Ok(())
        }
        "profile" => profile(rest),
        "sizes" => {
            sizes();
            Ok(())
        }
        "footprint" => footprint(rest),
        _ => Err(usage()),
    })
}

/// The `s3-count` binary.
pub fn count_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let size = if args.iter().any(|a| a == "--test") {
        Size::Test
    } else {
        Size::Bench
    };
    print!("{}", counts_table(size));
    ExitCode::SUCCESS
}

fn usage() -> String {
    "usage: s3-probe <bench|aggregate|roots|profile|sizes|footprint> … (see the s3_bench::cli docs)"
        .to_owned()
}

fn bench(args: &[String], mut quick: bool) -> CliResult {
    let mut filter = None;
    let mut out = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--bench" => {}
            "--quick" => quick = true,
            "--filter" => filter = Some(it.next().ok_or("--filter needs a value")?.clone()),
            "--out" => {
                let path = it.next().ok_or("--out needs a file")?;
                let mut file =
                    BufWriter::new(File::create(path).map_err(|e| format!("{path}: {e}"))?);
                writeln!(file, "{}", Measurement::TSV_HEADER).map_err(|e| e.to_string())?;
                out = Some(file);
            }
            other if !other.starts_with('-') => filter = Some(other.to_owned()),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let options = Options { quick, filter };
    println!(
        "s3-bench: {} mode, {} build, techniques {:?}",
        if quick {
            "quick (verification)"
        } else {
            "full"
        },
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        s3_vm::Technique::ALL
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>(),
    );
    let mut failed = None;
    suites::run(&options, &mut |row: &[Measurement]| {
        print!("{}", format_case(row));
        if let Some(file) = out.as_mut() {
            for m in row {
                if let Err(e) = writeln!(file, "{}", m.to_tsv()) {
                    failed.get_or_insert(e.to_string());
                }
            }
        }
    });
    if let Some(mut file) = out {
        file.flush().map_err(|e| e.to_string())?;
    }
    failed.map_or(Ok(()), Err)
}

fn read_run(path: &str) -> Result<Vec<Measurement>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(Measurement::from_tsv)
        .collect()
}

/// Print a Markdown summary of several runs: per case, each variant's median
/// of the run medians, half its range, and its ratio to the case's first
/// variant.
fn aggregate_files(paths: &[String]) -> CliResult {
    if paths.is_empty() {
        return Err("usage: aggregate <run.tsv>...".into());
    }
    let runs = paths
        .iter()
        .map(|p| read_run(p))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = aggregate(&runs)?;
    print!("{}", summary(&rows, runs.len()));
    Ok(())
}

fn summary(rows: &[Aggregated], runs: usize) -> String {
    let mut out = format!(
        "# Spike S3 benchmark summary\n\nMedian of {runs} run medians; ± is half the range of the run medians.\n"
    );
    let mut suite = "";
    let mut case = "";
    let mut baseline = Duration::ZERO;
    for row in rows {
        if row.suite != suite {
            suite = &row.suite;
            let _ = write!(
                out,
                "\n## {suite}\n\n| Case | Variant | per {unit} | ± | × first |\n|---|---|---:|---:|---:|\n",
                unit = row.unit
            );
            case = "";
        }
        if row.case != case {
            case = &row.case;
            baseline = row.median();
        }
        let _ = writeln!(
            out,
            "| {} | {} | {:.2} ns | {:.0}% | {:.2} |",
            row.case,
            row.variant,
            row.per_item_ns(),
            100.0 * row.half_spread(),
            row.median().as_secs_f64() / baseline.as_secs_f64(),
        );
    }
    out
}

fn roots() {
    for stack in crate::roots::STACKS {
        let [frame, push, get, run] = crate::roots::measure(stack, Duration::from_millis(300), 11);
        println!(
            "{stack:<14} frame open+close {:.2} ns\tpush {:.2} ns\tget {:.2} ns\tpush-run {:.2} ns",
            crate::roots::per_op_ns(&frame),
            crate::roots::per_op_ns(&push),
            crate::roots::per_op_ns(&get),
            crate::roots::per_op_ns(&run),
        );
    }
}

fn profile(args: &[String]) -> CliResult {
    let [case, seconds] = args else {
        return Err(format!(
            "usage: profile <case> <seconds>; cases: {}",
            suites::PROFILE_CASES
        ));
    };
    let seconds: f64 = seconds
        .parse()
        .map_err(|e| format!("bad seconds {seconds:?}: {e}"))?;
    let iterations = suites::profile(case, seconds)?;
    println!("{case}\titerations={iterations}");
    Ok(())
}

fn footprint(args: &[String]) -> CliResult {
    use s3_gc::{CellStack, DEFAULT_CAPACITY, RawStack, SliceStack, slot_buffer};
    let [stack, roots] = args else {
        return Err("usage: footprint <none|cell|raw|slice> <roots>".to_owned());
    };
    let roots: u64 = roots
        .parse()
        .map_err(|e| format!("bad roots {roots:?}: {e}"))?;
    match stack.as_str() {
        "none" => {}
        "cell" => fill(&CellStack::default(), roots),
        "raw" => fill(&RawStack::default(), roots),
        "slice" => fill(&SliceStack::new(&mut slot_buffer(DEFAULT_CAPACITY)), roots),
        other => return Err(format!("unknown stack {other:?}")),
    }
    Ok(())
}

/// Push `n` values in one frame.
fn fill<S: s3_gc::RootStack>(stack: &S, n: u64) {
    let frame = s3_gc::RootFrame::new(stack);
    for bits in 0..n {
        core::hint::black_box(s3_gc::Value::from_bits(bits).scope(&frame));
    }
    core::hint::black_box(stack.height());
}

fn sizes() {
    use s3_gc::{CellStack, RawStack, RefCellStack, RootStack, SliceStack, Value};
    println!(
        "Value                         {:>4} bytes",
        size_of::<Value>()
    );
    println!(
        "root slot reference: slice {}, raw {}, cell {}, refcell {} bytes",
        size_of::<<SliceStack<'static> as RootStack>::Slot<'static>>(),
        size_of::<<RawStack as RootStack>::Slot<'static>>(),
        size_of::<<CellStack as RootStack>::Slot<'static>>(),
        size_of::<<RefCellStack as RootStack>::Slot<'static>>(),
    );
    println!(
        "Default stack capacity        {} slots ({} MiB reserved)",
        s3_gc::DEFAULT_CAPACITY,
        (s3_gc::DEFAULT_CAPACITY * size_of::<Value>()) >> 20
    );
    println!(
        "Instr                         {:>4} bytes",
        size_of::<s3_vm::Instr>()
    );
    println!(
        "Object (4 slots)              {:>4} bytes",
        size_of::<s3_vm::Object>()
    );
    println!(
        "Array                         {:>4} bytes",
        size_of::<s3_vm::Array>()
    );
    println!(
        "Closure                       {:>4} bytes",
        size_of::<s3_vm::Closure>()
    );
    println!(
        "port NodeData                 {:>4} bytes (S1: {})",
        size_of::<crate::dom::NodeData>(),
        s1_b_dom::dom::NODE_BYTES
    );
}

/// The counts as a Markdown table.
#[must_use]
pub fn counts_table(size: Size) -> String {
    let mut out = String::new();
    if !s3_gc::count::enabled() {
        out.push_str("note: built without the `count` feature, so every count is zero\n\n");
    }
    out.push_str(
        "| Workload | Technique | Instructions | Root ops: bytecode | builtins | hooks | Root ops per instruction | Builtin calls | Native→bytecode calls | Bytecode calls | Coercions | Collections |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for c in suites::counts(size) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {:.5} | {} | {} | {} | {} | {} |",
            c.workload,
            c.technique,
            c.stats.instructions(),
            c.in_context(Context::Interpreter),
            c.in_context(Context::Builtin),
            c.in_context(Context::Coercion),
            c.roots_per_instruction(),
            c.stats.builtin_calls,
            c.stats.native_calls,
            c.stats.bytecode_calls,
            c.stats.coercions,
            c.collections,
        );
    }
    out.push_str("\nRoot operations by kind (frames / pushes / reads), all contexts:\n\n");
    for c in suites::counts(size)
        .iter()
        .filter(|c| c.technique == s3_vm::Technique::Match)
    {
        let sum = |a: [u64; 3]| a.iter().sum::<u64>();
        let _ = writeln!(
            out,
            "- {}: {} / {} / {}",
            c.workload,
            sum(c.roots.frames),
            sum(c.roots.pushes),
            sum(c.roots.reads),
        );
    }
    out
}

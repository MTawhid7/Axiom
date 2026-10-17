//! Command-line entry points: the `cargo bench` target and the `s1-probe`
//! binary.
//!
//! ```text
//! cargo bench -p s1-bench [-- [--filter <text>] [--out <file.tsv>]]
//!
//! s1-probe bench [--quick] [--filter <text>] [--out <file.tsv>]
//! s1-probe aggregate <run.tsv>...
//! s1-probe gen <tree|listeners|churn|pressure> <small|medium|large>
//! s1-probe gen tree-keep <elements>
//! s1-probe run <a|b|none> <script file>
//! s1-probe profile <a|b> <case> <seconds>
//! s1-probe roots
//! s1-probe sizes
//! ```
//!
//! - `gen` writes a script ([`script_io`](crate::script_io)) to stdout.
//!   `tree-keep` is the `tree` generator with ids kept, so every node stays
//!   live: the input for bytes-per-node measurements.
//! - `run` executes a script file through one pattern's driver, checks the
//!   outcome, and prints one line. `none` only parses it, which gives the
//!   baseline to subtract when the process is measured by `/usr/bin/time -l`.
//! - `profile` loops one case (see [`suites::PROFILE_CASES`]) for a sampling
//!   profiler.
//! - `roots` measures the unit costs of Pattern B's rooting primitives
//!   ([`roots`](crate::roots)).

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write as _};
use std::process::ExitCode;
use std::time::Instant;

use s1_scenarios::generate::{self, TreeParams};
use s1_scenarios::{Size, Workload};

use crate::dom::Dom;
use crate::harness::{Aggregated, Human, Measurement, Pattern, aggregate, format_pair, ratio};
use crate::suites::{self, Options};
use crate::{PatternA, PatternB, script_io};

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

/// The `cargo bench` target. Cargo passes `--bench`. Without it (as under
/// `cargo test --benches`), this runs the quick, verifying pass.
pub fn bench_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let quick = !args.iter().any(|a| a == "--bench");
    exit(bench(&args, quick))
}

/// The `s1-probe` binary.
pub fn probe_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        return exit(Err(usage()));
    };
    exit(match command.as_str() {
        "bench" => bench(rest, false),
        "aggregate" => aggregate_files(rest),
        "gen" => gen_script(rest),
        "run" => run_script(rest),
        "profile" => profile(rest),
        "roots" => {
            let costs = crate::roots::measure(std::time::Duration::from_millis(300), 11);
            println!(
                "frame open+close {:.2} ns\tscope (push) {:.2} ns\tScoped::get {:.2} ns",
                costs.frame_ns, costs.push_ns, costs.get_ns
            );
            Ok(())
        }
        "sizes" => {
            sizes();
            Ok(())
        }
        _ => Err(usage()),
    })
}

fn usage() -> String {
    "usage: s1-probe <bench|aggregate|gen|run|profile|roots|sizes> … (see the s1_bench::cli docs)"
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
        "s1-bench: {} mode, {} build{}",
        if quick {
            "quick (verification)"
        } else {
            "full"
        },
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "optimized"
        },
        options
            .filter
            .as_deref()
            .map_or(String::new(), |f| format!(", filter {f:?}")),
    );
    let mut write_error = None;
    suites::run(&options, &mut |pair| {
        print!("{}", format_pair(&pair[0], &pair[1]));
        if let Some(file) = out.as_mut() {
            for m in &pair {
                if let Err(e) = writeln!(file, "{}", m.to_tsv()) {
                    write_error.get_or_insert(e.to_string());
                }
            }
        }
    });
    if let Some(mut file) = out {
        file.flush().map_err(|e| e.to_string())?;
    }
    write_error.map_or(Ok(()), Err)
}

fn read_tsv(path: &str) -> Result<Vec<Measurement>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| Measurement::from_tsv(line).map_err(|e| format!("{path}: {e}")))
        .collect()
}

/// Print a Markdown summary of several runs: per case, the median of the run
/// medians for each pattern, the run-to-run spread, and the B/A ratio.
fn aggregate_files(paths: &[String]) -> CliResult {
    if paths.is_empty() {
        return Err("aggregate needs at least one results file".to_owned());
    }
    let runs = paths
        .iter()
        .map(|p| read_tsv(p))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = aggregate(&runs)?;
    println!(
        "Runs: {}. Each cell is the median of the per-run medians; the spread is the range of the run medians.\n",
        runs.len()
    );
    println!("| Suite | Case | Items per call | A | B | B/A | A per item | B per item |");
    println!("|---|---|---|---|---|---|---|---|");
    let pairs = pair_rows(&rows)?;
    for (a, b) in &pairs {
        println!(
            "| {} | {} | {} {} | {} (±{:.0}%) | {} (±{:.0}%) | {:.2} | {:.1} ns | {:.1} ns |",
            a.suite,
            a.case,
            a.items,
            a.unit,
            Human(a.median()),
            a.spread() * 50.0,
            Human(b.median()),
            b.spread() * 50.0,
            ratio(b.median(), a.median()),
            a.per_item_ns(),
            b.per_item_ns(),
        );
    }
    println!("\nExtra facts (first run):\n");
    for (a, b) in &pairs {
        if a.extra.is_empty() && b.extra.is_empty() {
            continue;
        }
        let show = |r: &Aggregated| {
            r.extra
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!("- `{}/{}`", a.suite, a.case);
        println!("  - A: {}", show(a));
        println!("  - B: {}", show(b));
    }
    Ok(())
}

fn pair_rows(rows: &[Aggregated]) -> Result<Vec<(&Aggregated, &Aggregated)>, String> {
    let mut pairs = Vec::new();
    for a in rows.iter().filter(|r| r.pattern == Pattern::A) {
        let b = rows
            .iter()
            .find(|r| r.pattern == Pattern::B && r.suite == a.suite && r.case == a.case)
            .ok_or_else(|| format!("{}/{} has no pattern B result", a.suite, a.case))?;
        pairs.push((a, b));
    }
    Ok(pairs)
}

fn gen_script(args: &[String]) -> CliResult {
    let [workload, size] = args else {
        return Err("usage: gen <workload> <size>  |  gen tree-keep <elements>".to_owned());
    };
    let generated = if workload == "tree-keep" {
        let elements = size
            .parse()
            .map_err(|e| format!("bad element count {size:?}: {e}"))?;
        let params = TreeParams {
            elements,
            forget_ids: false,
            ..TreeParams::preset(Size::Large)
        };
        generate::tree(&params, suites::SEED)
    } else {
        let workload = Workload::ALL
            .into_iter()
            .find(|w| w.name() == workload)
            .ok_or_else(|| format!("unknown workload {workload:?}"))?;
        let size = match size.as_str() {
            "small" => Size::Small,
            "medium" => Size::Medium,
            "large" => Size::Large,
            other => return Err(format!("unknown size {other:?}")),
        };
        workload.generate(size, suites::SEED)
    };
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    script_io::write(&mut out, &generated.script, &generated.expected)
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())
}

fn run_script(args: &[String]) -> CliResult {
    let [pattern, path] = args else {
        return Err("usage: run <a|b|none> <script file>".to_owned());
    };
    let file = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let parsed = script_io::read(BufReader::new(file)).map_err(|e| format!("{path}: {e}"))?;
    let script = &parsed.script;
    if pattern == "none" {
        println!("none\tops={}", script.ops.len());
        return Ok(());
    }
    let pattern = Pattern::parse(pattern).ok_or_else(|| format!("unknown pattern {pattern:?}"))?;
    let start = Instant::now();
    let run = match pattern {
        Pattern::A => PatternA::run_script(script),
        Pattern::B => PatternB::run_script(script),
    };
    let elapsed = start.elapsed();
    if run.outcome.digest() != parsed.expected_digest {
        return Err(format!(
            "{path}: pattern {pattern}'s outcome differs from the expected one"
        ));
    }
    let heap = &run.heap;
    println!(
        "{pattern}\tops={}\twall_ms={:.2}\tcollections={}\tmax_pause_us={:.1}\tpeak_live_nodes={}\tpeak_objects={}\tpeak_heap_bytes={}\toutcome=ok",
        script.ops.len(),
        elapsed.as_secs_f64() * 1e3,
        heap.collections,
        heap.max_pause.as_secs_f64() * 1e6,
        run.peak_live_nodes,
        heap.peak_objects,
        heap.peak_bytes,
    );
    Ok(())
}

fn profile(args: &[String]) -> CliResult {
    let [pattern, case, seconds] = args else {
        return Err(format!(
            "usage: profile <a|b> <case> <seconds>; cases: {:?}",
            suites::PROFILE_CASES
        ));
    };
    let seconds: f64 = seconds
        .parse()
        .map_err(|e| format!("bad seconds {seconds:?}: {e}"))?;
    let pattern = Pattern::parse(pattern).ok_or_else(|| format!("unknown pattern {pattern:?}"))?;
    let iterations = match pattern {
        Pattern::A => suites::profile::<PatternA>(case, seconds),
        Pattern::B => suites::profile::<PatternB>(case, seconds),
    }?;
    println!("{pattern}\t{case}\titerations={iterations}");
    Ok(())
}

/// Object sizes, which explain the per-node memory results.
fn sizes() {
    use s1_a_dom::dom as a;
    use s1_b_dom::dom as b;

    let header = s1_a_gc::BOX_HEADER_BYTES;
    println!("Pattern A (one allocation per box: {header}-byte header + value)");
    for (name, value) in [
        ("Node", size_of::<a::Node<'static>>()),
        ("Listener", size_of::<a::Listener<'static>>()),
        ("Closure", size_of::<a::Closure<'static>>()),
    ] {
        println!(
            "  {name:<9} value {value:>4} B   box {:>4} B",
            header + value
        );
    }
    println!("  Gc<T>: {} B", size_of::<a::NodeRef<'static>>());
    println!("Pattern B (dense typed arenas; no per-object header)");
    for (name, value) in [
        ("NodeData", size_of::<b::NodeData>()),
        ("ListenerData", size_of::<b::ListenerData>()),
        ("ClosureData", size_of::<b::ClosureData>()),
    ] {
        println!("  {name:<12} {value:>4} B");
    }
    println!(
        "  Handle: {} B, Option<Handle>: {} B{}",
        size_of::<b::NodeRef<'static>>(),
        size_of::<Option<b::NodeRef<'static>>>(),
        if cfg!(debug_assertions) {
            " (debug build: includes the epoch)"
        } else {
            ""
        },
    );
}

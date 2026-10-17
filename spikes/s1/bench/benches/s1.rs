//! `cargo bench -p s1-bench [-- [--filter <text>] [--out <file.tsv>]]`
//!
//! Runs every suite in `s1_bench::suites`. Without cargo's `--bench` flag (as
//! under `cargo test --benches`), it runs the quick, verifying pass instead.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    s1_bench::cli::bench_main()
}

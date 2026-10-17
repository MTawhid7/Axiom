//! `cargo bench -p s3-bench [-- [--filter <text>] [--out <file.tsv>]]`
//!
//! Runs every suite in `s3_bench::suites`. Without cargo's `--bench` flag (as
//! under `cargo test --benches`), it runs the quick, verifying pass instead.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    s3_bench::cli::bench_main()
}

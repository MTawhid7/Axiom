//! Measurement probes for spike S1: benchmark runs with a stable binary path,
//! result aggregation, script files for per-process RSS measurement, profiling
//! loops, and object sizes. See `s1_bench::cli`.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    s1_bench::cli::probe_main()
}

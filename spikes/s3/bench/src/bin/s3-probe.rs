//! Measurement probes for spike S3: benchmark runs with a stable binary path,
//! result aggregation, rooting unit costs, profiling loops and object sizes.
//! See `s3_bench::cli`.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    s3_bench::cli::probe_main()
}

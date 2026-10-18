//! Instruction, call and root-operation counts for spike S3-Q2. Built only
//! with the `count` feature. See `s3_bench::cli`.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    s3_bench::cli::count_main()
}

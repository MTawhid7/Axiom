//! # Spike S3: benchmarks and probes
//!
//! Measures what spike S3 asks (Step 4 tracker §10), on the M1:
//! - **S3-Q1:** the root stacks' unit costs ([`roots`]), and S1's dispatch
//!   case on S1's Pattern B and on a port of it to `s3-gc` ([`dom`],
//!   [`port`]) for each stack.
//! - **S3-Q2 and S3-Q3:** the VM's workloads on every dispatch technique and
//!   root stack, plus counts from a `count` build ([`suites::counts`]) and
//!   loops for `samply` profiles ([`suites::profile`]).
//! - **S3-Q4:** S1's traversal cases on four node layouts ([`layout`]).
//! - **S3-Q5:** writes through generated setters, without and with a
//!   card-marking barrier ([`dom`]'s setters; the `mutate` and `barrier`
//!   suites).
//!
//! S1's own crates are dependencies, so its code runs in the same process as
//! the baseline. Every suite verifies its inputs and results before timing.
#![forbid(unsafe_code)]

pub mod cli;
pub mod dom;
pub mod harness;
pub mod layout;
pub mod port;
pub mod roots;
pub mod suites;

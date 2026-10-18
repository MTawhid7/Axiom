//! # Spike S3: an interpreter-shaped workload over Pattern B
//!
//! A register-machine bytecode interpreter whose heap is `s3-gc`, built to
//! answer two of spike S3's questions (Step 4 tracker §10):
//! - **S3-Q2, rooting traffic.** The interpreter traces its register file as
//!   one region, so executing bytecode roots nothing value by value. Builtins
//!   (`forEach`, `map`, `sortBy`) and the `valueOf`-style coercion hook call
//!   back into bytecode while holding values, which they root on the root
//!   stack in Pattern B's style. With the `count` feature, every root
//!   operation is counted against its context.
//! - **S3-Q3, dispatch.** Four techniques run the same operations
//!   ([`Technique`]): a `match` loop, a table of handlers, handlers that
//!   return the next handler, and, on nightly, explicit tail calls.
//!
//! Correctness comes first: [`reference`] is an independent evaluator of the
//! same bytecode, and [`workloads`] carry native models of their results.
//! The oracle tests compare every technique on every root stack, at normal
//! pacing and when collecting at every allocation.
//!
//! The machine is described in [`bytecode`]. It is JavaScript-shaped, not
//! JavaScript.
#![cfg_attr(
    feature = "nightly-tail",
    feature(explicit_tail_calls),
    allow(incomplete_features)
)]
#![forbid(unsafe_code)]

pub mod builder;
pub mod builtins;
pub mod bytecode;
pub mod digest;
mod dispatch;
pub mod heap;
mod machine;
pub mod reference;
mod stats;
pub mod workloads;

pub use builder::{FnBuilder, FuncId, Label, ProgramBuilder, Reg};
pub use builtins::Builtin;
pub use bytecode::{Function, Instr, Program, op};
pub use dispatch::Technique;
pub use heap::{Array, Closure, Object, VmHeap};
pub use machine::{Loaded, MAX_FRAMES, MAX_NATIVE_DEPTH, REGISTERS, Vm, VmError};
pub use stats::Stats;

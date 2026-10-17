//! # Spike S1: benchmarks and measurement probes (CP7)
//!
//! Compares Pattern A (`s1-a-dom` on `s1-a-gc`) with Pattern B (`s1-b-dom` on
//! `s1-b-gc`) on identical inputs. It answers S1-Q3 (runtime cost) and supplies
//! the data for S1-Q4's memory side.
//!
//! ## Suites, and what each one attributes
//!
//! The patterns differ in two confounded ways (tracker §4.4): the rooting
//! discipline, and the storage representation (pointers to individually
//! allocated boxes against `u32` indices into typed arenas). Each suite is
//! chosen so that one factor dominates it.
//!
//! | Suite | Timed work | Dominant factor |
//! |---|---|---|
//! | `build` | create and link a generated tree, with attributes | allocation: a `malloc` per box (A) against a `Vec` push (B) |
//! | `traverse` | a pre-order walk over the links, reading each node | representation: pointer dereference against bounds-checked index; node size and density |
//! | `mutate` | random moves, inserts, removes and attribute writes | the write path: `Gc::write` + `unlock!` (A) against plain assignment through an index (B) |
//! | `dispatch` | events through a listener-heavy tree, non-allocating listeners | the rooting discipline: a resumable state machine in the root (A) against root-stack frames and `Scoped::get` (B) |
//! | `collect` | one full collection, against live-set size and with 50% interleaved garbage | the collector: mark–sweep (A) against mark–compact (B) |
//! | `scenario` | each CP2 workload, end to end, through the verified script drivers | everything together, with default pacing |
//!
//! No GC runs inside the `build`, `traverse`, `mutate` and `dispatch` suites
//! (pacing is disabled), so they measure the mutator alone. The `collect` and
//! `scenario` suites measure the collector.
//!
//! ## Verification before timing
//!
//! Inputs come from [`plan`], which validates every op against the reference
//! interpreter and records the expected results. Before a case is timed, both
//! patterns must reproduce them (tree digests, dispatch logs, live-object
//! counts, whole-scenario outcomes) and agree with each other on the
//! traversal fold. A benchmark that silently did less work than its rival
//! would otherwise look fast.
#![forbid(unsafe_code)]

pub mod cli;
pub mod dom;
pub mod harness;
pub mod pattern_a;
pub mod pattern_b;
pub mod plan;
pub mod roots;
pub mod script_io;
pub mod suites;

pub use dom::{Config, Dom, Fold, HeapStats, ScenarioRun};
pub use harness::{Measurement, Pattern, Samples, Sampling};
pub use pattern_a::PatternA;
pub use pattern_b::PatternB;

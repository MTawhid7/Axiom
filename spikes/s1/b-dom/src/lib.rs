//! # Spike S1, Pattern B: the DOM on a Nova-style heap
//!
//! This crate executes [`s1_scenarios`] scripts on [`s1_b_gc`], and must
//! reproduce the reference interpreter's outcome exactly. It answers S1-Q1 and
//! S1-Q2 for Pattern B.
//!
//! **Containment.** Neither this crate nor its whole dependency chain contains
//! any `unsafe`.
//!
//! **Ceremony.** Writes are plain field assignments (`heap.nodes[n].parent =
//! …`), with no barriers. The ceremony moves to handle lifetimes instead:
//! - every function that may collect takes a `GcScope`;
//! - handles are `bind`-ed to `gc.nogc()` for compile-time protection;
//! - handles are `unbind`-ed to be stored or passed on;
//! - handles are rooted in a `RootFrame` to survive a safepoint.
//!
//! Collection can happen at *any* allocation, so dispatch is an ordinary
//! function. It roots its event path and listener snapshots instead of
//! becoming a state machine.
#![forbid(unsafe_code)]

pub mod dom;
pub mod driver;

pub use driver::{RunReport, run, run_with};

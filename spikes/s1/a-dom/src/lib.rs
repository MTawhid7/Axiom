//! # Spike S1, Pattern A: the DOM on a gc-arena-style heap
//!
//! This crate executes [`s1_scenarios`] scripts on [`s1_a_gc`], and must
//! reproduce the reference interpreter's outcome exactly. It answers S1-Q1 and
//! S1-Q2 for Pattern A, which are about the code an engine author writes
//! *against* the GC:
//! - **Containment.** This crate is `#![forbid(unsafe_code)]`. The only
//!   `unsafe` in its expanded code comes from two audited macro templates in
//!   `s1-a-gc`: `#[derive(Trace)]` and `unlock!`.
//! - **Ceremony.**
//!   - Tree links live in `Lock`s, and every write goes through
//!     `Gc::write(mc, node)` plus `unlock!`.
//!   - Data that holds no `Gc` (attributes, flags) uses a plain `RefCell` or
//!     `Cell`, with no barrier.
//!   - Collection can only happen *between* mutation scopes, so an event
//!     dispatch (which runs arbitrary listener code) is a resumable state
//!     machine stored in the arena root ([`dispatch`]). The driver yields to
//!     the collector whenever the heap asks for it.
#![forbid(unsafe_code)]

pub mod dispatch;
pub mod dom;
pub mod driver;

pub use driver::{RunReport, run, run_with};

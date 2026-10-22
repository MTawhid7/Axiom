//! Axiom's recorder of spans, events, metrics and logs.
//!
//! Every process records what it does and what it costs through this crate,
//! into per-thread buffers with one writer each, and the browser's
//! `diagnostics` crate collects the records (`docs/architecture/observability.md`,
//! OB5–OB9).
//!
//! This is the crate's charter in its first form (ES15): M1-CP2 builds the
//! recorder in process, and expands this charter with it, before its code.

#![forbid(unsafe_code)]
// Panic-free (ES10; the crate map): its decoder reads records from less
// trusted processes inside the browser (OB24), and every process records
// through it, so a panic here would abort any of them.
#![deny(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

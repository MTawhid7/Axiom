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

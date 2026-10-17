//! The platform layer: CoreText and CoreGraphics calls. It is the only place
//! in this crate where `unsafe` is allowed (`tools/unsafe-policy.txt`).
//!
//! The rasterizer is S2's (`spikes/s2/window/src/platform/text.rs`), and its
//! `unsafe` blocks rest on the same two invariants, renamed for this crate:
//!
//! - **T1, CPU memory lent to a framework.** Every raw pointer passed to
//!   CoreGraphics or CoreText points into a live Rust allocation of at least
//!   the declared length. That allocation outlives every use the framework
//!   can make of the pointer: the call itself, or, for a bitmap context, the
//!   context's lifetime, which ends before the buffer is read.
//! - **T2, framework constants.** Extern statics such as
//!   `kCTFontAttributeName` are initialized when their framework loads (they
//!   are linked here), and are never written.
#![allow(unsafe_code)]

pub(crate) mod text;

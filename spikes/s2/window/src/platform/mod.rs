//! The platform layer: every call into AppKit, QuartzCore, Metal, CoreText and
//! CoreGraphics. It is the only place in this crate where `unsafe` is allowed
//! (`tools/unsafe-policy.txt`).
//!
//! objc2's bindings are safe wherever Apple's API can't be misused from
//! safe Rust. They mark a call `unsafe` when it takes a raw pointer, an index
//! with an unchecked bound, or GPU work that can read out of bounds. Every
//! `unsafe` block here cites one of these invariants:
//!
//! - **P1, window ownership.** A window is created with
//!   `releasedWhenClosed = false` before anything else can close it, so
//!   closing it never frees an object that Rust still holds a `Retained` to
//!   (objc2's documented requirement for `-[NSWindow initWithContentRect:…]`).
//! - **P2, CPU memory lent to a framework.** Every raw pointer passed to
//!   CoreGraphics, CoreText or Metal points into a live Rust allocation of at
//!   least the declared length. That allocation outlives every use the
//!   framework can make of the pointer: the call itself, or, for a bitmap
//!   context, the context's lifetime, which ends before the buffer is read.
//! - **P3, GPU memory read on the CPU.** A shared `MTLBuffer` is read only
//!   after the command buffer that wrote it has completed
//!   (`waitUntilCompleted`), and only within its length.
//! - **P4, shader resource bounds.** Each draw binds exactly what
//!   `shader.metal` reads: one `Quad` (`size_of::<Quad>()` bytes) at vertex
//!   buffer 0, and a texture at fragment texture 0. It draws 4 vertices, which
//!   the vertex shader turns into the quad's corners without reading any
//!   vertex buffer. Blits copy regions that lie inside both their source and
//!   their destination.
//! - **P5, indices.** Indexed subscripts stay below Metal's documented limits
//!   (index 0 of 8 color attachments; buffer and texture slot 0 of 31).
//! - **P6, framework constants.** Extern statics such as
//!   `NSDefaultRunLoopMode` and `kCTFontAttributeName` are initialized when
//!   their framework loads (they are linked here), and are never written.
#![allow(unsafe_code)]

pub(crate) mod app;
pub(crate) mod gpu;
pub(crate) mod text;

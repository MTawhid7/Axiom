//! The platform layer: ImageIO and CoreGraphics calls. It is the only place
//! in this crate where `unsafe` is allowed (`tools/unsafe-policy.txt`). Every
//! `unsafe` block cites one of these invariants:
//!
//! - **I1, memory lent to a framework.** Every pointer passed to CoreGraphics
//!   points into a live Rust allocation of at least the length the call may
//!   use. A bitmap context is dropped before its buffer is read or moved.
//! - **I2, framework constants.** Extern statics such as
//!   `kCGImageSourceShouldCacheImmediately` are initialized when their
//!   framework loads, and are never written.
//! - **I3, documented option types.** The options dictionary passed to
//!   ImageIO maps the `CFString` keys ImageIO documents to `CFBoolean`
//!   values, the type it documents for them.
//! - **I4, live objects.** Every CoreFoundation, ImageIO or CoreGraphics
//!   object a call receives is live and of the type the call declares: it is
//!   held in a `CFRetained`, or borrowed from one, for the whole call.
#![allow(unsafe_code)]

pub(crate) mod imageio;

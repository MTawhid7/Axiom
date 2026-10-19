//! The platform layer: CoreText, CoreGraphics and CoreFoundation calls, and
//! read-only file mappings. It is the only place in this crate where `unsafe`
//! is allowed (`tools/unsafe-policy.txt`). S4's text spike
//! (`spikes/s4/text/src/platform/`) is the model. Every `unsafe` block cites
//! one of these invariants:
//!
//! - **C1, memory lent to a framework.** Every pointer passed to CoreText or
//!   CoreGraphics points into a live Rust allocation of at least the length the
//!   call may use. That allocation outlives every use the framework can make of
//!   it: the call itself, or a bitmap context's lifetime, which ends before the
//!   buffer is read.
//! - **C2, framework constants.** Extern statics such as `kCTFontAttributeName`
//!   are initialized when their framework loads (they are linked here), and
//!   are never written.
//! - **C3, out-buffers sized by the run.** A buffer filled from a `CTRun`
//!   (glyphs, positions, advances, string indices) holds exactly the run's
//!   glyph count, read from the same run, which is immutable.
//! - **C4, documented element types.** A CoreFoundation collection is viewed
//!   with an element type only where CoreText documents it: a line's glyph runs
//!   are CF objects (`CTRun`s); a run's attributes and a font's variation are
//!   dictionaries of CF objects; a font's available tables are unboxed
//!   `CTFontTableTag` integers, not objects. Each element taken as a CF object
//!   is then downcast with a checked type test.
//! - **C5, read-only mappings.** A file mapping comes from one successful
//!   `mmap` of a whole regular file with `PROT_READ` and `MAP_PRIVATE`. It is
//!   only read, unmapped once by its only owner, and lent out only as slices
//!   that borrow that owner, so it may be shared between threads. The files
//!   mapped are system fonts on the read-only system volume, so they cannot
//!   shrink underneath the mapping.
//! - **C6, live objects and documented arguments.** Every CoreFoundation,
//!   CoreText or CoreGraphics object a call receives is live and of the type
//!   the call declares: it is held in a `CFRetained`, or borrowed from one, for
//!   the whole call. Every other argument is one the call documents: a null
//!   matrix is the identity transform, null attributes keep a font's own, a
//!   range lies within its string, an attribute key is one of the names
//!   CoreText documents, and an attribute value has the type documented for
//!   its key.
#![allow(unsafe_code)]

pub(crate) mod coretext;
pub(crate) mod mapping;

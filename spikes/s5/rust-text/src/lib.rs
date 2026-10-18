//! # Spike S5 (Step 5): the Rust-only text path
//!
//! The path `docs/architecture/text-and-fonts.md`
//! specifies for every web font (TX7) and, if S5-Q2 chooses it, for system
//! fonts too (TX12): no C parser ever sees the font's bytes.
//! - [`woff`] and [`woff2`] decode the containers with our own code, over
//!   `miniz_oxide` and `brotli-decompressor`.
//! - [`validate`] checks that the table directory and required tables parse
//!   with `read-fonts`, within [`Limits`].
//! - [`shape`] shapes with HarfRust.
//! - [`raster`] fills skrifa's outlines with zeno or `vello_cpu`.
//! - [`webfont`] runs the whole path on one file (S5-Q3); the `s5-webfont`
//!   binary runs it inside S4's renderer profile.
//! - [`mutate`] makes deterministic malformed variants of valid fonts (S5-Q3).
//!
//! No `unsafe` here, and no Apple framework: the `s5-webfont` binary links
//! neither CoreText nor CoreGraphics, which `otool -L` shows. The CoreText
//! side of S5's comparisons is `s5-text`.
#![forbid(unsafe_code)]

pub mod bytes;
pub mod mutate;
pub mod raster;
pub mod sfnt;
pub mod shape;
pub mod testfont;
pub mod validate;
pub mod webfont;
pub mod woff;
pub mod woff2;

/// Limits on hostile inputs (TX7: "it stays within limits on size, glyph count
/// and table sizes, set at implementation"). These are the spike's values;
/// the implementation sets the engine's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The largest container accepted.
    pub max_file: usize,
    /// The largest decoded font accepted.
    pub max_sfnt: usize,
    /// The most tables one font may have.
    pub max_tables: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file: 30 << 20,
            max_sfnt: 30 << 20,
            max_tables: 128,
        }
    }
}

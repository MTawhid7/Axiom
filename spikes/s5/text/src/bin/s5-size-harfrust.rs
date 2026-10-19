//! # Spike S5, S5-Q4: what HarfRust and read-fonts add to an executable
//!
//! Usage: `s5-size-harfrust FONT TEXT` shapes `TEXT` in the font and prints
//! the glyph count.
#![forbid(unsafe_code)]

use std::sync::Arc;

use s5_rust_text::shape::{self, Params};

fn main() {
    let mut args = std::env::args().skip(1);
    let data = std::fs::read(args.next().unwrap_or_default()).unwrap_or_default();
    let text = args.next().unwrap_or_default();
    let glyphs = shape::font(Arc::new(data), 0, &[])
        .and_then(|font| {
            shape::shape(&font, &text, 0..shape::utf16_len(&text), &Params::default()).ok()
        })
        .map_or(0, |g| g.len());
    println!("{glyphs}");
}

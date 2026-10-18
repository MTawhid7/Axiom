//! # Spike S5, S5-Q4: what skrifa and zeno add to an executable
//!
//! Usage: `s5-size-raster FONT GLYPH` draws the glyph at 16 px and prints its
//! ink.
#![forbid(unsafe_code)]

use s5_rust_text::raster::{self, Filler};
use skrifa::GlyphId;
use skrifa::instance::LocationRef;

fn main() {
    let mut args = std::env::args().skip(1);
    let data = std::fs::read(args.next().unwrap_or_default()).unwrap_or_default();
    let glyph: u32 = args.next().and_then(|g| g.parse().ok()).unwrap_or(0);
    let ink = skrifa::FontRef::new(&data)
        .ok()
        .and_then(|font| {
            raster::outline(&font, GlyphId::new(glyph), 16.0, LocationRef::default()).ok()
        })
        .and_then(|path| raster::fill(&path, (8.0, 24.0), 32, 32, Filler::Zeno).ok())
        .map_or(0, |canvas| canvas.ink());
    println!("{ink}");
}

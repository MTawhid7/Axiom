//! Text for the synthetic pages: a glyph atlas of printable ASCII in
//! Helvetica at three sizes, rasterized by S5's Rust path (`skrifa` and
//! `zeno`), and a line layout that turns strings into glyph quads (TX13's
//! coverage atlas, as the renderer would send it).

use std::collections::HashMap;

use s5_rust_text::raster::{self, Filler};
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::platform::gpu::Glyph;

/// The font: Helvetica, from the system's fonts (read-only).
const FONT_PATH: &str = "/System/Library/Fonts/Helvetica.ttc";
/// The atlas's side, in pixels.
pub(crate) const ATLAS_SIDE: usize = 1024;
/// Spacing between glyphs in the atlas, against sampling neighbours.
const PAD: usize = 1;

/// One rasterized glyph: its place in the atlas, and how to position it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Entry {
    /// Pixel rectangle in the atlas.
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    /// Offset of the mask's top-left corner from the pen, y down.
    left: f32,
    top: f32,
    advance: f32,
}

/// An 8-bit coverage atlas of glyphs at fixed pixel sizes.
#[derive(Debug)]
pub(crate) struct Atlas {
    pub(crate) pixels: Vec<u8>,
    entries: HashMap<(char, u32), Entry>,
    /// Line height for each size.
    lines: HashMap<u32, f32>,
}

impl Atlas {
    /// Rasterize printable ASCII at each of `sizes` (pixels per em).
    pub(crate) fn new(sizes: &[u32]) -> Result<Self, String> {
        let data = std::fs::read(FONT_PATH).map_err(|e| format!("{FONT_PATH}: {e}"))?;
        let font = FontRef::from_index(&data, 0).map_err(|e| format!("{FONT_PATH}: {e:?}"))?;
        let charmap = font.charmap();
        let mut pixels = vec![0_u8; ATLAS_SIDE * ATLAS_SIDE];
        let mut entries = HashMap::new();
        let mut lines = HashMap::new();
        let (mut x, mut y, mut shelf) = (PAD, PAD, 0_usize);
        for &size in sizes {
            let px = f32::from(u16::try_from(size).unwrap_or(u16::MAX));
            let metrics = font.metrics(Size::new(px), LocationRef::default());
            lines.insert(
                size,
                (metrics.ascent - metrics.descent + metrics.leading).ceil(),
            );
            let glyph_metrics = font.glyph_metrics(Size::new(px), LocationRef::default());
            for ch in ' '..='~' {
                let Some(gid) = charmap.map(ch) else {
                    continue;
                };
                let advance = glyph_metrics.advance_width(gid).unwrap_or(px * 0.5);
                let (mask, width, height, left, top) = rasterize(&font, gid, px)?;
                if x + width + PAD > ATLAS_SIDE {
                    (x, y, shelf) = (PAD, y + shelf + PAD, 0);
                }
                if y + height + PAD > ATLAS_SIDE {
                    return Err("the glyph atlas is full".to_owned());
                }
                for row in 0..height {
                    let to = (y + row) * ATLAS_SIDE + x;
                    pixels[to..to + width].copy_from_slice(&mask[row * width..(row + 1) * width]);
                }
                entries.insert(
                    (ch, size),
                    Entry {
                        x,
                        y,
                        width,
                        height,
                        left,
                        top,
                        advance,
                    },
                );
                x += width + PAD;
                shelf = shelf.max(height);
            }
        }
        Ok(Self {
            pixels,
            entries,
            lines,
        })
    }

    /// The line height at `size`.
    pub(crate) fn line_height(&self, size: u32) -> f32 {
        self.lines
            .get(&size)
            .copied()
            .unwrap_or(1.2 * f32::from(u16::try_from(size).unwrap_or(u16::MAX)))
    }

    /// The advance of `text` at `size`.
    pub(crate) fn width(&self, text: &str, size: u32) -> f32 {
        text.chars()
            .filter_map(|ch| self.entries.get(&(ch, size)))
            .map(|e| e.advance)
            .sum()
    }

    /// Glyph quads for `text` at `size`, its baseline starting at (`x`, `y`).
    pub(crate) fn line(
        &self,
        text: &str,
        size: u32,
        x: f32,
        y: f32,
        color: [f32; 4],
        out: &mut Vec<Glyph>,
    ) {
        let atlas_side = to_f32(ATLAS_SIDE);
        let mut pen = x;
        for ch in text.chars() {
            let Some(e) = self.entries.get(&(ch, size)) else {
                continue;
            };
            if e.width > 0 && e.height > 0 {
                let (gx, gy) = ((pen + e.left).round(), (y + e.top).round());
                out.push(Glyph {
                    rect: [gx, gy, to_f32(e.width), to_f32(e.height)],
                    uv: [
                        to_f32(e.x) / atlas_side,
                        to_f32(e.y) / atlas_side,
                        to_f32(e.x + e.width) / atlas_side,
                        to_f32(e.y + e.height) / atlas_side,
                    ],
                    color,
                });
            }
            pen += e.advance;
        }
    }

    /// Lay `text` out in lines no wider than `width`, from (`x`, `top`).
    /// Returns the y below the last line.
    #[expect(
        clippy::too_many_arguments,
        reason = "a layout call names its box and style"
    )]
    pub(crate) fn paragraph(
        &self,
        text: &str,
        size: u32,
        x: f32,
        top: f32,
        width: f32,
        color: [f32; 4],
        out: &mut Vec<Glyph>,
    ) -> f32 {
        let line_height = self.line_height(size);
        let ascent = (line_height * 0.8).round();
        let space = self.width(" ", size);
        let mut y = top;
        let mut line = String::new();
        let mut line_width = 0.0;
        for word in text.split_whitespace() {
            let word_width = self.width(word, size);
            if !line.is_empty() && line_width + space + word_width > width {
                self.line(&line, size, x, y + ascent, color, out);
                y += line_height;
                line.clear();
                line_width = 0.0;
            }
            if !line.is_empty() {
                line.push(' ');
                line_width += space;
            }
            line.push_str(word);
            line_width += word_width;
        }
        if !line.is_empty() {
            self.line(&line, size, x, y + ascent, color, out);
            y += line_height;
        }
        y
    }
}

/// One glyph's coverage mask, its size, and its offset from the pen (y down).
fn rasterize(
    font: &FontRef<'_>,
    gid: GlyphId,
    px: f32,
) -> Result<(Vec<u8>, usize, usize, f32, f32), String> {
    let path = raster::outline(font, gid, px, LocationRef::default());
    let Ok(path) = path else {
        return Ok((Vec::new(), 0, 0, 0.0, 0.0));
    };
    let Some(b) = path.bounds else {
        return Ok((Vec::new(), 0, 0, 0.0, 0.0));
    };
    let (x0, x1) = (b.min_x.floor() - 1.0, b.max_x.ceil() + 1.0);
    let (y0, y1) = (b.min_y.floor() - 1.0, b.max_y.ceil() + 1.0);
    let (width, height) = (to_usize(x1 - x0), to_usize(y1 - y0));
    // The canvas's origin: the pen at (-x0, y1) in canvas pixels, y down.
    let canvas = raster::fill(&path, (-x0, y1), width, height, Filler::Zeno)
        .map_err(|e| format!("glyph {}: {e:?}", gid.to_u32()))?;
    Ok((canvas.pixels, width, height, x0, -y1))
}

fn to_f32(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "glyph boxes are small and positive"
)]
fn to_usize(v: f32) -> usize {
    v.max(1.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_atlas_holds_ascii_and_lays_out_lines() {
        let atlas = Atlas::new(&[26]).expect("Helvetica rasterizes");
        assert!(atlas.entries.len() >= 94, "{}", atlas.entries.len());
        assert!(atlas.pixels.iter().any(|&p| p > 200), "some ink");
        let mut quads = Vec::new();
        atlas.line("Axiom", 26, 10.0, 40.0, [0.0, 0.0, 0.0, 1.0], &mut quads);
        assert_eq!(quads.len(), 5);
        assert!(quads.windows(2).all(|w| w[1].rect[0] > w[0].rect[0]));
        let bottom = atlas.paragraph(
            "one two three four five six seven eight nine ten",
            26,
            0.0,
            0.0,
            200.0,
            [0.0; 4],
            &mut quads,
        );
        assert!(
            bottom >= 2.0 * atlas.line_height(26),
            "wrapped onto several lines"
        );
    }
}

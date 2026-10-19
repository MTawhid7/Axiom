//! Glyph rasterization on the Rust path (TX12): skrifa's unhinted outlines,
//! filled into 8-bit coverage masks by one of the two candidates S5-Q2
//! compares, zeno or `vello_cpu`.
//!
//! **zeno's one `unsafe`** converts each coordinate with
//! `f32::to_int_unchecked`, which is undefined behaviour for a value that is
//! not finite or doesn't fit `i32` after scaling by 256 (`raster.rs` in zeno
//! 0.3.3). An untrusted font controls the coordinates, so [`fill`] checks them
//! before calling zeno; a published crate that relies on its callers for
//! soundness is a finding for the dependency register.

use std::io::Cursor;

use read_fonts::TableProvider;
use skrifa::bitmap::{BitmapData, BitmapStrikes, Origin};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, HintingInstance, OutlinePen, SmoothMode, Target};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use vello_cpu::kurbo::{BezPath, Point};

/// The two candidate rasterizers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filler {
    Zeno,
    Vello,
}

impl Filler {
    pub fn name(self) -> &'static str {
        match self {
            Self::Zeno => "zeno",
            Self::Vello => "vello_cpu",
        }
    }
}

/// Why a glyph wasn't rasterized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RasterError {
    /// The font has no outline for the glyph.
    NoOutline,
    /// skrifa failed to draw it.
    Draw(String),
    /// A coordinate is not finite or too large to fill safely.
    OutOfRange,
    /// The canvas is larger than `vello_cpu`'s `u16` dimensions.
    CanvasTooLarge,
    /// An embedded bitmap failed to decode.
    Bitmap(String),
}

/// A glyph's outline, in pixels at the requested size, y up, with its
/// bounds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    commands: Vec<Command>,
    pub bounds: Option<Bounds>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Command {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

impl Command {
    /// The command's coordinates, x and y interleaved.
    fn coordinates(&self) -> Vec<f32> {
        match *self {
            Command::Move(x, y) | Command::Line(x, y) => vec![x, y],
            Command::Quad(cx, cy, x, y) => vec![cx, cy, x, y],
            Command::Cubic(c0x, c0y, c1x, c1y, x, y) => vec![c0x, c0y, c1x, c1y, x, y],
            Command::Close => Vec::new(),
        }
    }
}

impl Path {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    fn include(&mut self, x: f32, y: f32) {
        let b = self.bounds.get_or_insert(Bounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
        b.min_x = b.min_x.min(x);
        b.min_y = b.min_y.min(y);
        b.max_x = b.max_x.max(x);
        b.max_y = b.max_y.max(y);
    }
}

impl OutlinePen for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        self.include(x, y);
        self.commands.push(Command::Move(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.include(x, y);
        self.commands.push(Command::Line(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.include(cx0, cy0);
        self.include(x, y);
        self.commands.push(Command::Quad(cx0, cy0, x, y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.include(cx0, cy0);
        self.include(cx1, cy1);
        self.include(x, y);
        self.commands.push(Command::Cubic(cx0, cy0, cx1, cy1, x, y));
    }

    fn close(&mut self) {
        self.commands.push(Command::Close);
    }
}

/// `glyph`'s unhinted outline at `size_px` pixels per em.
pub fn outline(
    font: &FontRef<'_>,
    glyph: GlyphId,
    size_px: f32,
    location: LocationRef<'_>,
) -> Result<Path, RasterError> {
    let outlines = font.outline_glyphs();
    let glyph = outlines.get(glyph).ok_or(RasterError::NoOutline)?;
    let mut path = Path::default();
    glyph
        .draw(
            DrawSettings::unhinted(Size::new(size_px), location),
            &mut path,
        )
        .map_err(|e| RasterError::Draw(e.to_string()))?;
    Ok(path)
}

/// `glyph`'s outline at `size_px` pixels per em, hinted vertically only
/// (`FreeType`'s "light" mode): the font's own hints where it has them, the
/// autohinter otherwise. S5-Q2 compares it with CoreText's small sizes.
pub fn outline_light_hinted(
    font: &FontRef<'_>,
    glyph: GlyphId,
    size_px: f32,
    location: LocationRef<'_>,
) -> Result<Path, RasterError> {
    let outlines = font.outline_glyphs();
    let target = Target::Smooth {
        mode: SmoothMode::Light,
        symmetric_rendering: true,
        preserve_linear_metrics: true,
    };
    let instance = HintingInstance::new(&outlines, Size::new(size_px), location, target)
        .map_err(|e| RasterError::Draw(e.to_string()))?;
    let glyph = outlines.get(glyph).ok_or(RasterError::NoOutline)?;
    let mut path = Path::default();
    glyph
        .draw(DrawSettings::hinted(&instance, false), &mut path)
        .map_err(|e| RasterError::Draw(e.to_string()))?;
    Ok(path)
}

/// An 8-bit coverage mask, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Canvas {
    /// The number of pixels with any coverage.
    pub fn ink(&self) -> usize {
        self.pixels.iter().filter(|&&c| c > 0).count()
    }
}

/// zeno's coordinates are scaled by 256 into an `i32`, so they must stay well
/// inside ±2^23. Fonts never come near: this only rejects hostile input.
const MAX_COORDINATE: f32 = 4_194_304.0; // 2^22

/// Fill `path` on a `width` × `height` canvas, with the glyph's origin at
/// `origin` (canvas pixels, y down) and the non-zero winding rule that
/// TrueType and CFF outlines use.
pub fn fill(
    path: &Path,
    origin: (f32, f32),
    width: usize,
    height: usize,
    filler: Filler,
) -> Result<Canvas, RasterError> {
    let place = |x: f32, y: f32| (origin.0 + x, origin.1 - y);
    let mut placed = Vec::with_capacity(path.commands.len());
    for command in &path.commands {
        let c = match *command {
            Command::Move(x, y) => {
                let (x, y) = place(x, y);
                Command::Move(x, y)
            }
            Command::Line(x, y) => {
                let (x, y) = place(x, y);
                Command::Line(x, y)
            }
            Command::Quad(cx, cy, x, y) => {
                let (cx, cy) = place(cx, cy);
                let (x, y) = place(x, y);
                Command::Quad(cx, cy, x, y)
            }
            Command::Cubic(c0x, c0y, c1x, c1y, x, y) => {
                let (c0x, c0y) = place(c0x, c0y);
                let (c1x, c1y) = place(c1x, c1y);
                let (x, y) = place(x, y);
                Command::Cubic(c0x, c0y, c1x, c1y, x, y)
            }
            Command::Close => Command::Close,
        };
        placed.push(c);
    }
    let in_range = |v: &f32| v.is_finite() && v.abs() <= MAX_COORDINATE;
    if !placed.iter().all(|c| c.coordinates().iter().all(in_range)) {
        return Err(RasterError::OutOfRange);
    }
    match filler {
        Filler::Zeno => Ok(fill_zeno(&placed, width, height)),
        Filler::Vello => fill_vello(&placed, width, height),
    }
}

fn fill_zeno(commands: &[Command], width: usize, height: usize) -> Canvas {
    use zeno::{Command as Z, Format, Mask, Point as P};
    let path: Vec<Z> = commands
        .iter()
        .map(|c| match *c {
            Command::Move(x, y) => Z::MoveTo(P::new(x, y)),
            Command::Line(x, y) => Z::LineTo(P::new(x, y)),
            Command::Quad(cx, cy, x, y) => Z::QuadTo(P::new(cx, cy), P::new(x, y)),
            Command::Cubic(ax, ay, bx, by, x, y) => {
                Z::CurveTo(P::new(ax, ay), P::new(bx, by), P::new(x, y))
            }
            Command::Close => Z::Close,
        })
        .collect();
    let mut pixels = vec![0_u8; width * height];
    if !path.is_empty() && width > 0 && height > 0 {
        Mask::new(&path)
            .size(
                u32::try_from(width).unwrap_or(u32::MAX),
                u32::try_from(height).unwrap_or(u32::MAX),
            )
            .format(Format::Alpha)
            .render_into(&mut pixels, None);
    }
    Canvas {
        width,
        height,
        pixels,
    }
}

fn fill_vello(commands: &[Command], width: usize, height: usize) -> Result<Canvas, RasterError> {
    use vello_cpu::{Pixmap, RenderContext, Resources, color::palette::css::WHITE};
    let pixmap_width = u16::try_from(width).map_err(|_| RasterError::CanvasTooLarge)?;
    let pixmap_height = u16::try_from(height).map_err(|_| RasterError::CanvasTooLarge)?;
    let point = |x: f32, y: f32| Point::new(f64::from(x), f64::from(y));
    let mut path = BezPath::new();
    for command in commands {
        match *command {
            Command::Move(x, y) => path.move_to(point(x, y)),
            Command::Line(x, y) => path.line_to(point(x, y)),
            Command::Quad(cx, cy, x, y) => path.quad_to(point(cx, cy), point(x, y)),
            Command::Cubic(c0x, c0y, c1x, c1y, x, y) => {
                path.curve_to(point(c0x, c0y), point(c1x, c1y), point(x, y));
            }
            Command::Close => path.close_path(),
        }
    }
    let mut pixels = vec![0_u8; width * height];
    if width == 0 || height == 0 || commands.is_empty() {
        return Ok(Canvas {
            width,
            height,
            pixels,
        });
    }
    let mut context = RenderContext::new(pixmap_width, pixmap_height);
    context.set_paint(WHITE);
    context.fill_path(&path);
    context.flush();
    let mut pixmap = Pixmap::new(pixmap_width, pixmap_height);
    context.render(&mut pixmap, &mut Resources::new());
    for (out, pixel) in pixels.iter_mut().zip(pixmap.data()) {
        *out = pixel.a;
    }
    Ok(Canvas {
        width,
        height,
        pixels,
    })
}

/// A colour glyph from an embedded bitmap (`sbix`, `CBDT` or `EBDT`), for
/// TX13's RGBA atlas: premultiplied RGBA scaled to the requested size, with
/// its top-left corner at (`left`, `top`) pixels from the glyph's origin, y
/// down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorGlyph {
    pub width: usize,
    pub height: usize,
    pub left: i32,
    pub top: i32,
    pub rgba: Vec<u8>,
}

impl ColorGlyph {
    /// The number of pixels with any coverage.
    pub fn ink(&self) -> usize {
        self.rgba.chunks_exact(4).filter(|p| p[3] > 0).count()
    }
}

/// The largest embedded bitmap decoded: 4,096 pixels on a side.
const MAX_BITMAP_SIDE: u32 = 4096;

/// `glyph`'s embedded bitmap at `size_px` pixels per em, from the strike
/// skrifa picks (the exact size, else the nearest larger, else the nearest
/// smaller), scaled by area averaging. Placement follows skrifa's fields:
/// `sbix` glyphs from their bottom-left corner, `CBDT` and `EBDT` glyphs from
/// their top-left.
pub fn color_glyph(
    font: &FontRef<'_>,
    glyph: GlyphId,
    size_px: f32,
) -> Result<ColorGlyph, RasterError> {
    let bitmap = BitmapStrikes::new(font)
        .glyph_for_size(Size::new(size_px), glyph)
        .ok_or(RasterError::NoOutline)?;
    if bitmap.width > MAX_BITMAP_SIDE || bitmap.height > MAX_BITMAP_SIDE || bitmap.ppem_y <= 0.0 {
        return Err(RasterError::OutOfRange);
    }
    let (rgba, width, height) = match bitmap.data {
        BitmapData::Png(bytes) => decode_png(bytes)?,
        BitmapData::Bgra(bytes) => {
            let (w, h) = (bitmap.width as usize, bitmap.height as usize);
            let bytes = bytes
                .get(..w * h * 4)
                .ok_or(RasterError::Bitmap("short BGRA data".into()))?;
            let rgba = bytes
                .chunks_exact(4)
                .flat_map(|p| [p[2], p[1], p[0], p[3]])
                .collect();
            (rgba, w, h)
        }
        BitmapData::Mask(mask) => {
            let alpha = mask
                .decode(bitmap.width, bitmap.height)
                .map_err(|e| RasterError::Bitmap(format!("{e:?}")))?;
            let rgba = alpha.iter().flat_map(|&a| [a, a, a, a]).collect();
            (rgba, bitmap.width as usize, bitmap.height as usize)
        }
    };
    let strike_scale = size_px / bitmap.ppem_y;
    let units_per_em = f32::from(font.head().map(|h| h.units_per_em()).unwrap_or(1000).max(1));
    let font_scale = size_px / units_per_em;
    let scaled_w = to_pixels(side_f32(width) * strike_scale).max(1);
    let scaled_h = to_pixels(side_f32(height) * strike_scale).max(1);
    let scaled = resample(&rgba, width, height, scaled_w, scaled_h);
    let (left, top) = match bitmap.placement_origin {
        Origin::BottomLeft => {
            let left = bitmap.bearing_x * font_scale + bitmap.inner_bearing_x * strike_scale;
            let bottom = bitmap.bearing_y * font_scale + bitmap.inner_bearing_y * strike_scale;
            (left, -(bottom + side_f32(scaled_h)))
        }
        Origin::TopLeft => (
            bitmap.inner_bearing_x * strike_scale,
            -(bitmap.inner_bearing_y * strike_scale),
        ),
    };
    Ok(ColorGlyph {
        width: scaled_w,
        height: scaled_h,
        left: round_i32(left),
        top: round_i32(top),
        rgba: scaled,
    })
}

/// A bitmap side in pixels (at most `MAX_BITMAP_SIDE`, or a scaled side below
/// 2^16) as `f32`, exactly.
fn side_f32(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bitmap sides are at most 4,096 pixels, and scales are small and positive"
)]
fn to_pixels(v: f32) -> usize {
    if v.is_finite() {
        v.round().clamp(0.0, 65_536.0) as usize
    } else {
        0
    }
}

#[expect(clippy::cast_possible_truncation, reason = "clamped to i32's range")]
fn round_i32(v: f32) -> i32 {
    if v.is_finite() {
        v.round().clamp(-1.0e9, 1.0e9) as i32
    } else {
        0
    }
}

/// Decode an embedded PNG to premultiplied RGBA.
fn decode_png(bytes: &[u8]) -> Result<(Vec<u8>, usize, usize), RasterError> {
    let err = |e: png::DecodingError| RasterError::Bitmap(e.to_string());
    let mut decoder =
        png::Decoder::new_with_limits(Cursor::new(bytes), png::Limits { bytes: 64 << 20 });
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(err)?;
    let info = reader.info();
    if info.width > MAX_BITMAP_SIDE || info.height > MAX_BITMAP_SIDE {
        return Err(RasterError::OutOfRange);
    }
    let size = reader
        .output_buffer_size()
        .ok_or(RasterError::Bitmap("PNG too large".into()))?;
    let mut buf = vec![0_u8; size];
    let frame = reader.next_frame(&mut buf).map_err(err)?;
    let (width, height) = (frame.width as usize, frame.height as usize);
    let channels = frame.color_type.samples();
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in buf.chunks(frame.line_size).take(height) {
        for px in row.chunks_exact(channels).take(width) {
            let (r, g, b, a) = match *px {
                [g] => (g, g, g, 255),
                [g, a] => (g, g, g, a),
                [r, g, b] => (r, g, b, 255),
                [r, g, b, a] => (r, g, b, a),
                _ => (0, 0, 0, 0),
            };
            let premultiply =
                |c: u8| u8::try_from((u16::from(c) * u16::from(a) + 127) / 255).unwrap_or(255);
            rgba.extend_from_slice(&[premultiply(r), premultiply(g), premultiply(b), a]);
        }
    }
    if rgba.len() != width * height * 4 {
        return Err(RasterError::Bitmap(
            "PNG rows shorter than its width".into(),
        ));
    }
    Ok((rgba, width, height))
}

/// Resample premultiplied RGBA by averaging the source pixels whose centres
/// fall in each destination pixel (the nearest one when none do).
fn resample(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    let mut out = vec![0_u8; dw * dh * 4];
    if sw == 0 || sh == 0 {
        return out;
    }
    let span = |d: usize, dn: usize, sn: usize| {
        let start = d * sn / dn;
        let end = ((d + 1) * sn).div_ceil(dn).clamp(start + 1, sn);
        start.min(sn - 1)..end
    };
    for dy in 0..dh {
        let rows = span(dy, dh, sh);
        for dx in 0..dw {
            let cols = span(dx, dw, sw);
            let mut sum = [0_u32; 4];
            let mut count = 0_u32;
            for y in rows.clone() {
                for x in cols.clone() {
                    let p = &src[(y * sw + x) * 4..(y * sw + x) * 4 + 4];
                    for c in 0..4 {
                        sum[c] += u32::from(p[c]);
                    }
                    count += 1;
                }
            }
            let at = (dy * dw + dx) * 4;
            for c in 0..4 {
                out[at + c] = u8::try_from(sum[c] / count.max(1)).unwrap_or(255);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testfont;

    #[test]
    fn both_fillers_cover_the_test_square_where_it_is() {
        let data = testfont::square_font();
        let font = FontRef::new(&data).expect("parses");
        // At 100 px per em the square spans x 10..60, y 0..50 (y up).
        let path = outline(
            &font,
            GlyphId::new(testfont::GLYPH_A),
            100.0,
            LocationRef::default(),
        )
        .expect("draws");
        for filler in [Filler::Zeno, Filler::Vello] {
            let canvas = fill(&path, (0.0, 60.0), 80, 80, filler).expect("fills");
            let at = |x: usize, y: usize| canvas.pixels[y * canvas.width + x];
            // Inside: x 10..60, rows 10..60 (y down, baseline at row 60).
            assert_eq!(at(30, 30), 255, "{filler:?} inside");
            assert_eq!(at(5, 30), 0, "{filler:?} left of it");
            assert_eq!(at(30, 65), 0, "{filler:?} below the baseline");
            assert_eq!(canvas.ink(), 50 * 50, "{filler:?} exact pixel area");
        }
    }

    #[test]
    fn rejects_coordinates_zeno_cannot_take_safely() {
        let mut path = Path::default();
        path.move_to(0.0, 0.0);
        path.line_to(f32::INFINITY, 0.0);
        path.line_to(0.0, 1.0);
        path.close();
        assert_eq!(
            fill(&path, (0.0, 0.0), 8, 8, Filler::Zeno),
            Err(RasterError::OutOfRange)
        );
        let mut path = Path::default();
        path.move_to(0.0, 0.0);
        path.line_to(1.0e9, 0.0);
        path.line_to(0.0, 1.0);
        path.close();
        assert_eq!(
            fill(&path, (0.0, 0.0), 8, 8, Filler::Zeno),
            Err(RasterError::OutOfRange)
        );
    }
}

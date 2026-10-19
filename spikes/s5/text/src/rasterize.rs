//! S5-Q2: CoreText's glyph masks against zeno's and `vello_cpu`'s, for the
//! corpus's glyphs at 12–32 px, at 1× and 2×, with and without font
//! smoothing, and what a glyph costs each way.
//!
//! Every comparison draws the same glyph, at the same size and variation, on
//! the same canvas with the same origin. CoreText draws white on black into a
//! gray context with smoothing off (the reference), into a gray context with
//! smoothing on, and into an RGB context with smoothing on (its red channel),
//! since smoothing may only apply in colour contexts. Colour glyphs (Apple
//! Color Emoji) are compared in RGBA.

use std::time::{Duration, Instant};

use s5_rust_text::raster::{self, Filler};
use skrifa::instance::LocationRef;
use skrifa::{GlyphId, MetadataProvider, Tag};

use crate::corpus;
use crate::fonts::{self, FontData};
use crate::platform::coretext::{self, Canvas, Font, Line, Smoothing};
use crate::sheet::Sheet;
use crate::{Options, shaping, to_px};

/// Text sizes, in points; each is drawn at 1× and 2× (Retina) scale.
const SIZES: [f64; 7] = [12.0, 14.0, 16.0, 20.0, 24.0, 28.0, 32.0];
const SCALES: [f64; 2] = [1.0, 2.0];
/// Glyphs sampled from each corpus line, spread across it.
const GLYPHS_PER_LINE: usize = 12;
/// Canvas margin around the union of both engines' glyph bounds, in pixels.
const PAD: f64 = 2.0;
/// Coverage differences above this count as visibly different pixels.
const VISIBLE: u8 = 32;

/// A glyph to compare.
struct Sample {
    line: &'static str,
    font: Font,
    data: FontData,
    glyph: u16,
    color: bool,
}

/// How one mask compares with a reference.
#[derive(Clone, Copy, Debug, Default)]
struct Diff {
    /// Mean absolute difference, 0–255.
    mean: f64,
    max: u8,
    /// The share of pixels differing by more than `VISIBLE`.
    visible: f64,
    /// Total coverage relative to the reference's.
    ink: f64,
}

fn diff(reference: &[u8], other: &[u8]) -> Diff {
    let n = reference.len().min(other.len());
    if n == 0 {
        return Diff::default();
    }
    let (mut sum, mut max, mut visible) = (0_u64, 0_u8, 0_usize);
    let (mut ink_ref, mut ink_other) = (0_u64, 0_u64);
    for (&a, &b) in reference.iter().zip(other) {
        let d = a.abs_diff(b);
        sum += u64::from(d);
        max = max.max(d);
        if d > VISIBLE {
            visible += 1;
        }
        ink_ref += u64::from(a);
        ink_other += u64::from(b);
    }
    #[expect(clippy::cast_precision_loss, reason = "pixel counts far below 2^52")]
    Diff {
        mean: sum as f64 / n as f64,
        max,
        visible: visible as f64 / n as f64,
        ink: if ink_ref == 0 {
            0.0
        } else {
            ink_other as f64 / ink_ref as f64
        },
    }
}

/// The ink-weighted centroid of a mask, in pixels (x, y down).
fn centroid(mask: &[u8], width: usize) -> (f64, f64) {
    let (mut sx, mut sy, mut total) = (0.0, 0.0, 0.0);
    for (i, &c) in mask.iter().enumerate() {
        let w = f64::from(c);
        sx += w * to_f64(i % width.max(1));
        sy += w * to_f64(i / width.max(1));
        total += w;
    }
    if total == 0.0 {
        (0.0, 0.0)
    } else {
        (sx / total, sy / total)
    }
}

/// Accumulated means of the `Diff` fields.
#[derive(Clone, Copy, Debug, Default)]
struct Mean {
    n: usize,
    abs_diff: f64,
    visible: f64,
    ink: f64,
    max: u8,
}

impl Mean {
    fn add(&mut self, d: Diff) {
        self.n += 1;
        self.abs_diff += d.mean;
        self.visible += d.visible;
        self.ink += d.ink;
        self.max = self.max.max(d.max);
    }

    fn show(&self) -> String {
        if self.n == 0 {
            return "-".to_owned();
        }
        #[expect(clippy::cast_precision_loss, reason = "small counts")]
        let n = self.n as f64;
        format!(
            "mean {:.2} visible {:.2}% ink {:.3} max {}",
            self.abs_diff / n,
            100.0 * self.visible / n,
            self.ink / n,
            self.max
        )
    }
}

pub(crate) fn run(options: &Options) -> Result<(), String> {
    let samples = gather();
    let mono = samples.iter().filter(|s| !s.color).count();
    println!(
        "# raster: {} glyphs ({mono} outline, {} colour) from {} lines",
        samples.len(),
        samples.len() - mono,
        corpus::samples().len()
    );
    println!(
        "# raster\tline\tfont\tglyph\tsize\tscale\tcanvas\tzeno\tvello\tsmoothing-gray\tsmoothing-rgb\tzeno-vs-rgb\tvello-vs-rgb\tzeno-light-hinted"
    );
    let mut means: Vec<((f64, f64), [Mean; 7])> = Vec::new();
    let mut shifts: Vec<((f64, f64), f64, f64, usize)> = Vec::new();
    for &scale in &SCALES {
        for &size in &SIZES {
            let mut row = [Mean::default(); 7];
            // CoreText's ink centroid minus zeno's, averaged: a systematic
            // offset shows here.
            let (mut dx, mut dy, mut n) = (0.0, 0.0, 0_usize);
            for sample in samples.iter().filter(|s| !s.color) {
                let Some(c) = compare(sample, size, scale) else {
                    continue;
                };
                let diffs = [
                    diff(&c.ct_off, &c.zeno),
                    diff(&c.ct_off, &c.vello),
                    diff(&c.ct_off, &c.ct_gray_on),
                    diff(&c.ct_off, &c.ct_rgb_on),
                    diff(&c.ct_rgb_on, &c.zeno),
                    diff(&c.ct_rgb_on, &c.vello),
                    diff(&c.ct_off, &c.zeno_hinted),
                ];
                for (mean, d) in row.iter_mut().zip(diffs) {
                    mean.add(d);
                }
                let (cx, cy) = centroid(&c.ct_off, c.width);
                let (zx, zy) = centroid(&c.zeno, c.width);
                dx += cx - zx;
                dy += cy - zy;
                n += 1;
                println!(
                    "raster\t{}\t{}\t{}\t{size}\t{scale}\t{}x{}\t{:.2}/{}/{:.3}\t{:.2}/{}/{:.3}\t{:.2}/{}\t{:.2}/{}/{:.3}\t{:.2}\t{:.2}\t{:.2}",
                    sample.line,
                    sample.font.postscript_name(),
                    sample.glyph,
                    c.width,
                    c.height,
                    diffs[0].mean,
                    diffs[0].max,
                    diffs[0].ink,
                    diffs[1].mean,
                    diffs[1].max,
                    diffs[1].ink,
                    diffs[2].mean,
                    diffs[2].max,
                    diffs[3].mean,
                    diffs[3].max,
                    diffs[3].ink,
                    diffs[4].mean,
                    diffs[5].mean,
                    diffs[6].mean,
                );
            }
            means.push(((size, scale), row));
            shifts.push(((size, scale), dx, dy, n));
        }
    }
    for ((size, scale), dx, dy, n) in &shifts {
        let n = to_f64(*n).max(1.0);
        println!(
            "# centroid {size} pt {scale}x	CoreText minus zeno: dx {:+.3} px, dy {:+.3} px (y down)",
            dx / n,
            dy / n
        );
    }
    let names = [
        "zeno vs CoreText",
        "vello_cpu vs CoreText",
        "CoreText smoothing (gray) vs off",
        "CoreText smoothing (RGB) vs off",
        "zeno vs CoreText smoothed",
        "vello_cpu vs CoreText smoothed",
        "zeno light-hinted vs CoreText",
    ];
    for ((size, scale), row) in &means {
        for (name, mean) in names.iter().zip(row) {
            println!("# mean {size} pt {scale}x\t{name}\t{}", mean.show());
        }
    }
    color(&samples, options.sheets.as_deref());
    timing(&samples, options.repeat.max(1) * 20);
    if let Some(dir) = &options.sheets {
        sheets(&samples, dir)?;
    }
    Ok(())
}

/// `raster-timing`: only the cost of a glyph each way, for repeated runs.
pub(crate) fn run_timing(options: &Options) {
    timing(&gather(), options.repeat.max(1) * 20);
}

/// Up to `GLYPHS_PER_LINE` distinct glyphs from each corpus line, spread
/// across CoreText's runs, at 16 pt.
fn gather() -> Vec<Sample> {
    let mut out = Vec::new();
    for sample in corpus::samples() {
        let line = Line::new(
            sample.text,
            &shaping::base_font(&sample, 16.0),
            Some(sample.language),
        );
        let mut candidates: Vec<(Font, FontData, u16)> = Vec::new();
        for run in line.runs() {
            let Ok(data) = fonts::font_data(&run.font) else {
                continue;
            };
            for &glyph in &run.glyphs {
                if !candidates.iter().any(|(f, _, g)| {
                    *g == glyph && f.postscript_name() == run.font.postscript_name()
                }) {
                    candidates.push((run.font.clone(), data.clone(), glyph));
                }
            }
        }
        // Drop glyphs with nothing to draw (spaces).
        candidates.retain(|(font, data, glyph)| {
            data.font_ref().is_some_and(|f| {
                is_color(&f, *glyph)
                    || raster::outline(
                        &f,
                        GlyphId::new(u32::from(*glyph)),
                        16.0,
                        LocationRef::default(),
                    )
                    .is_ok_and(|p| !p.is_empty())
            }) && font.units_per_em() > 0
        });
        let step = candidates.len().div_ceil(GLYPHS_PER_LINE).max(1);
        for (font, data, glyph) in candidates.into_iter().step_by(step).take(GLYPHS_PER_LINE) {
            let color = data.font_ref().is_some_and(|f| is_color(&f, glyph));
            out.push(Sample {
                line: sample.id,
                font,
                data,
                glyph,
                color,
            });
        }
    }
    out
}

fn is_color(font: &skrifa::FontRef<'_>, glyph: u16) -> bool {
    skrifa::bitmap::BitmapStrikes::new(font)
        .glyph_for_size(
            skrifa::instance::Size::new(16.0),
            GlyphId::new(u32::from(glyph)),
        )
        .is_some()
}

/// One glyph at one pixel size, every way.
struct Compared {
    width: usize,
    height: usize,
    ct_off: Vec<u8>,
    ct_gray_on: Vec<u8>,
    ct_rgb_on: Vec<u8>,
    zeno: Vec<u8>,
    vello: Vec<u8>,
    zeno_hinted: Vec<u8>,
}

/// The canvas both engines draw on, in device pixels: the union of their
/// bounds, with a margin.
fn layout(
    sample: &Sample,
    ct_font: &Font,
    scale: f64,
    path: &raster::Path,
) -> (usize, usize, (f64, f64)) {
    let ct = ct_font.glyph_bounds(sample.glyph);
    let (mut x0, mut y0) = (ct.origin.x * scale, ct.origin.y * scale);
    let (mut x1, mut y1) = (
        (ct.origin.x + ct.size.width) * scale,
        (ct.origin.y + ct.size.height) * scale,
    );
    if let Some(b) = path.bounds {
        x0 = x0.min(f64::from(b.min_x));
        y0 = y0.min(f64::from(b.min_y));
        x1 = x1.max(f64::from(b.max_x));
        y1 = y1.max(f64::from(b.max_y));
    }
    let (x0, y0) = (x0.floor() - PAD, y0.floor() - PAD);
    let (x1, y1) = (x1.ceil() + PAD, y1.ceil() + PAD);
    (to_px(x1 - x0), to_px(y1 - y0), (-x0, y1))
}

fn rust_variation(font: &Font) -> Vec<(Tag, f32)> {
    #[expect(clippy::cast_possible_truncation, reason = "axis values are small")]
    font.variation()
        .into_iter()
        .map(|(tag, value)| (Tag::new(&tag), value as f32))
        .collect()
}

/// `sample` at `size` points and `scale` device pixels per point: CoreText's
/// font keeps the point size, so its optical size and variation are the point
/// size's, and the Rust path draws that variation at `size × scale` pixels.
fn compare(sample: &Sample, size: f64, scale: f64) -> Option<Compared> {
    let ct_font = sample.font.with_size(size);
    let font_ref = sample.data.font_ref()?;
    let location = font_ref.axes().location(rust_variation(&ct_font));
    #[expect(clippy::cast_possible_truncation, reason = "pixel sizes are small")]
    let path = raster::outline(
        &font_ref,
        GlyphId::new(u32::from(sample.glyph)),
        (size * scale) as f32,
        LocationRef::from(&location),
    )
    .ok()?;
    let (width, height, origin) = layout(sample, &ct_font, scale, &path);
    let ct = |rgb: bool, smoothing: Smoothing| {
        let mut canvas = Canvas::new(width, height, rgb, smoothing).scaled(scale);
        canvas.draw_glyphs(&ct_font, &[sample.glyph], &[(0.0, 0.0)], origin);
        canvas.pixels
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "canvas coordinates are small"
    )]
    let origin32 = (origin.0 as f32, origin.1 as f32);
    let fill = |filler| raster::fill(&path, origin32, width, height, filler).map(|c| c.pixels);
    #[expect(clippy::cast_possible_truncation, reason = "pixel sizes are small")]
    let hinted = raster::outline_light_hinted(
        &font_ref,
        GlyphId::new(u32::from(sample.glyph)),
        (size * scale) as f32,
        LocationRef::from(&location),
    )
    .ok()
    .and_then(|p| raster::fill(&p, origin32, width, height, Filler::Zeno).ok())
    .map_or_else(|| vec![0; width * height], |c| c.pixels);
    Some(Compared {
        width,
        height,
        ct_off: ct(false, Smoothing::Off),
        ct_gray_on: ct(false, Smoothing::On),
        ct_rgb_on: ct(true, Smoothing::On),
        zeno: fill(Filler::Zeno).ok()?,
        vello: fill(Filler::Vello).ok()?,
        zeno_hinted: hinted,
    })
}

/// Colour glyphs: CoreText's RGBA rendering against the Rust path's (the
/// `sbix` PNG decoded by `png` and scaled by area averaging). With `sheets`,
/// one image per size of CoreText's emoji above the Rust path's.
fn color(samples: &[Sample], sheets: Option<&std::path::Path>) {
    for &scale in &SCALES {
        for &size in &SIZES {
            let px = size * scale;
            let mut alpha = Mean::default();
            let mut rgb = Mean::default();
            let (mut dx, mut dy, mut count) = (0.0, 0.0, 0.0);
            let mut pairs: Vec<(Vec<u8>, Vec<u8>, usize, usize)> = Vec::new();
            for sample in samples.iter().filter(|s| s.color) {
                let Some(font_ref) = sample.data.font_ref() else {
                    continue;
                };
                #[expect(clippy::cast_possible_truncation, reason = "pixel sizes are small")]
                let Ok(glyph) = raster::color_glyph(
                    &font_ref,
                    GlyphId::new(u32::from(sample.glyph)),
                    px as f32,
                ) else {
                    continue;
                };
                let ct_font = sample.font.with_size(size);
                let bounds = ct_font.glyph_bounds(sample.glyph);
                let (bx0, by0) = (bounds.origin.x * scale, bounds.origin.y * scale);
                let (bx1, by1) = (
                    (bounds.origin.x + bounds.size.width) * scale,
                    (bounds.origin.y + bounds.size.height) * scale,
                );
                let x0 = bx0.min(f64::from(glyph.left)).floor() - PAD;
                let x1 = bx1.max(f64::from(glyph.left) + to_f64(glyph.width)).ceil() + PAD;
                let y1 = by1.max(-f64::from(glyph.top)).ceil() + PAD;
                let y0 = by0
                    .min(-f64::from(glyph.top) - to_f64(glyph.height))
                    .floor()
                    - PAD;
                let (width, height) = (to_px(x1 - x0), to_px(y1 - y0));
                let origin = (-x0, y1);
                let ct = coretext::draw_color_glyph(
                    &ct_font,
                    sample.glyph,
                    width,
                    height,
                    origin,
                    scale,
                );
                let ours = place(&glyph, origin, width, height);
                let ct_alpha: Vec<u8> = ct.chunks_exact(4).map(|p| p[3]).collect();
                let our_alpha: Vec<u8> = ours.chunks_exact(4).map(|p| p[3]).collect();
                alpha.add(diff(&ct_alpha, &our_alpha));
                let ct_rgb: Vec<u8> = ct
                    .chunks_exact(4)
                    .flat_map(|p| [p[0], p[1], p[2]])
                    .collect();
                let our_rgb: Vec<u8> = ours
                    .chunks_exact(4)
                    .flat_map(|p| [p[0], p[1], p[2]])
                    .collect();
                rgb.add(diff(&ct_rgb, &our_rgb));
                let (cx, cy) = centroid(&ct_alpha, width);
                let (ox2, oy2) = centroid(&our_alpha, width);
                dx += cx - ox2;
                dy += cy - oy2;
                count += 1.0;
                pairs.push((ct, ours, width, height));
            }
            println!(
                "# colour {size} pt {scale}x\talpha {}\tRGB {}\tcentroid CoreText minus ours: dx {:+.2} px, dy {:+.2} px",
                alpha.show(),
                rgb.show(),
                if count > 0.0 { dx / count } else { 0.0 },
                if count > 0.0 { dy / count } else { 0.0 }
            );
            #[expect(clippy::float_cmp, reason = "the sizes are the constants")]
            if let Some(dir) = sheets
                && (size == 16.0 || size == 32.0)
                && scale == 1.0
            {
                let _ = color_sheet(dir, size, &pairs);
            }
        }
    }
}

/// The Rust path's colour glyph on a transparent RGBA canvas of `width` ×
/// `height`, with the glyph's origin at `origin` (pixels, y down).
fn place(glyph: &raster::ColorGlyph, origin: (f64, f64), width: usize, height: usize) -> Vec<u8> {
    let mut canvas = vec![0_u8; width * height * 4];
    #[expect(
        clippy::cast_possible_truncation,
        reason = "canvas coordinates are small"
    )]
    let (ox, oy) = (
        origin.0.round() as i64 + i64::from(glyph.left),
        origin.1.round() as i64 + i64::from(glyph.top),
    );
    for row in 0..glyph.height {
        for col in 0..glyph.width {
            if let (Ok(x), Ok(y)) = (
                usize::try_from(ox + to_i64(col)),
                usize::try_from(oy + to_i64(row)),
            ) && x < width
                && y < height
            {
                let src = (row * glyph.width + col) * 4;
                let dst = (y * width + x) * 4;
                canvas[dst..dst + 4].copy_from_slice(&glyph.rgba[src..src + 4]);
            }
        }
    }
    canvas
}

/// CoreText's colour glyphs (top) above the Rust path's (bottom), on white,
/// magnified 4×.
fn color_sheet(
    dir: &std::path::Path,
    size: f64,
    pairs: &[(Vec<u8>, Vec<u8>, usize, usize)],
) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let magnify = 4;
    let width: usize = pairs.iter().map(|p| (p.2 + 4) * magnify).sum();
    let height = pairs.iter().map(|p| p.3).max().unwrap_or(0) * magnify * 2 + 8;
    let mut sheet = Sheet::new(width.max(1), height.max(1));
    let mut x = 0;
    for (ct, ours, w, h) in pairs {
        for (row, rgba) in [ct, ours].into_iter().enumerate() {
            for (i, px) in rgba.chunks_exact(4).enumerate() {
                let (col, line) = (i % w, i / w);
                // Premultiplied over white.
                let color = [
                    px[0].saturating_add(255 - px[3]),
                    px[1].saturating_add(255 - px[3]),
                    px[2].saturating_add(255 - px[3]),
                ];
                for dy in 0..magnify {
                    for dx in 0..magnify {
                        let (sx, sy) = (
                            x + col * magnify + dx,
                            row * (h * magnify + 8) + line * magnify + dy,
                        );
                        if sx < sheet.width && sy < sheet.height {
                            let at = (sy * sheet.width + sx) * 3;
                            sheet.pixels[at..at + 3].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
        x += (w + 4) * magnify;
    }
    sheet.write(&dir.join(format!("raster-emoji-{size}pt.png")))
}

fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn to_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Rasterization cost per glyph: CoreText into one reused context, and the
/// Rust path's outline extraction plus fill, at 16 and 32 px.
fn timing(samples: &[Sample], repeats: usize) {
    for px in [16.0, 32.0] {
        let (mut ct, mut zeno, mut vello) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        let mut glyphs = 0_u32;
        for sample in samples.iter().filter(|s| !s.color) {
            let ct_font = sample.font.with_size(px);
            let Some(font_ref) = sample.data.font_ref() else {
                continue;
            };
            let location = font_ref.axes().location(rust_variation(&ct_font));
            let id = GlyphId::new(u32::from(sample.glyph));
            #[expect(clippy::cast_possible_truncation, reason = "pixel sizes are small")]
            let Ok(path) = raster::outline(&font_ref, id, px as f32, LocationRef::from(&location))
            else {
                continue;
            };
            let (width, height, origin) = layout(sample, &ct_font, 1.0, &path);
            ct += coretext::time_glyph(&ct_font, sample.glyph, width, height, origin, repeats);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "canvas coordinates are small"
            )]
            let origin32 = (origin.0 as f32, origin.1 as f32);
            for (filler, total) in [(Filler::Zeno, &mut zeno), (Filler::Vello, &mut vello)] {
                let start = Instant::now();
                for _ in 0..repeats {
                    #[expect(clippy::cast_possible_truncation, reason = "pixel sizes are small")]
                    let path =
                        raster::outline(&font_ref, id, px as f32, LocationRef::from(&location))
                            .ok();
                    if let Some(path) = path {
                        let _ = std::hint::black_box(raster::fill(
                            &path, origin32, width, height, filler,
                        ));
                    }
                }
                *total += start.elapsed();
            }
            glyphs += 1;
        }
        let per = |d: Duration| {
            let n = f64::from(glyphs) * to_f64(repeats);
            if n == 0.0 {
                0.0
            } else {
                d.as_secs_f64() * 1e9 / n
            }
        };
        println!(
            "# timing {px} px\t{glyphs} glyphs x {repeats}\tCoreText {:.0} ns/glyph\tzeno {:.0} ns/glyph\tvello_cpu {:.0} ns/glyph",
            per(ct),
            per(zeno),
            per(vello)
        );
    }
}

/// Magnified tiles for the owner's visual comparison: for two glyphs of each
/// line, at 12, 16 and 24 pt at 1×, and 12 pt at 2×: CoreText (smoothing off),
/// CoreText (smoothing on, RGB), zeno, `vello_cpu`, then zeno and `vello_cpu`
/// each overlaid on CoreText (ink only CoreText's red, only the other's blue).
fn sheets(samples: &[Sample], dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut lines: Vec<&'static str> = samples.iter().map(|s| s.line).collect();
    lines.dedup();
    for line in lines {
        let picked: Vec<&Sample> = samples
            .iter()
            .filter(|s| s.line == line && !s.color)
            .take(2)
            .collect();
        let mut tiles: Vec<(Compared, usize)> = Vec::new();
        for sample in &picked {
            for (size, scale) in [(12.0, 1.0), (16.0, 1.0), (24.0, 1.0), (12.0, 2.0)] {
                if let Some(c) = compare(sample, size, scale) {
                    let magnify = (120 / c.height.max(1)).clamp(2, 8);
                    tiles.push((c, magnify));
                }
            }
        }
        if tiles.is_empty() {
            continue;
        }
        let gap = 8;
        let width = tiles
            .iter()
            .map(|(c, m)| (c.width * m + gap) * 6)
            .max()
            .unwrap_or(0);
        let height: usize = tiles.iter().map(|(c, m)| c.height * m + gap).sum();
        let mut sheet = Sheet::new(width, height);
        let mut y = 0;
        for (c, m) in &tiles {
            let step = c.width * m + gap;
            sheet.mask(0, y, &c.ct_off, c.width, *m, [0, 0, 0]);
            sheet.mask(step, y, &c.ct_rgb_on, c.width, *m, [0, 0, 0]);
            sheet.mask(step * 2, y, &c.zeno, c.width, *m, [0, 0, 160]);
            sheet.mask(step * 3, y, &c.vello, c.width, *m, [0, 110, 0]);
            sheet.overlay(step * 4, y, &c.ct_off, &c.zeno, c.width, *m);
            sheet.overlay(step * 5, y, &c.ct_off, &c.vello, c.width, *m);
            y += c.height * m + gap;
        }
        sheet
            .write(&dir.join(format!("raster-{line}.png")))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_masks_do_not_differ() {
        let d = diff(&[0, 100, 255], &[0, 100, 255]);
        assert_eq!((d.mean, d.max, d.visible, d.ink), (0.0, 0, 0.0, 1.0));
    }

    #[test]
    fn differences_are_measured_against_the_reference() {
        let d = diff(&[0, 100, 200], &[0, 140, 100]);
        assert_eq!(d.max, 100);
        assert!((d.mean - 140.0 / 3.0).abs() < 1e-9);
        // Two pixels differ by more than `VISIBLE` (40 and 100).
        assert!((d.visible - 2.0 / 3.0).abs() < 1e-9);
        assert!((d.ink - 240.0 / 300.0).abs() < 1e-9);
    }

    #[test]
    fn the_centroid_weights_by_coverage() {
        // Coverage 255 at (1, 0) and at (1, 1) in a 2 × 2 mask.
        assert_eq!(centroid(&[0, 255, 0, 255], 2), (1.0, 0.5));
        assert_eq!(centroid(&[0; 4], 2), (0.0, 0.0));
    }
}

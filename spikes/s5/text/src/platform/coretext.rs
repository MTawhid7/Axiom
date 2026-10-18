//! CoreText and CoreGraphics: fonts and their fallback, lines and their runs,
//! table copies, and glyphs drawn into 8-bit masks in buffers Rust owns.

use std::ffi::c_void;
use std::ops::Range;
use std::path::PathBuf;
use std::ptr::{self, NonNull};

use objc2_core_foundation::{
    CFArray, CFAttributedString, CFBoolean, CFDictionary, CFIndex, CFNumber, CFRange, CFRetained,
    CFString, CFType, CFURL, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGGlyph, CGImageAlphaInfo,
};
use objc2_core_text::{
    CTFont, CTFontOrientation, CTFontTableOptions, CTFontUIFontType, CTLine, CTRun, CTRunStatus,
    kCTFontAttributeName, kCTFontURLAttribute, kCTForegroundColorFromContextAttributeName,
    kCTLanguageAttributeName,
};

/// One face at one size.
#[derive(Clone, Debug)]
pub(crate) struct Font(CFRetained<CTFont>);

impl Font {
    /// The font with this PostScript or full name, as CoreText matches it.
    pub(crate) fn named(name: &str, size: f64) -> Self {
        // SAFETY: C6. A null matrix means the identity transform
        // (CTFontCreateWithName's contract); the name is a valid CFString.
        Self(unsafe { CTFont::with_name(&CFString::from_str(name), size, ptr::null()) })
    }

    /// The system's user-interface font for `language`.
    pub(crate) fn system(size: f64, language: Option<&str>) -> Option<Self> {
        let language = language.map(CFString::from_str);
        // SAFETY: C6. The language, if any, is a valid CFString.
        unsafe {
            CTFont::new_ui_font_for_language(CTFontUIFontType::System, size, language.as_deref())
        }
        .map(Self)
    }

    pub(crate) fn postscript_name(&self) -> String {
        // SAFETY: C6. `self.0` is a valid CTFont.
        unsafe { self.0.post_script_name() }.to_string()
    }

    pub(crate) fn size(&self) -> f64 {
        // SAFETY: C6. `self.0` is a valid CTFont.
        unsafe { self.0.size() }
    }

    pub(crate) fn units_per_em(&self) -> u32 {
        // SAFETY: C6. `self.0` is a valid CTFont.
        unsafe { self.0.units_per_em() }
    }

    /// The file the font comes from, if CoreText reports one.
    pub(crate) fn file(&self) -> Option<PathBuf> {
        // SAFETY: C2.
        let key = unsafe { kCTFontURLAttribute };
        // SAFETY: C6. `key` is one of CTFont's documented attribute names.
        let url = unsafe { self.0.attribute(key) }?;
        url.downcast::<CFURL>().ok()?.to_file_path()
    }

    /// The font's variation-axis values, by tag, as CoreText applies them (for
    /// example the optical size it chooses for the system font).
    pub(crate) fn variation(&self) -> Vec<([u8; 4], f64)> {
        // SAFETY: C6. `self.0` is a valid CTFont.
        let Some(dictionary) = (unsafe { self.0.variation() }) else {
            return Vec::new();
        };
        // SAFETY: C4. A font's variation dictionary maps CFNumber axis
        // identifiers to CFNumber values, both CF objects.
        let dictionary =
            unsafe { CFRetained::cast_unchecked::<CFDictionary<CFType, CFType>>(dictionary) };
        let (keys, values) = dictionary.to_vecs();
        let mut axes: Vec<([u8; 4], f64)> = keys
            .into_iter()
            .zip(values)
            .filter_map(|(key, value)| {
                let tag = key.downcast::<CFNumber>().ok()?.as_i64()?;
                let value = value.downcast::<CFNumber>().ok()?.as_f64()?;
                Some((u32::try_from(tag).ok()?.to_be_bytes(), value))
            })
            .collect();
        axes.sort_by_key(|axis| axis.0);
        axes
    }

    /// Every table of the font, copied through `CTFontCopyTable` (TX6: the font
    /// data of a font without a readable file).
    pub(crate) fn tables(&self) -> Vec<([u8; 4], Vec<u8>)> {
        // SAFETY: C6. `self.0` is a valid CTFont.
        let Some(tags) = (unsafe { self.0.available_tables(CTFontTableOptions::NoOptions) }) else {
            return Vec::new();
        };
        let count = CFIndex::try_from(tags.len()).unwrap_or(0);
        let mut tables = Vec::new();
        for i in 0..count {
            // SAFETY: C4. The array holds unboxed tags; `i` is in bounds.
            let value: *const c_void = unsafe { tags.value_at_index(i) };
            let tag = u32::try_from(value.addr()).unwrap_or(0);
            // SAFETY: C6. `self.0` is a valid CTFont, and the tag came from it.
            if let Some(data) = unsafe { self.0.table(tag, CTFontTableOptions::NoOptions) } {
                tables.push((tag.to_be_bytes(), data.to_vec()));
            }
        }
        tables
    }

    /// The font CoreText's fallback chooses for the start of `range` (UTF-16)
    /// of `text`, for `language` (TX6).
    pub(crate) fn for_text(
        &self,
        text: &CFString,
        range: Range<usize>,
        language: Option<&str>,
    ) -> Self {
        let range = cf_range(range);
        let language = language.map(CFString::from_str);
        // SAFETY: C6. The string and language are valid CFStrings, and the range
        // lies within the string (the callers pass ranges of it).
        Self(unsafe {
            self.0
                .for_string_with_language(text, range, language.as_deref())
        })
    }

    /// The same face at another size. CoreText applies the size's optical
    /// variation, if the font has one.
    pub(crate) fn with_size(&self, size: f64) -> Self {
        // SAFETY: C6. A null matrix and no attributes keep the font's own
        // (CTFontCreateCopyWithAttributes's contract).
        Self(unsafe { self.0.copy_with_attributes(size, ptr::null(), None) })
    }

    /// Whether the font maps every UTF-16 unit of `text` (surrogate pairs
    /// included) to a glyph.
    pub(crate) fn covers(&self, text: &[u16]) -> bool {
        if text.is_empty() {
            return true;
        }
        let mut glyphs = vec![CGGlyph::default(); text.len()];
        let count = CFIndex::try_from(text.len()).unwrap_or(0);
        // SAFETY: C1. `text` and `glyphs` hold `count` entries each, and
        // outlive the call.
        unsafe {
            self.0.glyphs_for_characters(
                NonNull::from(&text[0]),
                NonNull::from(&mut glyphs[0]),
                count,
            )
        }
    }

    /// The glyph's bounding rectangle in points, y up.
    pub(crate) fn glyph_bounds(&self, glyph: u16) -> CGRect {
        let glyphs = [glyph];
        // SAFETY: C1. One glyph in, no per-glyph output buffer requested (null).
        unsafe {
            self.0.bounding_rects_for_glyphs(
                CTFontOrientation::Default,
                NonNull::from(&glyphs[0]),
                ptr::null_mut(),
                1,
            )
        }
    }
}

/// A run of one font in a laid-out line.
#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) font: Font,
    pub(crate) glyphs: Vec<u16>,
    /// UTF-16 offsets into the line's text, one per glyph.
    pub(crate) string_indices: Vec<usize>,
    /// Glyph origins in points from the line's origin, y up.
    pub(crate) positions: Vec<(f64, f64)>,
    pub(crate) advances: Vec<(f64, f64)>,
    pub(crate) rtl: bool,
    /// The run's UTF-16 range in the line's text.
    pub(crate) range: Range<usize>,
}

/// A line laid out by CoreText: shaped, with fallback fonts and bidi.
pub(crate) struct Line {
    line: CFRetained<CTLine>,
}

impl Line {
    pub(crate) fn new(text: &str, font: &Font, language: Option<&str>) -> Self {
        // SAFETY: C2.
        let (font_key, ink_key, language_key): (&CFString, &CFString, &CFString) = unsafe {
            (
                kCTFontAttributeName,
                kCTForegroundColorFromContextAttributeName,
                kCTLanguageAttributeName,
            )
        };
        let mut keys = vec![font_key, ink_key];
        let language = language.map(CFString::from_str);
        let mut values: Vec<&CFType> = vec![(*font.0).as_ref(), CFBoolean::new(true).as_ref()];
        if let Some(language) = &language {
            keys.push(language_key);
            values.push((**language).as_ref());
        }
        let attributes = CFDictionary::from_slices(&keys, &values);
        // SAFETY: C6. The dictionary maps CoreText attribute names to values of
        // the types CoreText documents for them (a CTFont, a CFBoolean, a
        // CFString language), which CFAttributedStringCreate requires.
        let styled = unsafe {
            CFAttributedString::new(
                None,
                Some(&CFString::from_str(text)),
                Some(attributes.as_opaque()),
            )
        }
        .expect("CFAttributedStringCreate");
        // SAFETY: C6. `styled` is a valid attributed string.
        let line = unsafe { CTLine::with_attributed_string(&styled) };
        Self { line }
    }

    /// The line's runs, in visual order.
    pub(crate) fn runs(&self) -> Vec<Run> {
        // SAFETY: C6. `self.line` is a valid CTLine.
        let runs = unsafe { self.line.glyph_runs() };
        // SAFETY: C4. A line's glyph runs are CF objects.
        let runs = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(runs) };
        runs.iter()
            .filter_map(|run| run.downcast::<CTRun>().ok())
            .filter_map(|run| read_run(&run))
            .collect()
    }

    /// The typographic width, ascent and descent, in points.
    pub(crate) fn bounds(&self) -> (f64, f64, f64) {
        let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
        // SAFETY: C1. Three valid pointers to local CGFloats, used only
        // during the call.
        let width = unsafe {
            self.line
                .typographic_bounds(&raw mut ascent, &raw mut descent, &raw mut leading)
        };
        (width, ascent, descent)
    }

    /// Draw the line onto `canvas`, its origin at `origin` (canvas pixels from
    /// the top-left, y down).
    pub(crate) fn draw(&self, canvas: &mut Canvas, origin: (f64, f64)) {
        canvas.draw(origin, |context| {
            // SAFETY: C6 and C1. The line and context are valid; the context
            // draws into the canvas's buffer.
            unsafe { self.line.draw(context) };
        });
    }
}

fn read_run(run: &CTRun) -> Option<Run> {
    // SAFETY: C6. `run` is a valid CTRun.
    let count = unsafe { run.glyph_count() };
    let n = usize::try_from(count).ok()?;
    let all = CFRange {
        location: 0,
        length: 0, // all glyphs
    };
    let mut glyphs = vec![CGGlyph::default(); n];
    let mut positions = vec![CGPoint::default(); n];
    let mut advances = vec![CGSize::default(); n];
    let mut indices: Vec<CFIndex> = vec![0; n];
    if n > 0 {
        // SAFETY: C3.
        unsafe { run.glyphs(all, NonNull::new(glyphs.as_mut_ptr())?) };
        // SAFETY: C3.
        unsafe { run.positions(all, NonNull::new(positions.as_mut_ptr())?) };
        // SAFETY: C3.
        unsafe { run.advances(all, NonNull::new(advances.as_mut_ptr())?) };
        // SAFETY: C3.
        unsafe { run.string_indices(all, NonNull::new(indices.as_mut_ptr())?) };
    }
    // SAFETY: C6. `run` is a valid CTRun.
    let attributes = unsafe { run.attributes() };
    // SAFETY: C4. A run's attributes map CFString names to CF objects.
    let attributes =
        unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(attributes) };
    // SAFETY: C2.
    let font_key = unsafe { kCTFontAttributeName };
    let font = attributes.get(font_key)?.downcast::<CTFont>().ok()?;
    // SAFETY: C6. `run` is a valid CTRun.
    let status = unsafe { run.status() };
    // SAFETY: C6. `run` is a valid CTRun.
    let range = unsafe { run.string_range() };
    let start = usize::try_from(range.location).ok()?;
    let len = usize::try_from(range.length).ok()?;
    Some(Run {
        font: Font(font),
        glyphs,
        string_indices: indices
            .into_iter()
            .map(|i| usize::try_from(i).unwrap_or(0))
            .collect(),
        positions: positions.into_iter().map(|p| (p.x, p.y)).collect(),
        advances: advances.into_iter().map(|a| (a.width, a.height)).collect(),
        rtl: status.contains(CTRunStatus::RightToLeft),
        range: start..start + len,
    })
}

fn cf_range(range: Range<usize>) -> CFRange {
    CFRange {
        location: CFIndex::try_from(range.start).unwrap_or(CFIndex::MAX),
        length: CFIndex::try_from(range.len()).unwrap_or(0),
    }
}

/// How CoreGraphics draws text into a [`Canvas`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Smoothing {
    /// Antialiased, font smoothing off.
    Off,
    /// Antialiased, with font smoothing ("dilation") allowed and requested.
    On,
}

/// A white-on-black 8-bit mask that CoreGraphics draws into: one byte per
/// pixel, rows top to bottom. In RGB mode CoreGraphics draws into an RGBX
/// buffer, and the mask is its red channel, as font smoothing may differ
/// between gray and colour contexts.
#[derive(Debug)]
pub(crate) struct Canvas {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u8>,
    rgb: bool,
    smoothing: Smoothing,
    /// Device pixels per point: 2 for a Retina backing store, where the font
    /// keeps its point size (and optical size) and the context is scaled.
    scale: f64,
}

impl Canvas {
    pub(crate) fn new(width: usize, height: usize, rgb: bool, smoothing: Smoothing) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
            rgb,
            smoothing,
            scale: 1.0,
        }
    }

    /// The same canvas, drawn at `scale` device pixels per point.
    pub(crate) fn scaled(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// Run `draw` with a bitmap context over this canvas, its text position at
    /// `origin` (pixels from the top-left, y down).
    fn draw(&mut self, origin: (f64, f64), draw: impl FnOnce(&CGContext)) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let channels = if self.rgb { 4 } else { 1 };
        // Draw over what the canvas already holds.
        let mut buffer: Vec<u8> = if self.rgb {
            self.pixels.iter().flat_map(|&v| [v, v, v, 255]).collect()
        } else {
            self.pixels.clone()
        };
        let (space, info) = if self.rgb {
            (
                CGColorSpace::new_device_rgb(),
                CGImageAlphaInfo::NoneSkipLast.0,
            )
        } else {
            (CGColorSpace::new_device_gray(), CGImageAlphaInfo::None.0)
        };
        let Some(space) = space else {
            return;
        };
        // SAFETY: C1. `buffer` holds width × height × channels bytes: 8 bits
        // per component, `width × channels` bytes per row. `context` is
        // dropped below, before `buffer` is read.
        let context = unsafe {
            CGBitmapContextCreate(
                buffer.as_mut_ptr().cast(),
                self.width,
                self.height,
                8,
                self.width * channels,
                Some(&space),
                info,
            )
        };
        let Some(context) = context else {
            return;
        };
        let smooth = self.smoothing == Smoothing::On;
        CGContext::set_allows_font_smoothing(Some(&context), smooth);
        CGContext::set_should_smooth_fonts(Some(&context), smooth);
        CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
        CGContext::set_should_subpixel_position_fonts(Some(&context), true);
        CGContext::set_allows_font_subpixel_quantization(Some(&context), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&context), false);
        CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
        CGContext::scale_ctm(Some(&context), self.scale, self.scale);
        // In points from here on.
        let height = f64::from(u32::try_from(self.height).unwrap_or(u32::MAX)) / self.scale;
        CGContext::set_text_position(
            Some(&context),
            origin.0 / self.scale,
            height - origin.1 / self.scale,
        );
        draw(&context);
        drop(context);
        if self.rgb {
            for (out, px) in self.pixels.iter_mut().zip(buffer.chunks_exact(4)) {
                *out = px[0];
            }
        } else {
            self.pixels = buffer;
        }
    }

    /// Draw glyphs of `font` at `positions` (points, relative to `origin`,
    /// y up), with the canvas's text origin at `origin` (device pixels from
    /// the top-left, y down).
    pub(crate) fn draw_glyphs(
        &mut self,
        font: &Font,
        glyphs: &[u16],
        positions: &[(f64, f64)],
        origin: (f64, f64),
    ) {
        let count = glyphs.len().min(positions.len());
        if count == 0 {
            return;
        }
        let pixel_height = f64::from(u32::try_from(self.height).unwrap_or(u32::MAX));
        let height = pixel_height / self.scale;
        let points: Vec<CGPoint> = positions[..count]
            .iter()
            .map(|&(x, y)| CGPoint {
                x: origin.0 / self.scale + x,
                y: height - origin.1 / self.scale + y,
            })
            .collect();
        // CTFontDrawGlyphs adds the text position to each glyph position, so
        // the text position goes at CoreGraphics' origin, the canvas's
        // bottom-left corner.
        self.draw((0.0, pixel_height), |context| {
            // SAFETY: C1. `glyphs` and `points` hold `count` entries each and
            // outlive the call; the context draws into the canvas.
            unsafe {
                font.0.draw_glyphs(
                    NonNull::from(&glyphs[0]),
                    NonNull::from(&points[0]),
                    count,
                    context,
                );
            }
        });
    }
}

/// One glyph drawn by CoreText into premultiplied RGBA on a transparent
/// canvas, for colour glyphs (Apple Color Emoji's `sbix` bitmaps), at `scale`
/// device pixels per point. The origin is in device pixels from the top-left,
/// y down.
pub(crate) fn draw_color_glyph(
    font: &Font,
    glyph: u16,
    width: usize,
    height: usize,
    origin: (f64, f64),
    scale: f64,
) -> Vec<u8> {
    let mut buffer = vec![0_u8; width * height * 4];
    if width == 0 || height == 0 {
        return buffer;
    }
    let Some(space) = CGColorSpace::new_device_rgb() else {
        return buffer;
    };
    // SAFETY: C1. `buffer` holds width × height × 4 bytes: 8 bits per
    // component, `width × 4` bytes per row. `context` is dropped below,
    // before `buffer` is returned.
    let context = unsafe {
        CGBitmapContextCreate(
            buffer.as_mut_ptr().cast(),
            width,
            height,
            8,
            width * 4,
            Some(&space),
            CGImageAlphaInfo::PremultipliedLast.0,
        )
    };
    let Some(context) = context else {
        return buffer;
    };
    CGContext::scale_ctm(Some(&context), scale, scale);
    let h = f64::from(u32::try_from(height).unwrap_or(u32::MAX)) / scale;
    let point = CGPoint {
        x: origin.0 / scale,
        y: h - origin.1 / scale,
    };
    let glyphs = [glyph];
    // SAFETY: C1. One glyph and one position, both live across the call; the
    // context draws into `buffer`.
    unsafe {
        font.0.draw_glyphs(
            NonNull::from(&glyphs[0]),
            NonNull::from(&point),
            1,
            &context,
        );
    }
    drop(context);
    buffer
}

/// The time CoreText takes to draw `glyph` `repeats` times into one gray
/// bitmap context, cleared before each draw: rasterization alone, without
/// creating a context per glyph.
pub(crate) fn time_glyph(
    font: &Font,
    glyph: u16,
    width: usize,
    height: usize,
    origin: (f64, f64),
    repeats: usize,
) -> std::time::Duration {
    let mut buffer = vec![0_u8; width.max(1) * height.max(1)];
    let Some(space) = CGColorSpace::new_device_gray() else {
        return std::time::Duration::ZERO;
    };
    // SAFETY: C1. `buffer` holds width × height bytes, one per pixel; the
    // context is dropped before `buffer`.
    let context = unsafe {
        CGBitmapContextCreate(
            buffer.as_mut_ptr().cast(),
            width.max(1),
            height.max(1),
            8,
            width.max(1),
            Some(&space),
            CGImageAlphaInfo::None.0,
        )
    };
    let Some(context) = context else {
        return std::time::Duration::ZERO;
    };
    CGContext::set_allows_font_smoothing(Some(&context), false);
    CGContext::set_should_smooth_fonts(Some(&context), false);
    let h = f64::from(u32::try_from(height).unwrap_or(u32::MAX));
    let w = f64::from(u32::try_from(width).unwrap_or(u32::MAX));
    let whole = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: w,
            height: h,
        },
    };
    let point = CGPoint {
        x: origin.0,
        y: h - origin.1,
    };
    let glyphs = [glyph];
    let start = std::time::Instant::now();
    for _ in 0..repeats {
        CGContext::set_gray_fill_color(Some(&context), 0.0, 1.0);
        CGContext::fill_rect(Some(&context), whole);
        CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
        // SAFETY: C1. One glyph and one position, live across the call.
        unsafe {
            font.0.draw_glyphs(
                NonNull::from(&glyphs[0]),
                NonNull::from(&point),
                1,
                &context,
            );
        }
    }
    let elapsed = start.elapsed();
    drop(context);
    elapsed
}

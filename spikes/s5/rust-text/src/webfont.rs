//! The whole Rust path for one web-font file (S5-Q3): sniff the container,
//! decode it, validate each font, shape sample text in it with HarfRust, and
//! rasterize every glyph that shaping produced with both candidate fillers.
//!
//! A malformed file must end as a [`Rejection`] or as a [`Report`]; never as
//! a crash or a hang. Glyphs that fail to draw are counted, not fatal: a
//! renderer would draw nothing for them.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use read_fonts::{FileRef, FontRef};
use skrifa::GlyphId;
use skrifa::charmap::Charmap;
use skrifa::instance::LocationRef;

use crate::raster::{self, Filler};
use crate::shape::{self, Params};
use crate::validate::{self, Summary};
use crate::{Limits, woff, woff2};

/// The kinds of file a web font can arrive as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Woff,
    Woff2,
    /// A bare OpenType font (`format("opentype")` or `"truetype"`).
    Sfnt,
    /// A bare font collection.
    Collection,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Woff => "woff",
            Self::Woff2 => "woff2",
            Self::Sfnt => "sfnt",
            Self::Collection => "collection",
        }
    }
}

/// The container, from the first four bytes.
pub fn sniff(data: &[u8]) -> Option<Kind> {
    let magic = u32::from_be_bytes(data.get(..4)?.try_into().ok()?);
    match magic {
        woff::SIGNATURE => Some(Kind::Woff),
        woff2::SIGNATURE => Some(Kind::Woff2),
        crate::sfnt::TRUETYPE | crate::sfnt::CFF => Some(Kind::Sfnt),
        m if m == u32::from_be_bytes(*b"true") => Some(Kind::Sfnt),
        crate::sfnt::COLLECTION => Some(Kind::Collection),
        _ => None,
    }
}

/// Where and why a file was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub stage: &'static str,
    pub reason: String,
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.stage, self.reason)
    }
}

fn reject(stage: &'static str, reason: impl fmt::Display) -> Rejection {
    Rejection {
        stage,
        reason: reason.to_string(),
    }
}

/// What the path did with an accepted file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub kind: Option<Kind>,
    pub fonts: usize,
    pub decoded_bytes: usize,
    /// Sample texts shaped, over all fonts.
    pub texts: usize,
    pub glyphs_shaped: usize,
    /// Masks produced (each distinct glyph, at each size, by each filler).
    pub masks: usize,
    /// Glyphs that failed to draw or fill.
    pub glyph_errors: usize,
    pub ink: u64,
    /// FNV-1a over every mask, so runs can be compared.
    pub hash: u64,
    pub decode_us: u128,
    pub shape_us: u128,
    pub raster_us: u128,
}

/// Sample texts; each font shapes those it covers.
const SAMPLES: &[&str] = &[
    "The quick brown fox jumps over the lazy dog. 0123456789",
    "PASS",
    "Ελληνικά και Русский текст",
    "اَللُّغَةُ الْعَرَبِيَّةُ جَمِيلَةٌ",
    "עִבְרִית",
    "हिन्दी क्षत्रिय",
    "தமிழ்",
    "ภาษาไทย",
    "ᠮᠣᠩᠭᠣᠯ ᠪᠢᠴᠢᠭ",
    "ߒߞߏ ߛߓߍ",
    "日本語の文章",
    "한국어",
];
/// Fonts that cover none of the samples shape their first mapped characters.
const FIRST_MAPPED: usize = 96;
/// At most this many distinct glyphs are rasterized per font.
const MAX_GLYPHS: usize = 512;
const SIZES_PX: [f32; 2] = [16.0, 48.0];

/// Run the path on one file.
pub fn check(data: &[u8], limits: &Limits) -> Result<Report, Rejection> {
    let mut report = Report::default();
    let start = Instant::now();
    let kind = sniff(data).ok_or_else(|| reject("sniff", "unknown format"))?;
    report.kind = Some(kind);
    let fonts: Vec<(Arc<Vec<u8>>, u32)> = match kind {
        Kind::Woff => vec![(
            Arc::new(woff::decode(data, limits).map_err(|e| reject("decode", e))?),
            0,
        )],
        Kind::Woff2 => woff2::decode(data, limits)
            .map_err(|e| reject("decode", e))?
            .into_iter()
            .map(|font| (Arc::new(font), 0))
            .collect(),
        Kind::Sfnt => {
            if data.len() > limits.max_sfnt {
                return Err(reject("decode", "font larger than the limit"));
            }
            vec![(Arc::new(data.to_vec()), 0)]
        }
        Kind::Collection => {
            if data.len() > limits.max_sfnt {
                return Err(reject("decode", "collection larger than the limit"));
            }
            let blob = Arc::new(data.to_vec());
            let count = match FileRef::new(data) {
                Ok(FileRef::Collection(c)) => c.len(),
                _ => return Err(reject("decode", "collection header does not parse")),
            };
            (0..count).map(|i| (Arc::clone(&blob), i)).collect()
        }
    };
    report.decode_us = start.elapsed().as_micros();
    report.fonts = fonts.len();
    let mut hash = Fnv::new();
    for (blob, index) in &fonts {
        report.decoded_bytes += blob.len();
        let summary =
            validate::validate(blob, *index, limits).map_err(|e| reject("validate", e))?;
        exercise(blob, *index, summary, &mut report, &mut hash)?;
    }
    report.hash = hash.0;
    Ok(report)
}

fn exercise(
    blob: &Arc<Vec<u8>>,
    index: u32,
    summary: Summary,
    report: &mut Report,
    hash: &mut Fnv,
) -> Result<(), Rejection> {
    let font_ref = FontRef::from_index(blob, index).map_err(|e| reject("validate", e))?;
    let shaping_font = shape::font(
        Arc::clone(blob) as Arc<dyn AsRef<[u8]> + Send + Sync>,
        index,
        &[],
    )
    .ok_or_else(|| reject("shape", "HarfRust could not open the font"))?;
    let charmap = Charmap::new(&font_ref);
    let mut texts: Vec<String> = SAMPLES
        .iter()
        .filter(|text| covered(&charmap, text))
        .map(|text| (*text).to_owned())
        .collect();
    if texts.is_empty() {
        let first: String = charmap
            .mappings()
            .filter_map(|(cp, _)| char::from_u32(cp))
            .filter(|c| !c.is_control())
            .take(FIRST_MAPPED)
            .collect();
        if !first.is_empty() {
            texts.push(first);
        }
    }

    let start = Instant::now();
    let mut glyphs = BTreeSet::new();
    for text in &texts {
        let shaped = shape::shape(
            &shaping_font,
            text,
            0..shape::utf16_len(text),
            &Params::default(),
        )
        .map_err(|e| reject("shape", format!("{e:?}")))?;
        report.texts += 1;
        report.glyphs_shaped += shaped.len();
        for glyph in shaped {
            if glyph.id < u32::from(summary.num_glyphs) && glyphs.len() < MAX_GLYPHS {
                glyphs.insert(glyph.id);
            }
        }
    }
    report.shape_us += start.elapsed().as_micros();

    let start = Instant::now();
    for &id in &glyphs {
        for size in SIZES_PX {
            let path =
                match raster::outline(&font_ref, GlyphId::new(id), size, LocationRef::default()) {
                    Ok(path) => path,
                    // Bitmap fonts, such as colour emoji, have no outlines (TX13).
                    Err(raster::RasterError::NoOutline) => {
                        match raster::color_glyph(&font_ref, GlyphId::new(id), size) {
                            Ok(glyph) => {
                                report.masks += 1;
                                report.ink += glyph.ink() as u64;
                                hash.write(&glyph.rgba);
                            }
                            Err(_) => report.glyph_errors += 1,
                        }
                        continue;
                    }
                    Err(_) => {
                        report.glyph_errors += 1;
                        continue;
                    }
                };
            let Some(bounds) = path.bounds else {
                continue; // An empty glyph, such as a space.
            };
            // A canvas around the glyph's bounds, one pixel of margin; glyphs
            // larger than eight ems are not drawn (as a renderer would cap them).
            let width = (bounds.max_x - bounds.min_x).ceil() + 2.0;
            let height = (bounds.max_y - bounds.min_y).ceil() + 2.0;
            if !(width.is_finite() && height.is_finite())
                || width > size * 8.0
                || height > size * 8.0
            {
                report.glyph_errors += 1;
                continue;
            }
            let origin = (1.0 - bounds.min_x.floor(), 1.0 + bounds.max_y.ceil());
            for filler in [Filler::Zeno, Filler::Vello] {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "checked finite, positive and small above"
                )]
                match raster::fill(&path, origin, width as usize, height as usize, filler) {
                    Ok(canvas) => {
                        report.masks += 1;
                        report.ink += canvas.ink() as u64;
                        hash.write(&canvas.pixels);
                    }
                    Err(_) => report.glyph_errors += 1,
                }
            }
        }
    }
    report.raster_us += start.elapsed().as_micros();
    Ok(())
}

/// Whether the font maps nine in ten of `text`'s non-space characters.
fn covered(charmap: &Charmap<'_>, text: &str) -> bool {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mapped = chars.iter().filter(|&&c| charmap.map(c).is_some()).count();
    !chars.is_empty() && mapped * 10 >= chars.len() * 9
}

/// FNV-1a, to compare masks between runs.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testfont;

    #[test]
    fn accepts_the_test_font_in_every_container() {
        let font = testfont::square_font();
        let limits = Limits::default();
        let files = [
            font.clone(),
            woff::encode(&font, true).expect("encodes"),
            woff2::encode_stored(&font).expect("encodes"),
        ];
        let mut hashes = Vec::new();
        for file in files {
            let report = check(&file, &limits).expect("accepted");
            assert_eq!(report.fonts, 1);
            assert!(report.masks > 0, "{report:?}");
            assert_eq!(report.glyph_errors, 0, "{report:?}");
            hashes.push(report.hash);
        }
        // The same font renders identically whatever its container.
        assert!(hashes.windows(2).all(|w| w[0] == w[1]), "{hashes:?}");
    }

    #[test]
    fn rejects_unknown_formats() {
        assert_eq!(
            check(b"GIF89a", &Limits::default()).unwrap_err().stage,
            "sniff"
        );
    }
}

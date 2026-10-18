//! S5-Q4: what each configuration of the text stack adds to a sandboxed
//! helper's footprint after shaping and rasterizing the corpus, and how fast
//! shaping is.
//!
//! Configurations:
//! - `none`: the helper enters its profile and does nothing (the baseline).
//! - `coretext`: CoreText lays out each line (fallback and shaping) and draws
//!   each distinct glyph into an atlas of 8-bit masks.
//! - `harfrust-coretext`: CoreText's fallback chooses a font for each
//!   grapheme cluster (ICU4X's segmenter); HarfRust shapes each run; CoreText
//!   draws the glyphs.
//! - `rust`: the same fallback and shaping; skrifa and zeno draw the glyphs
//!   (colour glyphs from `sbix` through `png`).
//! - `rust-icu`: `rust`, plus ICU4X's line breaker (its "auto" model, with
//!   the LSTM and dictionary data for South-East Asian scripts and CJK) and
//!   word segmenter over every line.
//! - `coretext-latin`, `harfrust-coretext-latin`, `rust-latin`: the same, on
//!   the corpus's Latin line alone: each stack's cost for a renderer's first
//!   text, where the whole corpus is a heavily multilingual page.
//!
//! The helper's side follows S4's protocol: it reads compiled profile bytecode
//! from descriptor 3, applies it, does its work, says ready and waits; the
//! broker then reads its footprint (`s4_sandbox::bench::measure_startup`).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use icu_segmenter::options::{LineBreakOptions, WordBreakInvariantOptions};
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter, WordSegmenter};
use objc2_core_foundation::CFString;
use s4_sandbox::bench;
use s4_sandbox::platform::{process, sandbox};
use s4_sandbox::probe::{Line as State, SOCKET_FD};
use s4_sandbox::profile::RENDEZVOUS_PARAM;
use s5_rust_text::raster::{self, Filler};
use s5_rust_text::shape::{self, Params};
use skrifa::instance::LocationRef;
use skrifa::{GlyphId, MetadataProvider, Tag};

use crate::corpus::{self, Sample};
use crate::fonts::{self, FontData};
use crate::platform::coretext::{Canvas, Font, Line, Smoothing};
use crate::{Options, shaping};

/// The configurations, in the order the broker measures them.
const CONFIGS: [&str; 8] = [
    "none",
    "coretext-latin",
    "harfrust-coretext-latin",
    "rust-latin",
    "coretext",
    "harfrust-coretext",
    "rust",
    "rust-icu",
];
/// The size the workload shapes and draws at, in points (1×).
const SIZE: f64 = 16.0;

/// What a workload made, kept alive while the footprint is read.
#[derive(Default)]
struct Work {
    glyphs_shaped: usize,
    /// Distinct glyph masks, by font and glyph.
    atlas: HashMap<(String, u16), Vec<u8>>,
    line_breaks: usize,
    words: usize,
    /// One mapping (or table copy) per font, as a renderer keeps them.
    fonts: HashMap<String, FontData>,
}

impl Work {
    fn atlas_bytes(&self) -> usize {
        self.atlas.values().map(Vec::len).sum()
    }
}

/// `ready --config NAME`: the sandboxed helper.
pub(crate) fn ready(options: &Options) -> Result<(), String> {
    let config = options.config.clone().unwrap_or_else(|| "none".to_owned());
    let mut socket = process::adopt_inherited_socket(SOCKET_FD).map_err(|e| e.to_string())?;
    let bytecode = bench::read_bytecode(&mut socket).map_err(|e| e.to_string())?;
    sandbox::apply_bytecode(&bytecode)?;
    let (work, elapsed) = work(&config)?;
    report(&work, elapsed);
    std::hint::black_box(&work);
    bench::ready_and_wait_on(socket).map_err(|e| e.to_string())
}

/// `work --config NAME`: the same work, unsandboxed unless `--profile` is
/// given, with its report (for checking a configuration by hand).
pub(crate) fn work_mode(options: &Options) -> Result<(), String> {
    let config = options.config.clone().unwrap_or_else(|| "none".to_owned());
    let (work, elapsed) = work(&config)?;
    report(&work, elapsed);
    if options.detail {
        let mut keys: Vec<String> = work
            .atlas
            .iter()
            .map(|((font, glyph), mask)| {
                let ink = mask.iter().filter(|&&c| c > 0).count();
                format!("{font}\t{glyph}\tink {ink}")
            })
            .collect();
        keys.sort();
        for key in keys {
            println!("# atlas\t{key}");
        }
    }
    Ok(())
}

fn work(config: &str) -> Result<(Work, Duration), String> {
    let start = Instant::now();
    let work = run(config)?;
    Ok((work, start.elapsed()))
}

fn report(work: &Work, elapsed: Duration) {
    let state = |key: &str, value: String| println!("{}", State::State(key.to_owned(), value));
    state("sandboxed", sandbox::is_sandboxed().to_string());
    state("glyphs-shaped", work.glyphs_shaped.to_string());
    state("atlas-glyphs", work.atlas.len().to_string());
    state("atlas-bytes", work.atlas_bytes().to_string());
    state("line-breaks", work.line_breaks.to_string());
    state("words", work.words.to_string());
    state("fonts", work.fonts.len().to_string());
    state("work-microseconds", elapsed.as_micros().to_string());
}

fn run(config: &str) -> Result<Work, String> {
    let mut work = Work::default();
    let (config, samples): (&str, Vec<Sample>) = match config.strip_suffix("-latin") {
        Some(base) => (
            base,
            corpus::samples()
                .into_iter()
                .filter(|s| s.id == "latin")
                .collect(),
        ),
        None => (config, corpus::samples()),
    };
    match config {
        "none" => {}
        "coretext" => {
            for sample in &samples {
                coretext_line(sample, &mut work);
            }
        }
        "harfrust-coretext" | "rust" | "rust-icu" => {
            let segmenter = GraphemeClusterSegmenter::new();
            let rust_raster = config != "harfrust-coretext";
            for sample in &samples {
                harfrust_line(sample, segmenter, rust_raster, &mut work)?;
            }
            if config == "rust-icu" {
                let lines = LineSegmenter::new_auto(LineBreakOptions::default());
                let words = WordSegmenter::new_auto(WordBreakInvariantOptions::default());
                for sample in &samples {
                    work.line_breaks += lines.segment_str(sample.text).count();
                    work.words += words.segment_str(sample.text).count();
                }
            }
        }
        other => return Err(format!("unknown configuration {other:?}")),
    }
    Ok(work)
}

/// CoreText only: its line layout, then each new glyph drawn into the atlas.
fn coretext_line(sample: &Sample, work: &mut Work) {
    let line = Line::new(
        sample.text,
        &shaping::base_font(sample, SIZE),
        Some(sample.language),
    );
    for run in line.runs() {
        work.glyphs_shaped += run.glyphs.len();
        let name = run.font.postscript_name();
        for &glyph in &run.glyphs {
            work.atlas
                .entry((name.clone(), glyph))
                .or_insert_with(|| coretext_mask(&run.font, glyph));
        }
    }
}

fn coretext_mask(font: &Font, glyph: u16) -> Vec<u8> {
    let side = to_px(SIZE * 2.0);
    let mut canvas = Canvas::new(side, side, false, Smoothing::Off);
    canvas.draw_glyphs(font, &[glyph], &[(0.0, 0.0)], (SIZE / 2.0, SIZE * 1.5));
    canvas.pixels
}

/// HarfRust after CoreText's per-cluster fallback; CoreText or the Rust path
/// draws.
fn harfrust_line(
    sample: &Sample,
    segmenter: icu_segmenter::GraphemeClusterSegmenterBorrowed<'static>,
    rust_raster: bool,
    work: &mut Work,
) -> Result<(), String> {
    let base = shaping::base_font(sample, SIZE);
    for (font, range) in itemize(sample, &base, segmenter) {
        let name = font.postscript_name();
        let data = if let Some(data) = work.fonts.get(&name) {
            data.clone()
        } else {
            let data = fonts::font_data(&font)?;
            work.fonts.insert(name.clone(), data.clone());
            data
        };
        let hr_font = shape::font_hiding(
            data.blob.clone(),
            data.index,
            &variation(&font),
            &[*b"trak"],
        )
        .ok_or("HarfRust could not open a font")?;
        let params = Params {
            rtl: None,
            language: Some(sample.language),
            point_size: None,
        };
        let mut glyphs =
            shape::shape(&hr_font, sample.text, range, &params).map_err(|e| format!("{e:?}"))?;
        #[expect(clippy::cast_possible_truncation, reason = "font sizes are small")]
        shape::apply_tracking(
            &mut glyphs,
            shape::normal_tracking(data.bytes(), data.index, SIZE as f32),
        );
        work.glyphs_shaped += glyphs.len();
        for glyph in &glyphs {
            let id = u16::try_from(glyph.id).unwrap_or(0);
            if work.atlas.contains_key(&(name.clone(), id)) {
                continue;
            }
            let mask = if rust_raster {
                rust_mask(&data, &font, id)
            } else {
                coretext_mask(&font, id)
            };
            work.atlas.insert((name.clone(), id), mask);
        }
    }
    Ok(())
}

fn rust_mask(data: &FontData, font: &Font, glyph: u16) -> Vec<u8> {
    let Some(font_ref) = data.font_ref() else {
        return Vec::new();
    };
    let id = GlyphId::new(u32::from(glyph));
    #[expect(clippy::cast_possible_truncation, reason = "font sizes are small")]
    let px = SIZE as f32;
    let location = font_ref.axes().location(
        variation(font)
            .into_iter()
            .map(|(tag, value)| (Tag::new(&tag), value)),
    );
    match raster::outline(&font_ref, id, px, LocationRef::from(&location)) {
        Ok(path) => {
            let side = to_px(SIZE * 2.0);
            raster::fill(&path, (px / 2.0, px * 1.5), side, side, Filler::Zeno)
                .map_or_else(|_| Vec::new(), |c| c.pixels)
        }
        Err(raster::RasterError::NoOutline) => {
            raster::color_glyph(&font_ref, id, px).map_or_else(|_| Vec::new(), |g| g.rgba)
        }
        Err(_) => Vec::new(),
    }
}

fn variation(font: &Font) -> Vec<([u8; 4], f32)> {
    #[expect(clippy::cast_possible_truncation, reason = "axis values are small")]
    font.variation()
        .into_iter()
        .map(|(tag, value)| (tag, value as f32))
        .collect()
}

/// Split `sample` into runs of one font: CoreText's fallback for each
/// grapheme cluster the current font doesn't cover (TX6), as Chromium
/// resolves fallback per character.
fn itemize(
    sample: &Sample,
    base: &Font,
    segmenter: icu_segmenter::GraphemeClusterSegmenterBorrowed<'static>,
) -> Vec<(Font, std::ops::Range<usize>)> {
    let text = sample.text;
    let units: Vec<u16> = text.encode_utf16().collect();
    let cf_text = CFString::from_str(text);
    // Grapheme boundaries, converted from UTF-8 to UTF-16 offsets.
    let mut bounds: Vec<usize> = Vec::new();
    let mut utf16 = 0;
    let mut last_byte = 0;
    for byte in segmenter.segment_str(text) {
        utf16 += text[last_byte..byte].encode_utf16().count();
        last_byte = byte;
        bounds.push(utf16);
    }
    let mut runs: Vec<(Font, std::ops::Range<usize>)> = Vec::new();
    for pair in bounds.windows(2) {
        let cluster = pair[0]..pair[1];
        let current = runs.last().map_or_else(|| base.clone(), |(f, _)| f.clone());
        let font = if current.covers(&units[cluster.clone()]) {
            current
        } else if base.covers(&units[cluster.clone()]) {
            base.clone()
        } else {
            base.for_text(&cf_text, cluster.clone(), Some(sample.language))
        };
        match runs.last_mut() {
            Some((f, range)) if f.postscript_name() == font.postscript_name() => {
                range.end = cluster.end;
            }
            _ => runs.push((font, cluster)),
        }
    }
    runs
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "canvas sizes are small and positive"
)]
fn to_px(v: f64) -> usize {
    v.ceil().max(1.0) as usize
}

/// `footprint`: the broker. For each configuration, `--runs` helpers, each
/// sandboxed with S5's text profile (or `--profile`) compiled here, as
/// Axiom's browser compiles its helpers' profiles (ADR-0004, D2).
pub(crate) fn footprint(options: &Options) -> Result<(), String> {
    let source = options
        .profile_source()?
        .unwrap_or_else(|| crate::TEXT_S5.to_owned());
    let dir = crate::executable_dir()?;
    let compiled = sandbox::compile(
        &source,
        &[
            (RENDEZVOUS_PARAM, "org.axiom.s5.unused"),
            ("EXECUTABLE_DIR", &dir),
        ],
    )?;
    let bytecode = compiled.bytecode()?;
    let helper = std::env::current_exe().map_err(|e| e.to_string())?;
    println!(
        "# footprint\tconfig\trun\tcurrent-bytes\tlifetime-max-bytes\tspawn-to-ready-us\tstates"
    );
    for config in CONFIGS {
        let args = vec!["ready".to_owned(), "--config".to_owned(), config.to_owned()];
        for run in 0..options.runs.max(1) {
            let sample = bench::measure_startup(&helper, &args, Some(&bytecode))
                .map_err(|e| e.to_string())?;
            let states: Vec<String> = sample
                .states
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            println!(
                "footprint\t{config}\t{run}\t{}\t{}\t{}\t{}",
                sample.footprint.current,
                sample.footprint.lifetime_max,
                sample.spawn_to_ready.as_micros(),
                states.join(" ")
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    Ok(())
}

/// A corpus run as Axiom would itemize it, for `speed`: the font CoreText's
/// fallback chose, opened once for each shaper.
struct Job {
    text: &'static str,
    language: &'static str,
    /// UTF-16 offsets.
    range: std::ops::Range<usize>,
    /// The same range, in bytes.
    bytes: std::ops::Range<usize>,
    rtl: bool,
    ct_font: Font,
    hr_font: shape::Font,
}

/// `speed`: shaping cost per glyph. CoreText's line layout of each corpus
/// line (fallback included); CoreText again on the runs Axiom would itemize,
/// each in its resolved font (no fallback search); and HarfRust shaping those
/// same runs, with fonts opened and shapers prepared once.
pub(crate) fn speed(options: &Options) -> Result<(), String> {
    let repeat = options.repeat.max(1) * 20;
    let samples = corpus::samples();
    let mut jobs = Vec::new();
    for sample in &samples {
        let (_, runs, _) = shaping::compare_line(sample, SIZE, shaping::Tracking::Axiom);
        for run in runs {
            let data = fonts::font_data(&run.font)?;
            let hr_font = shape::font_hiding(
                data.blob.clone(),
                data.index,
                &variation(&run.font),
                &[*b"trak"],
            )
            .ok_or("HarfRust could not open a font")?;
            jobs.push(Job {
                text: sample.text,
                language: sample.language,
                bytes: utf8_range(sample.text, &run.range),
                range: run.range.clone(),
                rtl: run.rtl,
                ct_font: run.font.clone(),
                hr_font,
            });
        }
    }
    let count = |line: &Line| line.runs().iter().map(|r| r.glyphs.len()).sum::<usize>();
    let (mut line_glyphs, mut run_glyphs, mut hr_glyphs) = (0, 0, 0);
    let start = Instant::now();
    for _ in 0..repeat {
        for sample in &samples {
            let base = shaping::base_font(sample, SIZE);
            line_glyphs += count(&Line::new(sample.text, &base, Some(sample.language)));
        }
    }
    let line_time = start.elapsed();
    let start = Instant::now();
    for _ in 0..repeat {
        for job in &jobs {
            let text = &job.text[job.bytes.clone()];
            run_glyphs += count(&Line::new(text, &job.ct_font, Some(job.language)));
        }
    }
    let run_time = start.elapsed();
    let shapers: Vec<harfrust::ShaperFont<'_, '_>> = jobs
        .iter()
        .map(|job| harfrust::ShaperFont::new(&job.hr_font))
        .collect();
    let start = Instant::now();
    for _ in 0..repeat {
        for (job, shaper) in jobs.iter().zip(&shapers) {
            let params = Params {
                rtl: Some(job.rtl),
                language: Some(job.language),
                point_size: None,
            };
            hr_glyphs += shape::shape_with(shaper, job.text, job.range.clone(), &params)
                .map_err(|e| format!("{e:?}"))?
                .len();
        }
    }
    let hr_time = start.elapsed();
    let per = |d: Duration, n: usize| {
        let n = f64::from(u32::try_from(n).unwrap_or(u32::MAX)).max(1.0);
        d.as_secs_f64() * 1e9 / n
    };
    println!(
        "# speed\t{repeat} passes over the corpus\tCoreText line layout {:.0} ns/glyph ({line_glyphs} glyphs)\tCoreText on resolved runs {:.0} ns/glyph ({run_glyphs} glyphs)\tHarfRust on the same runs {:.0} ns/glyph ({hr_glyphs} glyphs)",
        per(line_time, line_glyphs),
        per(run_time, run_glyphs),
        per(hr_time, hr_glyphs)
    );
    Ok(())
}

/// The bytes of `text` that the UTF-16 range `range` covers.
fn utf8_range(text: &str, range: &std::ops::Range<usize>) -> std::ops::Range<usize> {
    let (mut utf16, mut start, mut end) = (0, text.len(), text.len());
    for (byte, ch) in text.char_indices() {
        if utf16 == range.start {
            start = byte;
        }
        if utf16 == range.end {
            end = byte;
            break;
        }
        utf16 += ch.len_utf16();
    }
    start.min(end)..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_ranges_map_to_bytes() {
        // "é" is two bytes and one UTF-16 unit; "😀" four bytes and two units.
        let text = "aé😀b";
        assert_eq!(utf8_range(text, &(0..1)), 0..1);
        assert_eq!(utf8_range(text, &(1..2)), 1..3);
        assert_eq!(utf8_range(text, &(2..4)), 3..7);
        assert_eq!(utf8_range(text, &(4..5)), 7..8);
        assert_eq!(utf8_range(text, &(0..5)), 0..8);
    }
}

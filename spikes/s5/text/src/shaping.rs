//! S5-Q1: HarfRust against CoreText on this Mac's system fonts.
//!
//! CoreText lays out each corpus line, with its fallback choosing a font for
//! each script. For each of CoreText's runs, HarfRust shapes the same text in
//! the same face, at the same size and variation, with the rest of the line as
//! context, and the two are compared glyph by glyph: IDs, positions and
//! advances, and clusters.

use std::fmt::Write as _;
use std::ops::Range;

use s5_rust_text::shape::{self, Params};

use crate::corpus::Sample;
use crate::fonts::{self, Source};
use crate::platform::coretext::{Font, Line, Run};

/// Positions and advances closer than this, in points, count as equal.
pub(crate) const TOLERANCE_PT: f64 = 0.01;

/// How a run compares. The first two render identically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The same glyphs, positions and advances.
    Same,
    /// The same glyphs at the same positions, with the run's total advance the
    /// same, but an adjustment (kerning, a mark's attachment) attributed to a
    /// different glyph's advance or offset.
    Split,
    /// The same glyphs at the same positions; the run's total advance differs.
    Advance,
    /// The same glyphs; some position differs.
    Positions,
    /// Different glyphs.
    Glyphs,
    /// HarfRust couldn't shape the run.
    Error,
}

impl Verdict {
    pub(crate) const ALL: [Self; 6] = [
        Self::Same,
        Self::Split,
        Self::Advance,
        Self::Positions,
        Self::Glyphs,
        Self::Error,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Same => "same",
            Self::Split => "split",
            Self::Advance => "advance",
            Self::Positions => "positions",
            Self::Glyphs => "glyphs",
            Self::Error => "error",
        }
    }
}

/// Who applies AAT tracking (`trak`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tracking {
    /// The design (TX10): HarfRust never sees `trak`; Axiom adds the normal
    /// track after shaping.
    Axiom,
    /// HarfRust applies `trak` itself, as it does by default.
    HarfRust,
}

/// One CoreText run against HarfRust.
#[derive(Clone, Debug)]
pub(crate) struct RunComparison {
    pub(crate) font: String,
    pub(crate) source: String,
    pub(crate) rtl: bool,
    pub(crate) range: Range<usize>,
    pub(crate) ct_glyphs: usize,
    pub(crate) hr_glyphs: usize,
    pub(crate) verdict: Verdict,
    pub(crate) max_advance_diff: f64,
    pub(crate) max_position_diff: f64,
    /// Glyphs whose CoreText string index differs from HarfRust's cluster.
    pub(crate) cluster_diffs: usize,
    pub(crate) detail: String,
    /// HarfRust's glyphs and their origins in points from the line's origin,
    /// y up, for drawing the pair.
    pub(crate) hr_glyph_ids: Vec<u16>,
    pub(crate) hr_positions: Vec<(f64, f64)>,
    /// One row per glyph pair, for runs that differ: IDs, positions, advances
    /// and clusters side by side.
    pub(crate) rows: Vec<String>,
    /// The variation CoreText applied, as tag=value pairs.
    pub(crate) variation: String,
    /// FNV-1a of CoreText's glyph IDs, to compare runs between settings: a
    /// font CoreText can't load in the sandbox shapes differently there.
    pub(crate) ct_hash: u64,
    /// Bytes of font data the Rust side shaped from.
    pub(crate) font_bytes: usize,
}

/// The font CoreText lays out `sample` from.
pub(crate) fn base_font(sample: &Sample, size: f64) -> Font {
    if sample.base == "system" {
        Font::system(size, Some(sample.language)).unwrap_or_else(|| Font::named("Helvetica", size))
    } else {
        Font::named(sample.base, size)
    }
}

/// Lay out `sample` with CoreText and compare each run with HarfRust.
///
/// CoreText sometimes splits one word of one font into several runs after
/// shaping it (vocalized Arabic in Geeza Pro, for example). Axiom itemizes runs
/// by font before shaping, and HarfRust's AAT path sees no context beyond its
/// buffer, so adjacent runs of the same font and direction, with contiguous
/// text, are merged first and shaped as one.
pub(crate) fn compare_line(
    sample: &Sample,
    size: f64,
    tracking: Tracking,
) -> (Line, Vec<Run>, Vec<RunComparison>) {
    let line = Line::new(sample.text, &base_font(sample, size), Some(sample.language));
    let runs = merge_runs(line.runs());
    let comparisons = runs
        .iter()
        .map(|run| compare_run(sample, run, tracking))
        .collect();
    (line, runs, comparisons)
}

/// Merge visually adjacent runs of one font and direction whose text ranges
/// meet (in logical order, which for right-to-left runs is reversed).
fn merge_runs(runs: Vec<Run>) -> Vec<Run> {
    let mut merged: Vec<Run> = Vec::with_capacity(runs.len());
    for run in runs {
        if let Some(last) = merged.last_mut()
            && last.rtl == run.rtl
            && last.font.postscript_name() == run.font.postscript_name()
            && (if run.rtl {
                run.range.end == last.range.start
            } else {
                last.range.end == run.range.start
            })
        {
            last.range = last.range.start.min(run.range.start)..last.range.end.max(run.range.end);
            last.glyphs.extend(run.glyphs);
            last.string_indices.extend(run.string_indices);
            last.positions.extend(run.positions);
            last.advances.extend(run.advances);
            continue;
        }
        merged.push(run);
    }
    merged
}

#[expect(clippy::too_many_lines, reason = "one comparison, step by step")]
fn compare_run(sample: &Sample, run: &Run, tracking: Tracking) -> RunComparison {
    let mut result = RunComparison {
        font: run.font.postscript_name(),
        source: String::new(),
        rtl: run.rtl,
        range: run.range.clone(),
        ct_glyphs: run.glyphs.len(),
        hr_glyphs: 0,
        verdict: Verdict::Error,
        max_advance_diff: 0.0,
        max_position_diff: 0.0,
        cluster_diffs: 0,
        detail: String::new(),
        hr_glyph_ids: Vec::new(),
        hr_positions: Vec::new(),
        rows: Vec::new(),
        variation: String::new(),
        ct_hash: run.glyphs.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &g| {
            (h ^ u64::from(g)).wrapping_mul(0x0100_0000_01b3)
        }),
        font_bytes: 0,
    };
    let data = match fonts::font_data(&run.font) {
        Ok(data) => data,
        Err(e) => {
            "unavailable".clone_into(&mut result.source);
            result.detail = e;
            return result;
        }
    };
    result.source = match &data.source {
        Source::File(path) => format!("file:{}", path.display()),
        Source::Tables => "tables".to_owned(),
    };
    result.font_bytes = data.bytes().len();
    let axes = run.font.variation();
    for (tag, value) in &axes {
        let _ = write!(
            result.variation,
            "{}={value} ",
            String::from_utf8_lossy(tag)
        );
    }
    #[expect(clippy::cast_possible_truncation, reason = "axis values are small")]
    let variations: Vec<([u8; 4], f32)> = axes.iter().map(|&(t, v)| (t, v as f32)).collect();
    let hr_font = match tracking {
        Tracking::Axiom => {
            shape::font_hiding(data.blob.clone(), data.index, &variations, &[*b"trak"])
        }
        Tracking::HarfRust => shape::font(data.blob.clone(), data.index, &variations),
    };
    let Some(hr_font) = hr_font else {
        "HarfRust could not open the font".clone_into(&mut result.detail);
        return result;
    };
    let size = run.font.size();
    #[expect(clippy::cast_possible_truncation, reason = "font sizes are small")]
    let size_f32 = size as f32;
    let params = Params {
        rtl: Some(run.rtl),
        language: Some(sample.language),
        point_size: (tracking == Tracking::HarfRust).then_some(size_f32),
    };
    let mut glyphs = match shape::shape(&hr_font, sample.text, run.range.clone(), &params) {
        Ok(glyphs) => glyphs,
        Err(e) => {
            result.detail = format!("{e:?}");
            return result;
        }
    };
    if tracking == Tracking::Axiom {
        let units = shape::normal_tracking(data.bytes(), data.index, size_f32);
        shape::apply_tracking(&mut glyphs, units);
    }
    result.hr_glyphs = glyphs.len();
    let scale = size / f64::from(run.font.units_per_em());

    // HarfRust's positions, anchored at CoreText's first glyph so that the
    // line's earlier runs don't matter.
    let first_offset = glyphs.first().map_or((0.0, 0.0), |g| {
        (f64::from(g.x_offset) * scale, f64::from(g.y_offset) * scale)
    });
    let anchor = run
        .positions
        .first()
        .map_or((0.0, 0.0), |p| (p.0 - first_offset.0, 0.0));
    let (mut pen_x, mut pen_y) = anchor;
    for g in &glyphs {
        result.hr_positions.push((
            pen_x + f64::from(g.x_offset) * scale,
            pen_y + f64::from(g.y_offset) * scale,
        ));
        result.hr_glyph_ids.push(u16::try_from(g.id).unwrap_or(0));
        pen_x += f64::from(g.x_advance) * scale;
        pen_y += f64::from(g.y_advance) * scale;
    }

    let same_ids = run.glyphs.len() == glyphs.len()
        && run
            .glyphs
            .iter()
            .zip(&glyphs)
            .all(|(&ct, hr)| u32::from(ct) == hr.id);
    let names = data
        .font_ref()
        .map(|f| skrifa::MetadataProvider::glyph_names(&f));
    let name = |id: u32| {
        names
            .as_ref()
            .and_then(|n| n.get(skrifa::GlyphId::new(id)))
            .map_or(id.to_string(), |n| format!("{id}:{n}"))
    };
    if !same_ids {
        result.verdict = Verdict::Glyphs;
        result.detail = first_difference(&run.glyphs, &glyphs);
        for i in 0..run.glyphs.len().max(glyphs.len()) {
            let ct = run
                .glyphs
                .get(i)
                .map_or("-".to_owned(), |&g| name(u32::from(g)));
            let hr = glyphs.get(i).map_or("-".to_owned(), |g| name(g.id));
            let ct_index = run
                .string_indices
                .get(i)
                .map_or("-".to_owned(), ToString::to_string);
            let hr_index = glyphs
                .get(i)
                .map_or("-".to_owned(), |g| g.cluster.to_string());
            result.rows.push(format!(
                "{i}\tct {ct} hr {hr}\tindex ct {ct_index} hr {hr_index}"
            ));
        }
        return result;
    }
    let (mut ct_total, mut hr_total) = (0.0, 0.0);
    for (i, g) in glyphs.iter().enumerate() {
        let hr = result.hr_positions[i];
        let ct = run.positions[i];
        let position_diff = (hr.0 - ct.0).abs().max((hr.1 - ct.1).abs());
        result.max_position_diff = result.max_position_diff.max(position_diff);
        let hr_advance = f64::from(g.x_advance) * scale;
        result.max_advance_diff = result
            .max_advance_diff
            .max((hr_advance - run.advances[i].0).abs());
        ct_total += run.advances[i].0;
        hr_total += hr_advance;
        if run.string_indices[i] != g.cluster as usize {
            result.cluster_diffs += 1;
        }
    }
    result.verdict = if result.max_position_diff > TOLERANCE_PT {
        Verdict::Positions
    } else if (ct_total - hr_total).abs() > TOLERANCE_PT {
        Verdict::Advance
    } else if result.max_advance_diff > TOLERANCE_PT {
        Verdict::Split
    } else {
        Verdict::Same
    };
    if result.verdict != Verdict::Same {
        for (i, g) in glyphs.iter().enumerate() {
            result.rows.push(format!(
                "{i}\tct {} hr {}\tpos ct ({:.3}, {:.3}) hr ({:.3}, {:.3})\tadv ct {:.3} hr {:.3}\tindex ct {} hr {}",
                name(u32::from(run.glyphs[i])),
                name(g.id),
                run.positions[i].0,
                run.positions[i].1,
                result.hr_positions[i].0,
                result.hr_positions[i].1,
                run.advances[i].0,
                f64::from(g.x_advance) * scale,
                run.string_indices[i],
                g.cluster
            ));
        }
    }
    result
}

/// The glyph IDs around the first place the two sequences differ.
fn first_difference(ct: &[u16], hr: &[shape::Glyph]) -> String {
    let at = ct
        .iter()
        .zip(hr)
        .position(|(&c, h)| u32::from(c) != h.id)
        .unwrap_or_else(|| ct.len().min(hr.len()));
    let from = at.saturating_sub(3);
    let mut out = format!("first difference at glyph {at}: CoreText");
    for id in ct.iter().skip(from).take(7) {
        let _ = write!(out, " {id}");
    }
    out.push_str(" / HarfRust");
    for g in hr.iter().skip(from).take(7) {
        let _ = write!(out, " {}", g.id);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus;

    /// Latin in Helvetica: HarfRust and CoreText place the same glyphs at the
    /// same positions (kerning may be attributed differently: `Split`).
    #[test]
    fn latin_runs_render_alike() {
        let sample = corpus::samples()
            .into_iter()
            .find(|s| s.id == "latin")
            .expect("the corpus has a Latin line");
        let (_, _, comparisons) = compare_line(&sample, 16.0, Tracking::Axiom);
        assert!(!comparisons.is_empty());
        for c in &comparisons {
            assert!(
                matches!(c.verdict, Verdict::Same | Verdict::Split),
                "{} {}: {}",
                c.font,
                c.verdict.name(),
                c.detail
            );
        }
    }
}

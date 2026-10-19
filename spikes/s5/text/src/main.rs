//! # Spike S5: CoreText against the Rust text path, on this Mac's system fonts
//!
//! Questions (Step 5 tracker, §10.1):
//! - **S5-Q1, shaping:** do HarfRust's glyphs, clusters, advances and offsets
//!   match CoreText's on a multi-script corpus, each script in the system font
//!   CoreText's fallback chooses?
//! - **S5-Q2, rasterization:** how do zeno's and `vello_cpu`'s masks compare
//!   with CoreText's at 12–32 px, at 1× and 2×, with and without font
//!   smoothing, and what does a glyph cost each way?
//! - **S5-Q4, footprint and speed:** what does each configuration of the stack
//!   add to a sandboxed helper's footprint, and how fast is shaping?
//!
//! Usage: `s5-text MODE [--profile none|text|text-s5|FILE] ...`
//! - `shape [--detail on] [--tracking axiom|harfrust] [--sheets DIR]`: S5-Q1.
//!   One line per CoreText run, with a hash of CoreText's glyphs for
//!   comparing runs under different profiles. `--detail on` adds each run's
//!   glyphs side by side. `--tracking harfrust` lets HarfRust apply `trak`
//!   itself, where Axiom applies the normal track after shaping (TX10). With
//!   `--sheets`, a PNG per corpus line: CoreText's rendering above HarfRust's
//!   glyphs drawn at HarfRust's positions, and their overlay. Sheets are
//!   written to disk, so only unsandboxed.
//! - `raster [--sheets DIR] [--repeat N]`: S5-Q2: masks compared, colour
//!   glyphs compared, and the cost of a glyph each way.
//! - `raster-timing [--repeat N]`: only the cost of a glyph each way.
//! - `ready --config CONFIG`: a sandboxed helper for S5-Q4: it reads its
//!   compiled profile from descriptor 3, applies it, runs the configuration's
//!   work, then says ready and waits (S4's protocol).
//! - `work --config CONFIG [--detail on]`: the same work in this process, for
//!   checking a configuration by hand.
//! - `footprint [--runs N]`: S5-Q4's broker: launches `ready` helpers for
//!   each configuration, sandboxed with S5's text profile (or `--profile`)
//!   compiled here, and reads their footprints.
//! - `speed [--repeat N]`: S5-Q4's shaping speed: CoreText's line layout,
//!   CoreText on resolved runs, and HarfRust on the same runs.
//!
//! `--profile text` applies S4's text profile (system fonts and ICU's data
//! readable) before any work. `--profile text-s5` applies S5's
//! (`profiles/text.sb`), which adds metadata access to the executable's
//! directory (without it CoreText crashes drawing colour emoji) and read
//! access to PingFang's directory (without it CoreText substitutes a stub
//! for Chinese). `--profile FILE` applies an SBPL file. Every profile gets the
//! `EXECUTABLE_DIR` parameter.
#![deny(unsafe_code)]

mod corpus;
mod fonts;
mod platform;
mod rasterize;
mod shaping;
mod sheet;
mod workload;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use s4_sandbox::platform::sandbox;
use s4_sandbox::profile::{Method, RENDEZVOUS_PARAM, TEXT};

use crate::platform::coretext::{Canvas, Smoothing};
use crate::sheet::Sheet;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = args.first().cloned() else {
        return usage();
    };
    let options = match Options::parse(&args[1..]) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("s5-text: {message}");
            return ExitCode::from(2);
        }
    };
    let result = match mode.as_str() {
        "shape" => options.enter_profile().and_then(|()| shape(&options)),
        "raster" => options
            .enter_profile()
            .and_then(|()| rasterize::run(&options)),
        "raster-timing" => options
            .enter_profile()
            .map(|()| rasterize::run_timing(&options)),
        "ready" => workload::ready(&options),
        "work" => options
            .enter_profile()
            .and_then(|()| workload::work_mode(&options)),
        "footprint" => workload::footprint(&options),
        "speed" => options
            .enter_profile()
            .and_then(|()| workload::speed(&options)),
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("s5-text: {message}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: s5-text shape|raster|raster-timing|ready|work|footprint|speed [--profile none|text|text-s5|FILE] [--sheets DIR] [--config NAME] [--runs N] [--repeat N] [--detail on] [--tracking axiom|harfrust]"
    );
    ExitCode::from(2)
}

/// Command-line options shared by the modes.
#[derive(Debug, Default)]
pub(crate) struct Options {
    /// `None`, S4's text profile, S5's, or a file.
    pub(crate) profile: Option<ProfileChoice>,
    pub(crate) sheets: Option<PathBuf>,
    pub(crate) config: Option<String>,
    pub(crate) runs: usize,
    pub(crate) repeat: usize,
    /// Print each differing run's glyphs side by side.
    pub(crate) detail: bool,
    /// Who applies AAT tracking in S5-Q1.
    pub(crate) tracking: Option<shaping::Tracking>,
}

#[derive(Debug, Clone)]
pub(crate) enum ProfileChoice {
    /// S4's text profile, unchanged.
    Text,
    /// S5's text profile: S4's, plus metadata of the executable's directory
    /// and PingFang's directory.
    TextS5,
    File(PathBuf),
}

/// S5's text profile.
pub(crate) const TEXT_S5: &str = include_str!("../profiles/text.sb");

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            runs: 3,
            repeat: 5,
            ..Self::default()
        };
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
            let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
            let number = || value.parse::<usize>().map_err(|e| format!("{flag}: {e}"));
            match flag.as_str() {
                "--profile" => {
                    options.profile = match value.as_str() {
                        "none" => None,
                        "text" => Some(ProfileChoice::Text),
                        "text-s5" => Some(ProfileChoice::TextS5),
                        file => Some(ProfileChoice::File(PathBuf::from(file))),
                    }
                }
                "--sheets" => options.sheets = Some(PathBuf::from(value)),
                "--config" => options.config = Some(value.clone()),
                "--runs" => options.runs = number()?,
                "--repeat" => options.repeat = number()?,
                "--detail" => options.detail = value == "on",
                "--tracking" => {
                    options.tracking = Some(match value.as_str() {
                        "axiom" => shaping::Tracking::Axiom,
                        "harfrust" => shaping::Tracking::HarfRust,
                        other => return Err(format!("unknown tracking {other:?}")),
                    });
                }
                other => return Err(format!("unknown flag {other}")),
            }
        }
        Ok(options)
    }

    /// The profile's SBPL source, if any.
    pub(crate) fn profile_source(&self) -> Result<Option<String>, String> {
        match &self.profile {
            None => Ok(None),
            Some(ProfileChoice::Text) => Ok(Some(TEXT.to_owned())),
            Some(ProfileChoice::TextS5) => Ok(Some(TEXT_S5.to_owned())),
            Some(ProfileChoice::File(path)) => std::fs::read_to_string(path)
                .map(Some)
                .map_err(|e| format!("{}: {e}", path.display())),
        }
    }

    /// Apply the profile, if any, before anything else happens.
    fn enter_profile(&self) -> Result<(), String> {
        if let Some(source) = self.profile_source()? {
            let dir = executable_dir()?;
            sandbox::enter(
                &source,
                &[
                    (RENDEZVOUS_PARAM, "org.axiom.s5.unused"),
                    ("EXECUTABLE_DIR", &dir),
                ],
                Method::InitWithParameters,
            )
            .map_err(|e| format!("the profile did not apply: {e}"))?;
        }
        println!("# sandboxed={}", sandbox::is_sandboxed());
        Ok(())
    }
}

/// The directory this executable runs from, for the `EXECUTABLE_DIR`
/// profile parameter.
pub(crate) fn executable_dir() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .and_then(Path::to_str)
        .map(str::to_owned)
        .ok_or_else(|| "the executable's directory isn't UTF-8".to_owned())
}

/// The sizes S5-Q1 shapes at, in points.
const SHAPE_SIZES: [f64; 2] = [16.0, 32.0];
/// The size of the visual-check sheets.
const SHEET_SIZE: f64 = 32.0;

fn shape(options: &Options) -> Result<(), String> {
    println!(
        "# shape\tline\tsize\trun\tutf16-range\tfont\tsource\tdir\tct-glyphs\thr-glyphs\tverdict\tmax-advance-diff-pt\tmax-position-diff-pt\tcluster-diffs\tdetail\tct-hash\tfont-bytes"
    );
    let tracking = options.tracking.unwrap_or(shaping::Tracking::Axiom);
    println!("# tracking: {tracking:?}");
    let mut totals = [0_usize; shaping::Verdict::ALL.len()];
    for sample in corpus::samples() {
        for size in SHAPE_SIZES {
            let (line, runs, comparisons) = shaping::compare_line(&sample, size, tracking);
            for (i, c) in comparisons.iter().enumerate() {
                totals[c.verdict as usize] += 1;
                println!(
                    "shape\t{}\t{size}\t{i}\t{}..{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{}\t{:016x}\t{}",
                    sample.id,
                    c.range.start,
                    c.range.end,
                    c.font,
                    c.source,
                    if c.rtl { "rtl" } else { "ltr" },
                    c.ct_glyphs,
                    c.hr_glyphs,
                    c.verdict.name(),
                    c.max_advance_diff,
                    c.max_position_diff,
                    c.cluster_diffs,
                    c.detail,
                    c.ct_hash,
                    c.font_bytes
                );
                if options.detail {
                    println!("#   variation: {}", c.variation);
                    for row in &c.rows {
                        println!("#   {row}");
                    }
                }
            }
            if let Some(dir) = &options.sheets
                && size == SHEET_SIZE
            {
                write_pair(dir, sample.id, &line, &runs, &comparisons)?;
            }
        }
    }
    let summary: Vec<String> = shaping::Verdict::ALL
        .iter()
        .map(|v| format!("{} {}", totals[*v as usize], v.name()))
        .collect();
    println!("# summary: {}", summary.join(", "));
    Ok(())
}

/// CoreText's rendering of the line above HarfRust's glyphs drawn, with
/// CoreText, at HarfRust's positions: any difference is shaping, not
/// rasterization.
fn write_pair(
    dir: &Path,
    id: &str,
    line: &platform::coretext::Line,
    runs: &[platform::coretext::Run],
    comparisons: &[shaping::RunComparison],
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (width, ascent, descent) = line.bounds();
    let margin = 16.0;
    let row_height = (ascent + descent + margin).ceil();
    let canvas_width = to_px(width + 2.0 * margin);
    let row = to_px(row_height);
    let origin = (margin, margin / 2.0 + ascent);
    let mut ct = Canvas::new(canvas_width, row, false, Smoothing::Off);
    line.draw(&mut ct, origin);
    let mut hr = Canvas::new(canvas_width, row, false, Smoothing::Off);
    for (run, comparison) in runs.iter().zip(comparisons) {
        hr.draw_glyphs(
            &run.font,
            &comparison.hr_glyph_ids,
            &comparison.hr_positions,
            origin,
        );
    }
    // CoreText, HarfRust, then both overlaid at twice the size: ink only
    // CoreText's red, only HarfRust's blue, shared black.
    let mut sheet = Sheet::new(canvas_width * 2, row * 4 + 8);
    sheet.mask(0, 0, &ct.pixels, canvas_width, 1, [0, 0, 0]);
    sheet.mask(0, row + 4, &hr.pixels, canvas_width, 1, [0, 0, 160]);
    sheet.overlay(0, row * 2 + 8, &ct.pixels, &hr.pixels, canvas_width, 2);
    sheet
        .write(&dir.join(format!("shaping-{id}.png")))
        .map_err(|e| e.to_string())
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "canvas sizes are small and positive"
)]
pub(crate) fn to_px(v: f64) -> usize {
    v.max(1.0).ceil() as usize
}

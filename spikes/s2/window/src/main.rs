//! # Spike S2 (CP8): AppKit + Metal + CoreText, without Xcode
//!
//! Opens an AppKit window whose content view hosts a `CAMetalLayer`. CoreText
//! lays out one line of text and CoreGraphics rasterizes it into a coverage
//! mask. A Metal pipeline, compiled from MSL source at run time, draws the
//! mask over a cleared frame.
//!
//! The spike answers two questions:
//! - **S2-Q5:** does this work through objc2 with the Command Line Tools
//!   alone, with `unsafe` confined to `src/platform/`?
//! - **S2-Q6:** what does it cost to build and to run?
//!
//! Usage: `s2-window [--smoke] [--seconds N]`
//! - With no flags, the window stays open until it is closed, and redraws
//!   when its size changes. Live resizing stretches the last frame until the
//!   mouse is released (a known limit of the hand-pumped event loop).
//! - `--smoke` draws a few frames, reads the last one back, checks it against
//!   the coverage mask, prints timings, and exits: 0 if the frame is right.
//! - `--seconds N` closes after N seconds, for idle CPU and memory
//!   measurements.
#![deny(unsafe_code)]

mod platform;

use std::process::ExitCode;
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;

use crate::platform::app::{App, Wait};
use crate::platform::gpu::{CLEAR, Gpu, Readback, TEXT_COLOR};
use crate::platform::text::{Coverage, rasterize_line};

const TITLE: &str = "Arbor S2: CoreText on Metal";
const TEXT: &str = "Arbor S2: CoreText → CGBitmapContext → Metal, without Xcode";
const FONT: &str = "Helvetica";
const FONT_POINTS: f64 = 26.0;
const WINDOW_POINTS: (f64, f64) = (960.0, 200.0);
/// Frames the smoke test draws. It checks the last one.
const SMOKE_FRAMES: u32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Options {
    smoke: bool,
    seconds: Option<f64>,
}

fn main() -> ExitCode {
    let start = Instant::now();
    let options = match parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("s2-window: {message}\nusage: s2-window [--smoke] [--seconds N]");
            return ExitCode::from(2);
        }
    };
    match run(options, start) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("s2-window: {message}");
            ExitCode::FAILURE
        }
    }
}

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => options.smoke = true,
            "--seconds" => {
                let value = args.next().ok_or("--seconds needs a value")?;
                let seconds: f64 = value
                    .parse()
                    .map_err(|e| format!("bad --seconds {value:?}: {e}"))?;
                if !(seconds.is_finite() && seconds > 0.0) {
                    return Err(format!("--seconds must be positive, not {value}"));
                }
                options.seconds = Some(seconds);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(options)
}

fn run(options: Options, start: Instant) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("must run on the main thread")?;
    let mut gpu = Gpu::new()?;
    let compiled = start.elapsed();
    let app = App::new(mtm, TITLE, WINDOW_POINTS.0, WINDOW_POINTS.1, gpu.device())?;
    let shown = start.elapsed();
    let coverage = rasterize_line(TEXT, FONT, FONT_POINTS * app.scale())?;
    gpu.set_text(&coverage)?;
    let deadline = options.seconds.map(|s| start + Duration::from_secs_f64(s));

    let mut frames = 0_u32;
    let mut drawn = None;
    let mut checked_frame = None;
    loop {
        let wait = match deadline {
            _ if options.smoke || drawn.is_none() => Wait::No,
            Some(deadline) => Wait::For(
                deadline
                    .saturating_duration_since(Instant::now())
                    .as_secs_f64(),
            ),
            None => Wait::Forever,
        };
        if !app.pump(wait) {
            break; // the window was closed
        }
        let size = app.sync_drawable_size();
        if drawn != Some(size) || (options.smoke && frames < SMOKE_FRAMES) {
            let origin = centred(size, &coverage);
            let readback = options.smoke && frames + 1 == SMOKE_FRAMES;
            let frame = gpu.render(app.layer(), [origin.0, origin.1].map(to_f32), readback)?;
            frames += 1;
            drawn = Some(size);
            if frames == 1 {
                println!(
                    "s2-window: MSL compiled at {:.1} ms, window shown at {:.1} ms, \
                     first frame presented at {:.1} ms after start; drawable {}×{} px, \
                     text {}×{} px, scale {}",
                    millis(compiled),
                    millis(shown),
                    millis(start.elapsed()),
                    size.0,
                    size.1,
                    coverage.width,
                    coverage.height,
                    app.scale(),
                );
            }
            if let Some(frame) = frame {
                checked_frame = Some((frame, origin));
            }
        }
        if options.smoke && frames >= SMOKE_FRAMES {
            break;
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break;
        }
    }

    if options.smoke {
        let (frame, origin) =
            checked_frame.ok_or("the window closed before the smoke test finished")?;
        println!("s2-window: smoke OK: {}", check(&frame, origin, &coverage)?);
    }
    Ok(())
}

/// The text's top-left corner, in whole pixels, so that texels map 1:1 to
/// pixels.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "drawable sizes are small, finite and non-negative"
)]
fn centred((width, height): (f64, f64), text: &Coverage) -> (usize, usize) {
    let left = ((width as usize).saturating_sub(text.width)) / 2;
    let top = ((height as usize).saturating_sub(text.height)) / 2;
    (left, top)
}

/// The smoke test's oracle. The frame must show the background everywhere
/// outside the text's box. Inside it, it must show exactly the ink pixels
/// that the coverage mask predicts.
///
/// A pixel counts as ink when its red channel is more than `THRESHOLD` above
/// the background's. Blending text of colour `t` and coverage `c` over the
/// background `b` gives `a·t + (1 − a)·b` with `a = (c / 255)·tₐ`. The
/// prediction applies that formula to every mask pixel. GPU rounding can move
/// a pixel that sits exactly on the threshold, so a small tolerance is allowed.
fn check(frame: &Readback, origin: (usize, usize), text: &Coverage) -> Result<String, String> {
    const THRESHOLD: u8 = 96;
    let background = [
        unorm8(CLEAR.blue),
        unorm8(CLEAR.green),
        unorm8(CLEAR.red),
        255,
    ];
    let pixel = |x: usize, y: usize| {
        let at = (y * frame.width + x) * 4;
        [
            frame.bgra[at],
            frame.bgra[at + 1],
            frame.bgra[at + 2],
            frame.bgra[at + 3],
        ]
    };

    let corner = pixel(0, 0);
    if corner
        .iter()
        .zip(background)
        .any(|(&got, want)| got.abs_diff(want) > 1)
    {
        return Err(format!(
            "background is {corner:?} (BGRA), expected {background:?}"
        ));
    }

    let inside_box = |x: usize, y: usize| {
        (origin.0..origin.0 + text.width).contains(&x)
            && (origin.1..origin.1 + text.height).contains(&y)
    };
    let (mut inside, mut outside) = (0_usize, 0_usize);
    for y in 0..frame.height {
        for x in 0..frame.width {
            if pixel(x, y)[2].saturating_sub(background[2]) > THRESHOLD {
                if inside_box(x, y) {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
        }
    }
    let (text_red, text_alpha) = (f64::from(TEXT_COLOR[0]), f64::from(TEXT_COLOR[3]));
    let background_red = f64::from(background[2]) / 255.0;
    let predicted = text
        .pixels
        .iter()
        .filter(|&&c| {
            let alpha = f64::from(c) / 255.0 * text_alpha;
            let red = alpha * text_red + (1.0 - alpha) * background_red;
            (red - background_red) * 255.0 > f64::from(THRESHOLD) + 0.5
        })
        .count();
    let tolerance = (predicted / 200).max(4);
    if outside > 0 || predicted == 0 || inside.abs_diff(predicted) > tolerance {
        return Err(format!(
            "{inside} ink pixels inside the text box (predicted {predicted} ± {tolerance}), \
             {outside} outside it"
        ));
    }
    Ok(format!(
        "{inside} ink pixels where the mask predicts {predicted}, none outside the \
         {}×{} text box; background {background:?} (BGRA); frame {}×{} px",
        text.width, text.height, frame.width, frame.height
    ))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the input is a colour component in [0, 1]"
)]
fn unorm8(component: f64) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[expect(
    clippy::cast_precision_loss,
    reason = "pixel coordinates are far below 2^24, so f32 holds them exactly"
)]
fn to_f32(pixels: usize) -> f32 {
    pixels as f32
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Options, String> {
        parse(list.iter().map(ToString::to_string))
    }

    #[test]
    fn options_parse() {
        assert_eq!(args(&[]), Ok(Options::default()));
        assert_eq!(
            args(&["--smoke", "--seconds", "2.5"]),
            Ok(Options {
                smoke: true,
                seconds: Some(2.5)
            })
        );
        assert!(args(&["--seconds"]).is_err());
        assert!(args(&["--seconds", "-1"]).is_err());
        assert!(args(&["--seconds", "NaN"]).is_err());
        assert!(args(&["--frobnicate"]).is_err());
    }

    #[test]
    fn text_is_centred_in_whole_pixels() {
        let text = Coverage {
            width: 10,
            height: 4,
            pixels: vec![0; 40],
        };
        assert_eq!(centred((31.0, 9.0), &text), (10, 2));
        assert_eq!(
            centred((5.0, 2.0), &text),
            (0, 0),
            "a larger text box clamps"
        );
    }

    /// A frame that shows `text` (white, opaque) at `origin` over the
    /// background, blended as the GPU does.
    fn render_on_cpu(
        width: usize,
        height: usize,
        origin: (usize, usize),
        text: &Coverage,
    ) -> Readback {
        let background = [
            unorm8(CLEAR.blue),
            unorm8(CLEAR.green),
            unorm8(CLEAR.red),
            255,
        ];
        let mut bgra = background.repeat(width * height);
        for y in 0..text.height {
            for x in 0..text.width {
                let alpha = f64::from(text.pixels[y * text.width + x]) / 255.0;
                let at = ((origin.1 + y) * width + origin.0 + x) * 4;
                for channel in 0..3 {
                    let below = f64::from(bgra[at + channel]) / 255.0;
                    bgra[at + channel] = unorm8(alpha + (1.0 - alpha) * below);
                }
            }
        }
        Readback {
            width,
            height,
            bgra,
        }
    }

    fn sample_text() -> Coverage {
        // A 6×3 mask with full, partial and no coverage.
        Coverage {
            width: 6,
            height: 3,
            pixels: vec![
                0, 255, 200, 104, 105, 0, 255, 255, 0, 0, 30, 0, 0, 0, 0, 255, 0, 0,
            ],
        }
    }

    #[test]
    fn the_oracle_accepts_a_correct_frame() {
        let text = sample_text();
        let frame = render_on_cpu(20, 8, (7, 2), &text);
        let report = check(&frame, (7, 2), &text).expect("a correct frame passes");
        // Ink is coverage ≥ 105: 255, 200, 105, 255, 255, 255.
        assert!(
            report.starts_with("6 ink pixels where the mask predicts 6"),
            "{report}"
        );
    }

    #[test]
    fn the_oracle_rejects_stray_ink_and_missing_text() {
        let text = sample_text();
        let mut frame = render_on_cpu(20, 8, (7, 2), &text);
        // One white pixel outside the text box.
        frame.bgra[..3].fill(255);
        assert!(check(&frame, (7, 2), &text).is_err(), "ink outside the box");

        let blank = render_on_cpu(
            20,
            8,
            (7, 2),
            &Coverage {
                pixels: vec![0; 18],
                ..text.clone()
            },
        );
        assert!(
            check(&blank, (7, 2), &text).is_err(),
            "no ink where the mask has some"
        );

        let elsewhere = render_on_cpu(20, 8, (1, 4), &text);
        assert!(
            check(&elsewhere, (7, 2), &text).is_err(),
            "text drawn in the wrong place"
        );
    }
}

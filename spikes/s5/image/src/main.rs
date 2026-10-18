//! # Spike S5, S5-Q5: images in the renderer's sandbox
//!
//! Under S4's renderer profile, does ImageIO decode AVIF, and which rules does
//! it need (`docs/architecture/images-and-media.md`, IM3)? How do the safe
//! Rust decoders (`png`, `jpeg-decoder` in its `platform_independent` mode,
//! `gif`, `image-webp`) compare with ImageIO in time per megapixel and peak
//! memory (IM2)?
//!
//! Usage:
//! - `s5-image decode --decoder rust|imageio|imageio-lazy
//!   [--profile PROFILE] [--repeat N] [--warmup FILE] FILE...`:
//!   read every file, apply the profile, decode the warm-up file once if one
//!   is given (unreported, so first-use costs fall outside the measurement),
//!   then decode each file `N` times and print one line: format, size, the
//!   median total time and its two parts, time per megapixel, the peak memory
//!   the first decode added (its output included), and a hash of the pixels. `imageio` decodes when the
//!   image is created (`kCGImageSourceShouldCacheImmediately`), then draws it
//!   into Axiom's buffer: its parts are the decode and the conversion.
//!   `imageio-lazy` caches nothing, so drawing decodes, as an engine that
//!   owns its pixel buffers would use it. `PROFILE` is `none`, `renderer`
//!   (S4's), `renderer+imageio` (S4's plus R1, `profiles/imageio.sb`),
//!   `renderer+avif` (plus R2–R6, `profiles/avif.sb`), or a file, which may
//!   use the `EXECUTABLE_DIR` parameter.
//! - `compare FILE...`: decode each with both, and report how far apart their
//!   pixels are.
//! - `encode-webp IN.png OUT.webp`: a lossless WebP, with `image-webp`'s own
//!   encoder (the system can't write WebP; `sips --formats`).
//! - `pattern OUT.png WIDTH HEIGHT`: a deterministic high-entropy test image.
#![deny(unsafe_code)]

mod decoders;
mod platform;

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use platform::imageio::Cache;
use s4_sandbox::platform::{process, sandbox};
use s4_sandbox::profile::{Method, RENDERER, RENDEZVOUS_PARAM};

/// The rule ImageIO needs in S4's renderer profile (R1).
const IMAGEIO: &str = include_str!("../profiles/imageio.sb");
/// The rules it needs for AVIF besides (R2–R6).
const AVIF: &str = include_str!("../profiles/avif.sb");

/// A decoded image: sRGB, premultiplied RGBA, rows top to bottom.
#[derive(Clone, Debug)]
pub(crate) struct Decoded {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) rgba: Vec<u8>,
    pub(crate) format: String,
}

/// The decoders `decode` measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Engine {
    Rust,
    ImageIo,
    ImageIoLazy,
}

impl Engine {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "rust" => Ok(Self::Rust),
            "imageio" => Ok(Self::ImageIo),
            "imageio-lazy" => Ok(Self::ImageIoLazy),
            other => Err(format!("unknown decoder {other}")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::ImageIo => "imageio",
            Self::ImageIoLazy => "imageio-lazy",
        }
    }

    /// Decode `data` to premultiplied RGBA: the image, the decode time and
    /// the conversion time, where the conversion is timed apart (ImageIO
    /// decoding at creation). Otherwise the first time covers both.
    fn decode(self, data: &[u8]) -> Result<(Decoded, Duration, Duration), String> {
        match self {
            Self::Rust => decoders::decode(data).map(|(image, time)| (image, time, Duration::ZERO)),
            Self::ImageIo => platform::imageio::decode(data, Cache::Immediately),
            Self::ImageIoLazy => platform::imageio::decode(data, Cache::Never),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("decode") => decode(&args[1..]),
        Some("compare") => compare(&args[1..]),
        Some("encode-webp") => encode_webp(&args[1..]),
        Some("pattern") => pattern(&args[1..]),
        _ => Err("usage: s5-image decode|compare|encode-webp|pattern ...".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("s5-image: {message}");
            ExitCode::FAILURE
        }
    }
}

fn decode(args: &[String]) -> Result<(), String> {
    let (mut engine, mut profile, mut repeat) = (Engine::Rust, None::<String>, 5_usize);
    let (mut warmup, mut files) = (None::<String>, Vec::new());
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--decoder" => engine = Engine::parse(&value()?)?,
            "--profile" => profile = Some(value()?),
            "--repeat" => repeat = value()?.parse().map_err(|_| "--repeat needs a number")?,
            "--warmup" => warmup = Some(value()?),
            file => files.push(file.to_owned()),
        }
    }
    let read = |file: &String| std::fs::read(file).map_err(|e| format!("{file}: {e}"));
    let warmup = warmup.as_ref().map(read).transpose()?;
    let inputs = files
        .iter()
        .map(|file| read(file).map(|data| (file, data)))
        .collect::<Result<Vec<_>, _>>()?;
    enter_profile(profile.as_deref())?;
    println!("# sandboxed={}", sandbox::is_sandboxed());
    if let Some(data) = &warmup {
        engine
            .decode(data)
            .map_err(|e| format!("the warm-up decode failed: {e}"))?;
    }
    println!(
        "# decode\tfile\tdecoder\tformat\tsize\tmedian-us\tus-per-megapixel\tdecode-us\tconvert-us\tpeak-added-bytes\thash\tmean-rgba"
    );
    for (name, data) in &inputs {
        let before = peak_footprint();
        let (mut times, mut first, mut added) = (Vec::with_capacity(repeat), None, 0);
        for _ in 0..repeat.max(1) {
            match engine.decode(data) {
                Ok((image, decode, convert)) => {
                    times.push((decode, convert));
                    if first.is_none() {
                        // The first decode's peak, its output included. Each
                        // image is freed before the next decode.
                        added = peak_footprint().saturating_sub(before);
                        first = Some(Pixels::of(&image));
                    }
                }
                Err(e) => {
                    println!("error\t{name}\t{}\t{e}", engine.name());
                    break;
                }
            }
        }
        if let Some(pixels) = first {
            report(name, engine, &pixels, &times, added);
        }
    }
    Ok(())
}

/// Apply no profile, S4's renderer profile with or without S5's rules, or a
/// profile file, which may use the `EXECUTABLE_DIR` parameter.
fn enter_profile(profile: Option<&str>) -> Result<(), String> {
    let source = match profile {
        None | Some("none") => return Ok(()),
        Some("renderer") => RENDERER.to_owned(),
        Some("renderer+imageio") => format!("{RENDERER}\n{IMAGEIO}"),
        Some("renderer+avif") => format!("{RENDERER}\n{IMAGEIO}\n{AVIF}"),
        Some(file) => std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?,
    };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .and_then(Path::to_str)
        .unwrap_or("/")
        .to_owned();
    sandbox::enter(
        &source,
        &[
            (RENDEZVOUS_PARAM, "org.axiom.s5.unused"),
            ("EXECUTABLE_DIR", &dir),
        ],
        Method::InitWithParameters,
    )
    .map_err(|e| format!("the profile did not apply: {e}"))
}

/// What `decode` reports about an image's pixels, so that the pixels
/// themselves can be freed before the next decode.
struct Pixels {
    format: String,
    width: usize,
    height: usize,
    hash: u64,
    mean: String,
}

impl Pixels {
    fn of(decoded: &Decoded) -> Self {
        Self {
            format: decoded.format.clone(),
            width: decoded.width,
            height: decoded.height,
            hash: fnv(&decoded.rgba),
            mean: mean_rgba(decoded),
        }
    }
}

/// One `decode` line: the medians of the total and of each part, and a
/// summary of the pixels for comparing runs.
fn report(name: &str, engine: Engine, pixels: &Pixels, times: &[(Duration, Duration)], added: u64) {
    let total = median(times.iter().map(|&(decode, convert)| decode + convert));
    let megapixels = to_f64(pixels.width * pixels.height) / 1e6;
    println!(
        "decode\t{name}\t{}\t{}\t{}x{}\t{}\t{:.0}\t{}\t{}\t{added}\t{:016x}\t{}",
        engine.name(),
        pixels.format,
        pixels.width,
        pixels.height,
        total.as_micros(),
        total.as_secs_f64() * 1e6 / megapixels.max(1e-9),
        median(times.iter().map(|&(decode, _)| decode)).as_micros(),
        median(times.iter().map(|&(_, convert)| convert)).as_micros(),
        pixels.hash,
        pixels.mean
    );
}

fn median(times: impl Iterator<Item = Duration>) -> Duration {
    let mut times: Vec<Duration> = times.collect();
    times.sort();
    times.get(times.len() / 2).copied().unwrap_or_default()
}

/// The mean of each channel, as `r/g/b/a`.
fn mean_rgba(decoded: &Decoded) -> String {
    let mut sums = [0_u64; 4];
    for px in decoded.rgba.chunks_exact(4) {
        for (sum, &c) in sums.iter_mut().zip(px) {
            *sum += u64::from(c);
        }
    }
    let pixels = to_f64(decoded.width * decoded.height).max(1.0);
    #[expect(
        clippy::cast_precision_loss,
        reason = "sums of bytes stay far below 2^53"
    )]
    let means: Vec<String> = sums
        .iter()
        .map(|&sum| format!("{:.1}", sum as f64 / pixels))
        .collect();
    means.join("/")
}

/// The process's peak physical footprint so far.
fn peak_footprint() -> u64 {
    let pid = i32::try_from(std::process::id()).unwrap_or(0);
    process::footprint(pid).map_or(0, |f| f.lifetime_max)
}

fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// How far the Rust decoder's pixels are from ImageIO's.
fn compare(files: &[String]) -> Result<(), String> {
    println!("# compare\tfile\tsize\tmean-abs-diff\tmax-diff\tshare-over-8");
    for file in files {
        let data = std::fs::read(file).map_err(|e| e.to_string())?;
        let ours = decoders::decode(&data).map(|(d, _)| d);
        let theirs = platform::imageio::decode(&data, Cache::Immediately).map(|(d, _, _)| d);
        match (ours, theirs) {
            (Ok(a), Ok(b)) if a.width == b.width && a.height == b.height => {
                let (mut sum, mut max, mut over) = (0_u64, 0_u8, 0_usize);
                for (&x, &y) in a.rgba.iter().zip(&b.rgba) {
                    let d = x.abs_diff(y);
                    sum += u64::from(d);
                    max = max.max(d);
                    if d > 8 {
                        over += 1;
                    }
                }
                let n = to_f64(a.rgba.len()).max(1.0);
                println!(
                    "compare\t{file}\t{}x{}\t{:.3}\t{max}\t{:.4}",
                    a.width,
                    a.height,
                    to_f64(usize::try_from(sum).unwrap_or(usize::MAX)) / n,
                    to_f64(over) / n
                );
            }
            (Ok(a), Ok(b)) => println!(
                "compare\t{file}\tsizes differ: {}x{} and {}x{}",
                a.width, a.height, b.width, b.height
            ),
            (a, b) => println!(
                "compare\t{file}\tRust: {}; ImageIO: {}",
                a.err().unwrap_or_else(|| "ok".to_owned()),
                b.err().unwrap_or_else(|| "ok".to_owned())
            ),
        }
    }
    Ok(())
}

fn encode_webp(args: &[String]) -> Result<(), String> {
    let [input, output] = args else {
        return Err("encode-webp IN.png OUT.webp".to_owned());
    };
    let data = std::fs::read(input).map_err(|e| e.to_string())?;
    let (decoded, _) = decoders::decode(&data)?;
    // Lossless WebP of straight-alpha RGBA: the test images are opaque, so
    // premultiplied and straight alpha coincide.
    let file = BufWriter::new(File::create(output).map_err(|e| e.to_string())?);
    image_webp::WebPEncoder::new(file)
        .encode(
            &decoded.rgba,
            u32::try_from(decoded.width).map_err(|e| e.to_string())?,
            u32::try_from(decoded.height).map_err(|e| e.to_string())?,
            image_webp::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())
}

/// A deterministic image with gradients, hard edges and noise: the noise
/// gives JPEG and PNG close to their worst-case entropy.
fn pattern(args: &[String]) -> Result<(), String> {
    let [output, width_arg, height_arg] = args else {
        return Err("pattern OUT.png WIDTH HEIGHT".to_owned());
    };
    let width: u32 = width_arg.parse().map_err(|_| "WIDTH")?;
    let height: u32 = height_arg.parse().map_err(|_| "HEIGHT")?;
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 3);
    for y in 0..height {
        for x in 0..width {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let noise = (state & 0x3F).to_le_bytes()[0];
            let stripe = if (x / 64 + y / 64) % 2 == 0 { 40_u8 } else { 0 };
            let r = ((x * 255) / width.max(1)).to_le_bytes()[0];
            let g = ((y * 255) / height.max(1)).to_le_bytes()[0];
            let b = (((x + y) * 255) / (width + height).max(1)).to_le_bytes()[0];
            pixels.extend_from_slice(&[
                r.saturating_add(noise),
                g.saturating_add(stripe),
                b.wrapping_add(noise / 2),
            ]);
        }
    }
    let file = BufWriter::new(File::create(output).map_err(|e| e.to_string())?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&pixels)
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` × `height` image as straight-alpha RGBA: gradients, with
    /// alpha falling from opaque when `alpha` is set.
    fn image(width: u32, height: u32, alpha: bool) -> Vec<u8> {
        let byte = |n: u32| u8::try_from(n.min(255)).unwrap_or(255);
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let a = if alpha {
                    byte(255 - (x + y) * 255 / (width + height))
                } else {
                    255
                };
                pixels.extend_from_slice(&[byte(x * 255 / width), byte(y * 255 / height), 200, a]);
            }
        }
        pixels
    }

    fn png_file(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut file = Vec::new();
        let mut encoder = png::Encoder::new(&mut file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(pixels).expect("PNG data");
        writer.finish().expect("PNG end");
        file
    }

    fn premultiply(pixels: &[u8]) -> Vec<u8> {
        pixels
            .chunks_exact(4)
            .flat_map(|p| decoders::premultiplied(p[0], p[1], p[2], p[3]))
            .collect()
    }

    #[test]
    fn sniffing_names_each_format() {
        let mut avif = vec![0, 0, 0, 0x1C];
        avif.extend_from_slice(b"ftypavif");
        let mut heic = vec![0, 0, 0, 0x18];
        heic.extend_from_slice(b"ftypheic");
        let png = png_file(&image(2, 2, false), 2, 2);
        assert_eq!(decoders::sniff(&png), Some("png"));
        assert_eq!(decoders::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpeg"));
        assert_eq!(decoders::sniff(b"GIF89a"), Some("gif"));
        assert_eq!(decoders::sniff(b"RIFF\0\0\0\0WEBPVP8L"), Some("webp"));
        assert_eq!(decoders::sniff(&avif), Some("avif"));
        assert_eq!(decoders::sniff(&heic), Some("heif"));
        assert_eq!(decoders::sniff(b"BM"), None);
    }

    #[test]
    fn premultiplication_rounds_to_nearest() {
        assert_eq!(decoders::premultiplied(255, 128, 0, 128), [128, 64, 0, 128]);
        assert_eq!(decoders::premultiplied(10, 20, 30, 255), [10, 20, 30, 255]);
        assert_eq!(decoders::premultiplied(10, 20, 30, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn png_decodes_to_premultiplied_rgba() {
        let pixels = image(37, 23, true);
        let (decoded, _) = decoders::decode(&png_file(&pixels, 37, 23)).expect("decodes");
        assert_eq!((decoded.width, decoded.height), (37, 23));
        assert_eq!(decoded.rgba, premultiply(&pixels));
    }

    #[test]
    fn lossless_webp_round_trips() {
        let pixels = image(41, 17, false);
        let mut file = Vec::new();
        image_webp::WebPEncoder::new(&mut file)
            .encode(&pixels, 41, 17, image_webp::ColorType::Rgba8)
            .expect("encodes");
        let (decoded, _) = decoders::decode(&file).expect("decodes");
        assert_eq!((decoded.width, decoded.height), (41, 17));
        assert_eq!(decoded.rgba, pixels);
    }

    #[test]
    fn imageio_agrees_with_the_rust_decoder_on_png() {
        for alpha in [false, true] {
            let pixels = image(64, 48, alpha);
            let file = png_file(&pixels, 64, 48);
            let (ours, _) = decoders::decode(&file).expect("Rust decodes");
            for cache in [Cache::Immediately, Cache::Never] {
                let (theirs, _, _) =
                    platform::imageio::decode(&file, cache).expect("ImageIO decodes");
                assert_eq!((theirs.width, theirs.height), (64, 48));
                let max = ours
                    .rgba
                    .iter()
                    .zip(&theirs.rgba)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max();
                assert_eq!(max, Some(0), "alpha {alpha}, {cache:?}");
            }
        }
    }
}

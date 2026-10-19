//! The safe Rust decoders (IM2), each to sRGB premultiplied RGBA so their
//! output compares with ImageIO's drawn into the same format. None of them
//! applies colour management: the test images are sRGB.

use std::io::Cursor;
use std::time::{Duration, Instant};

use crate::Decoded;

/// Decode `bytes` with the decoder for its format, sniffed from its first
/// bytes. Returns the pixels and the decode time (conversion to premultiplied
/// RGBA included, as ImageIO's time includes CoreGraphics' drawing).
pub(crate) fn decode(bytes: &[u8]) -> Result<(Decoded, Duration), String> {
    let start = Instant::now();
    let decoded = match sniff(bytes) {
        Some("png") => png_rgba(bytes)?,
        Some("jpeg") => jpeg_rgba(bytes)?,
        Some("gif") => gif_rgba(bytes)?,
        Some("webp") => webp_rgba(bytes)?,
        Some(other) => return Err(format!("no Rust decoder for {other}")),
        None => return Err("unknown format".to_owned()),
    };
    Ok((decoded, start.elapsed()))
}

pub(crate) fn sniff(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("webp"),
        [
            _,
            _,
            _,
            _,
            b'f',
            b't',
            b'y',
            b'p',
            b'a',
            b'v',
            b'i',
            b'f',
            ..,
        ] => Some("avif"),
        [_, _, _, _, b'f', b't', b'y', b'p', ..] => Some("heif"),
        _ => None,
    }
}

pub(crate) fn premultiplied(red: u8, green: u8, blue: u8, alpha: u8) -> [u8; 4] {
    let scale = |c: u8| u8::try_from((u16::from(c) * u16::from(alpha) + 127) / 255).unwrap_or(255);
    [scale(red), scale(green), scale(blue), alpha]
}

fn png_rgba(bytes: &[u8]) -> Result<Decoded, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("PNG too large")?];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (width, height) = (frame.width as usize, frame.height as usize);
    let channels = frame.color_type.samples();
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in buf.chunks(frame.line_size).take(height) {
        for px in row.chunks_exact(channels).take(width) {
            rgba.extend_from_slice(&match *px {
                [g] => [g, g, g, 255],
                [g, a] => premultiplied(g, g, g, a),
                [r, g, b] => [r, g, b, 255],
                [r, g, b, a] => premultiplied(r, g, b, a),
                _ => [0; 4],
            });
        }
    }
    Ok(Decoded {
        width,
        height,
        rgba,
        format: "png".to_owned(),
    })
}

fn jpeg_rgba(bytes: &[u8]) -> Result<Decoded, String> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let pixels = decoder.decode().map_err(|e| e.to_string())?;
    let info = decoder.info().ok_or("no JPEG info")?;
    let (width, height) = (usize::from(info.width), usize::from(info.height));
    let rgba: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        jpeg_decoder::PixelFormat::RGB24 => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => return Err(format!("unsupported JPEG pixel format {other:?}")),
    };
    Ok(Decoded {
        width,
        height,
        rgba,
        format: "jpeg".to_owned(),
    })
}

/// The first frame, composited onto the logical screen as ImageIO does.
fn gif_rgba(bytes: &[u8]) -> Result<Decoded, String> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(Cursor::new(bytes))
        .map_err(|e| e.to_string())?;
    let (width, height) = (usize::from(decoder.width()), usize::from(decoder.height()));
    let mut rgba = vec![0_u8; width * height * 4];
    if let Some(frame) = decoder.read_next_frame().map_err(|e| e.to_string())? {
        let (left, top) = (usize::from(frame.left), usize::from(frame.top));
        let frame_width = usize::from(frame.width);
        for (i, px) in frame.buffer.chunks_exact(4).enumerate() {
            let (x, y) = (left + i % frame_width, top + i / frame_width);
            if x < width && y < height {
                let at = (y * width + x) * 4;
                rgba[at..at + 4].copy_from_slice(&premultiplied(px[0], px[1], px[2], px[3]));
            }
        }
    }
    Ok(Decoded {
        width,
        height,
        rgba,
        format: "gif".to_owned(),
    })
}

fn webp_rgba(bytes: &[u8]) -> Result<Decoded, String> {
    let mut decoder =
        image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    let (width, height) = (w as usize, h as usize);
    let alpha = decoder.has_alpha();
    let mut buf = vec![0; decoder.output_buffer_size().ok_or("WebP too large")?];
    decoder.read_image(&mut buf).map_err(|e| e.to_string())?;
    let rgba = if alpha {
        buf.chunks_exact(4)
            .flat_map(|p| premultiplied(p[0], p[1], p[2], p[3]))
            .collect()
    } else {
        buf.chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect()
    };
    Ok(Decoded {
        width,
        height,
        rgba,
        format: "webp".to_owned(),
    })
}

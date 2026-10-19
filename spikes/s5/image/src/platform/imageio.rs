//! ImageIO decodes an image (IM3: AVIF, in the renderer); CoreGraphics draws
//! it into premultiplied RGBA in a buffer Rust owns.

use std::time::{Duration, Instant};

use objc2_core_foundation::{
    CFBoolean, CFData, CFDictionary, CFString, CFType, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo, kCGColorSpaceSRGB,
};
use objc2_image_io::{
    CGImageSource, kCGImageSourceShouldCache, kCGImageSourceShouldCacheImmediately,
};

use crate::Decoded;

/// When ImageIO decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cache {
    /// When the image is created (`kCGImageSourceShouldCacheImmediately`), so
    /// the decode and the conversion are timed apart. ImageIO keeps its own
    /// decoded copy while the image lives.
    Immediately,
    /// Never (`kCGImageSourceShouldCache` false): CoreGraphics decodes while
    /// drawing, and ImageIO keeps no copy.
    Never,
}

/// Decode the first image of `bytes` and draw it with CoreGraphics into sRGB
/// premultiplied RGBA. Returns the pixels, the time to create the image (the
/// decode itself with `Cache::Immediately`) and the time to draw it (which
/// includes the decode with `Cache::Never`).
pub(crate) fn decode(bytes: &[u8], cache: Cache) -> Result<(Decoded, Duration, Duration), String> {
    let data = CFData::from_bytes(bytes);
    // SAFETY: I2.
    let (immediately, should_cache): (&CFString, &CFString) = unsafe {
        (
            kCGImageSourceShouldCacheImmediately,
            kCGImageSourceShouldCache,
        )
    };
    let flag = CFBoolean::new(cache == Cache::Immediately);
    let values: [&CFType; 2] = [flag.as_ref(), flag.as_ref()];
    let options = CFDictionary::from_slices(&[immediately, should_cache], &values);
    let start = Instant::now();
    // SAFETY: I3. No options for the source.
    let source = unsafe { CGImageSource::with_data(&data, None) }
        .ok_or("ImageIO did not recognize the data")?;
    // SAFETY: I3. The options map ImageIO's documented keys to CFBooleans.
    let image = unsafe { source.image_at_index(0, Some(options.as_opaque())) }
        .ok_or("ImageIO did not decode the image")?;
    let decode = start.elapsed();
    // SAFETY: I4. `source` is a valid image source.
    let format = unsafe { source.r#type() }.map_or_else(String::new, |t| t.to_string());
    let start = Instant::now();
    let rgba = draw(&image)?;
    let draw_time = start.elapsed();
    Ok((
        Decoded {
            width: CGImage::width(Some(&image)),
            height: CGImage::height(Some(&image)),
            rgba,
            format,
        },
        decode,
        draw_time,
    ))
}

fn draw(image: &CGImage) -> Result<Vec<u8>, String> {
    let (width, height) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    let mut buffer = vec![0_u8; width * height * 4];
    if width == 0 || height == 0 {
        return Ok(buffer);
    }
    // SAFETY: I2.
    let srgb = unsafe { kCGColorSpaceSRGB };
    let space = CGColorSpace::with_name(Some(srgb)).ok_or("no sRGB colour space")?;
    // SAFETY: I1. `buffer` holds width × height × 4 bytes, `width × 4` per
    // row; `context` is dropped before `buffer` is returned.
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
    }
    .ok_or("CGBitmapContextCreate failed")?;
    let to_f64 = |n: usize| f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let rect = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: to_f64(width),
            height: to_f64(height),
        },
    };
    CGContext::draw_image(Some(&context), rect, Some(image));
    drop(context);
    Ok(buffer)
}

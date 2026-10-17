//! CoreText lays out one line of text. CoreGraphics rasterizes it into an
//! 8-bit coverage mask in a buffer that Rust owns.

use core::ptr;

use objc2_core_foundation::{CFAttributedString, CFBoolean, CFDictionary, CFString, CFType};
use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext, CGImageAlphaInfo};
use objc2_core_text::{
    CTFont, CTLine, kCTFontAttributeName, kCTForegroundColorFromContextAttributeName,
};

/// Transparent margin around the glyphs, in pixels.
const PADDING: f64 = 2.0;

/// An 8-bit coverage mask: one byte per pixel (0 = no ink), rows top to
/// bottom, `width` bytes per row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Coverage {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u8>,
}

/// Lay out `text` in the font named `font_name` at `size` pixels, and
/// rasterize it with grayscale antialiasing.
pub(crate) fn rasterize_line(text: &str, font_name: &str, size: f64) -> Result<Coverage, String> {
    // SAFETY: a null matrix means the identity transform
    // (CTFontCreateWithName's contract); the name is a valid CFString.
    let font = unsafe { CTFont::with_name(&CFString::from_str(font_name), size, ptr::null()) };
    // SAFETY: P6.
    let (font_key, ink_key): (&CFString, &CFString) = unsafe {
        (
            kCTFontAttributeName,
            kCTForegroundColorFromContextAttributeName,
        )
    };
    // Ink comes from the context's fill colour. Without this attribute,
    // CoreText draws black, which would leave the mask empty.
    let values: [&CFType; 2] = [(*font).as_ref(), CFBoolean::new(true).as_ref()];
    let attributes = CFDictionary::from_slices(&[font_key, ink_key], &values);
    // SAFETY: the dictionary maps CFString attribute names to values of the
    // types CoreText documents for them (a CTFont, a CFBoolean), which is
    // what CFAttributedStringCreate requires of it.
    let styled = unsafe {
        CFAttributedString::new(
            None,
            Some(&CFString::from_str(text)),
            Some(attributes.as_opaque()),
        )
    }
    .ok_or("CFAttributedStringCreate failed")?;
    // SAFETY: `styled` is a valid attributed string, built just above.
    let line = unsafe { CTLine::with_attributed_string(&styled) };

    let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
    // SAFETY: P2. Three valid pointers to local CGFloats, used only during
    // the call.
    let advance =
        unsafe { line.typographic_bounds(&raw mut ascent, &raw mut descent, &raw mut leading) };
    let width = pixels(advance + 2.0 * PADDING);
    let height = pixels(ascent + descent + 2.0 * PADDING);

    let mut mask = vec![0_u8; width * height];
    let gray = CGColorSpace::new_device_gray().ok_or("no device-gray color space")?;
    // SAFETY: P2. `mask` holds `width` × `height` bytes: 8 bits per pixel,
    // one channel, `width` bytes per row. `context` is dropped below, before
    // `mask` is read or moved, so CoreGraphics never writes to it afterwards.
    let context = unsafe {
        CGBitmapContextCreate(
            mask.as_mut_ptr().cast(),
            width,
            height,
            8,
            width,
            Some(&gray),
            CGImageAlphaInfo::None.0,
        )
    }
    .ok_or("CGBitmapContextCreate failed")?;
    // White ink on a black (empty) mask; the baseline sits above the descent.
    CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
    CGContext::set_text_position(Some(&context), PADDING, PADDING + descent);
    // SAFETY: `line` and `context` are valid. The context draws into `mask`
    // under the P2 argument above.
    unsafe { line.draw(&context) };
    drop(context);

    Ok(Coverage {
        width,
        height,
        pixels: mask,
    })
}

/// A non-negative length in pixels, rounded up.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "text metrics here are small, finite and non-negative"
)]
fn pixels(length: f64) -> usize {
    length.max(0.0).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CoreText needs no window, so this runs under `cargo test`.
    #[test]
    fn a_line_rasterizes_to_a_plausible_mask() {
        let mask = rasterize_line("Axiom", "Helvetica", 40.0).expect("CoreText is available");
        assert_eq!(mask.pixels.len(), mask.width * mask.height);
        // Five glyphs at 40 px: roughly 100 px wide and 40–60 px tall.
        assert!((60..200).contains(&mask.width), "width {}", mask.width);
        assert!((30..80).contains(&mask.height), "height {}", mask.height);
        let ink = mask.pixels.iter().filter(|&&c| c > 0).count();
        assert!(ink > mask.pixels.len() / 10, "too little ink: {ink}");
        // The padding stays empty.
        assert!(
            mask.pixels[..mask.width].iter().all(|&c| c == 0),
            "top row has ink"
        );
        assert_eq!(
            rasterize_line("", "Helvetica", 40.0).map(|m| m.width),
            Ok(4)
        );
    }
}

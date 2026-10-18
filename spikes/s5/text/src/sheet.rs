//! Images for the owner's visual checks (S5-Q1, S5-Q2): masks stacked or
//! tiled, magnified, and written as grayscale or RGB PNG files.

use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

/// A grayscale image being assembled.
#[derive(Clone, Debug)]
pub(crate) struct Sheet {
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// RGB, three bytes per pixel.
    pub(crate) pixels: Vec<u8>,
}

impl Sheet {
    /// A white sheet.
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![255; width * height * 3],
        }
    }

    /// Paint an 8-bit coverage mask as dark ink on the sheet at (`x`, `y`),
    /// each pixel `scale` × `scale`, in `color` (RGB of full ink).
    pub(crate) fn mask(
        &mut self,
        x: usize,
        y: usize,
        mask: &[u8],
        width: usize,
        scale: usize,
        color: [u8; 3],
    ) {
        if width == 0 {
            return;
        }
        for (row, line) in mask.chunks(width).enumerate() {
            for (col, &coverage) in line.iter().enumerate() {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let (px, py) = (x + col * scale + dx, y + row * scale + dy);
                        if px < self.width && py < self.height {
                            let at = (py * self.width + px) * 3;
                            for (pixel, &ink) in self.pixels[at..at + 3].iter_mut().zip(&color) {
                                // Blend toward `color` by coverage, over the
                                // existing pixel.
                                let (old, ink, a) =
                                    (u32::from(*pixel), u32::from(ink), u32::from(coverage));
                                let blended = (old * (255 - a) + ink * a + 127) / 255;
                                *pixel = u8::try_from(blended).unwrap_or(255);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Overlay two masks of the same size at (`x`, `y`), magnified by
    /// `scale`: ink only in `a` red, only in `b` blue, in both black.
    pub(crate) fn overlay(
        &mut self,
        x: usize,
        y: usize,
        a: &[u8],
        b: &[u8],
        width: usize,
        scale: usize,
    ) {
        if width == 0 {
            return;
        }
        for (i, (&ca, &cb)) in a.iter().zip(b).enumerate() {
            let (row, col) = (i / width, i % width);
            let pixel = [255 - cb, 255 - ca.max(cb), 255 - ca];
            for dy in 0..scale {
                for dx in 0..scale {
                    let (px, py) = (x + col * scale + dx, y + row * scale + dy);
                    if px < self.width && py < self.height {
                        let at = (py * self.width + px) * 3;
                        self.pixels[at..at + 3].copy_from_slice(&pixel);
                    }
                }
            }
        }
    }

    pub(crate) fn write(&self, path: &Path) -> io::Result<()> {
        let file = BufWriter::new(File::create(path)?);
        let mut encoder = png::Encoder::new(
            file,
            u32::try_from(self.width).map_err(io::Error::other)?,
            u32::try_from(self.height).map_err(io::Error::other)?,
        );
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(io::Error::other)?;
        writer
            .write_image_data(&self.pixels)
            .map_err(io::Error::other)?;
        writer.finish().map_err(io::Error::other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_blend_by_coverage_over_the_sheet() {
        let mut sheet = Sheet::new(3, 1);
        sheet.mask(0, 0, &[255, 128, 0], 3, 1, [0, 0, 255]);
        assert_eq!(sheet.pixels, [0, 0, 255, 127, 127, 255, 255, 255, 255]);
    }

    #[test]
    fn overlays_colour_each_mask_apart() {
        let mut sheet = Sheet::new(3, 1);
        sheet.overlay(0, 0, &[255, 0, 255], &[0, 255, 255], 3, 1);
        // Only in `a`: red; only in `b`: blue; in both: black.
        assert_eq!(sheet.pixels, [255, 0, 0, 0, 0, 255, 0, 0, 0]);
    }
}

//! A small TrueType font built in code, so that tests need no font files:
//! `.notdef`, a square for `A`, a triangle for `B`, a composite of the square
//! for `C`, and an empty glyph for space. Units per em: 1,000.

use crate::bytes::{put_i16, put_u16, put_u32};
use crate::sfnt::{self, Table};

/// A point of a simple glyph; every point is on the curve.
type Contour = &'static [(i16, i16)];

const NOTDEF: &[Contour] = &[&[(50, 0), (50, 700), (450, 700), (450, 0)]];
const SQUARE: &[Contour] = &[&[(100, 0), (100, 500), (600, 500), (600, 0)]];
const TRIANGLE: &[Contour] = &[&[(100, 0), (350, 500), (600, 0)]];
/// The composite's offset from the square.
const COMPOSITE_DX: i16 = 50;
const NUM_GLYPHS: u16 = 5;
const ADVANCE: u16 = 700;

/// The font's bytes, as an sfnt.
pub fn square_font() -> Vec<u8> {
    let glyphs = [
        simple_glyph(NOTDEF),
        simple_glyph(SQUARE),
        simple_glyph(TRIANGLE),
        composite_glyph(),
        Vec::new(),
    ];
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    for glyph in &glyphs {
        put_u32(&mut loca, u32::try_from(glyf.len()).expect("small"));
        glyf.extend_from_slice(glyph);
        while glyf.len() % 4 != 0 {
            glyf.push(0);
        }
    }
    put_u32(&mut loca, u32::try_from(glyf.len()).expect("small"));

    let mut hmtx = Vec::new();
    for lsb in [50, 100, 100, 150, 0] {
        put_u16(&mut hmtx, ADVANCE);
        put_i16(&mut hmtx, lsb);
    }

    let tables = vec![
        Table {
            tag: *b"head",
            data: head(),
        },
        Table {
            tag: *b"hhea",
            data: hhea(),
        },
        Table {
            tag: *b"maxp",
            data: maxp(),
        },
        Table {
            tag: *b"hmtx",
            data: hmtx,
        },
        Table {
            tag: *b"cmap",
            data: cmap(),
        },
        Table {
            tag: *b"loca",
            data: loca,
        },
        Table {
            tag: *b"glyf",
            data: glyf,
        },
        Table {
            tag: *b"post",
            data: post(),
        },
    ];
    sfnt::write(sfnt::TRUETYPE, tables).expect("a valid font")
}

/// The glyph for each test character.
pub const GLYPH_A: u32 = 1;
pub const GLYPH_B: u32 = 2;
pub const GLYPH_C: u32 = 3;
pub const GLYPH_SPACE: u32 = 4;

fn bbox(contours: &[Contour]) -> (i16, i16, i16, i16) {
    let points = contours.iter().flat_map(|c| c.iter());
    let (mut x0, mut y0, mut x1, mut y1) = (i16::MAX, i16::MAX, i16::MIN, i16::MIN);
    for &(x, y) in points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (x0, y0, x1, y1)
}

/// A simple glyph with uncompressed flags and two-byte coordinate deltas.
fn simple_glyph(contours: &[Contour]) -> Vec<u8> {
    let mut out = Vec::new();
    put_i16(
        &mut out,
        i16::try_from(contours.len()).expect("few contours"),
    );
    let (x0, y0, x1, y1) = bbox(contours);
    for v in [x0, y0, x1, y1] {
        put_i16(&mut out, v);
    }
    let mut end = 0_u16;
    for contour in contours {
        end += u16::try_from(contour.len()).expect("few points");
        put_u16(&mut out, end - 1);
    }
    put_u16(&mut out, 0); // instructionLength
    let points: Vec<(i16, i16)> = contours.iter().flat_map(|c| c.iter().copied()).collect();
    out.extend(std::iter::repeat_n(0x01_u8, points.len())); // on curve, long deltas
    let mut last = 0;
    for &(x, _) in &points {
        put_i16(&mut out, x - last);
        last = x;
    }
    last = 0;
    for &(_, y) in &points {
        put_i16(&mut out, y - last);
        last = y;
    }
    out
}

/// One component: the square, moved right by `COMPOSITE_DX`.
fn composite_glyph() -> Vec<u8> {
    const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
    const ARGS_ARE_XY_VALUES: u16 = 0x0002;
    let mut out = Vec::new();
    put_i16(&mut out, -1);
    let (x0, y0, x1, y1) = bbox(SQUARE);
    for v in [x0 + COMPOSITE_DX, y0, x1 + COMPOSITE_DX, y1] {
        put_i16(&mut out, v);
    }
    put_u16(&mut out, ARG_1_AND_2_ARE_WORDS | ARGS_ARE_XY_VALUES);
    put_u16(&mut out, u16::try_from(GLYPH_A).expect("small"));
    put_i16(&mut out, COMPOSITE_DX);
    put_i16(&mut out, 0);
    out
}

fn head() -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, 0x0001_0000); // version
    put_u32(&mut out, 0x0001_0000); // fontRevision
    put_u32(&mut out, 0); // checkSumAdjustment
    put_u32(&mut out, 0x5F0F_3CF5); // magicNumber
    put_u16(&mut out, 0x000B); // flags
    put_u16(&mut out, 1000); // unitsPerEm
    out.extend_from_slice(&[0; 16]); // created, modified
    for v in [50, 0, 650, 700] {
        put_i16(&mut out, v); // xMin, yMin, xMax, yMax
    }
    put_u16(&mut out, 0); // macStyle
    put_u16(&mut out, 8); // lowestRecPPEM
    put_i16(&mut out, 2); // fontDirectionHint
    put_i16(&mut out, 1); // indexToLocFormat: long offsets
    put_i16(&mut out, 0); // glyphDataFormat
    out
}

fn hhea() -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, 0x0001_0000);
    put_i16(&mut out, 800); // ascender
    put_i16(&mut out, -200); // descender
    put_i16(&mut out, 0); // lineGap
    put_u16(&mut out, ADVANCE); // advanceWidthMax
    put_i16(&mut out, 0); // minLeftSideBearing
    put_i16(&mut out, 50); // minRightSideBearing
    put_i16(&mut out, 650); // xMaxExtent
    put_i16(&mut out, 1); // caretSlopeRise
    put_i16(&mut out, 0); // caretSlopeRun
    put_i16(&mut out, 0); // caretOffset
    out.extend_from_slice(&[0; 8]); // reserved
    put_i16(&mut out, 0); // metricDataFormat
    put_u16(&mut out, NUM_GLYPHS); // numberOfHMetrics
    out
}

fn maxp() -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, 0x0001_0000);
    put_u16(&mut out, NUM_GLYPHS);
    for v in [4, 1, 4, 1, 2, 0, 0, 0, 0, 0, 0, 1, 1] {
        put_u16(&mut out, v);
    }
    out
}

/// A format 4 subtable for (3, 1): space → 4, `A`–`C` → 1–3.
fn cmap() -> Vec<u8> {
    let segments: [(u16, u16, i16); 3] = [
        (0x20, 0x20, 4 - 0x20),
        (0x41, 0x43, 1 - 0x41),
        (0xFFFF, 0xFFFF, 1),
    ];
    let seg_count = u16::try_from(segments.len()).expect("three");
    let mut sub = Vec::new();
    put_u16(&mut sub, 4); // format
    put_u16(&mut sub, 16 + 8 * seg_count); // length
    put_u16(&mut sub, 0); // language
    put_u16(&mut sub, seg_count * 2);
    put_u16(&mut sub, 4); // searchRange
    put_u16(&mut sub, 1); // entrySelector
    put_u16(&mut sub, 2); // rangeShift
    for &(_, end, _) in &segments {
        put_u16(&mut sub, end);
    }
    put_u16(&mut sub, 0); // reservedPad
    for &(start, _, _) in &segments {
        put_u16(&mut sub, start);
    }
    for &(_, _, delta) in &segments {
        put_i16(&mut sub, delta);
    }
    for _ in &segments {
        put_u16(&mut sub, 0); // idRangeOffset
    }
    let mut out = Vec::new();
    put_u16(&mut out, 0); // version
    put_u16(&mut out, 1); // numTables
    put_u16(&mut out, 3); // platformID: Windows
    put_u16(&mut out, 1); // encodingID: Unicode BMP
    put_u32(&mut out, 12); // offset
    out.extend_from_slice(&sub);
    out
}

fn post() -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, 0x0003_0000); // version 3: no glyph names
    put_u32(&mut out, 0); // italicAngle
    put_i16(&mut out, -100); // underlinePosition
    put_i16(&mut out, 50); // underlineThickness
    out.extend_from_slice(&[0; 20]); // isFixedPitch, memory fields
    out
}

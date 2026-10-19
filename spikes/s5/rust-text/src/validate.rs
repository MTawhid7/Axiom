//! Validation of a decoded font before use (TX7): its table directory and
//! required tables parse with `read-fonts`, and it stays within [`Limits`].
//!
//! This is not a sanitizer like OTS. The parsers behind it are memory-safe
//! Rust, so validation only has to stop fonts that would be useless or
//! excessive; anything subtler shows up as a shaping or drawing error, which
//! the caller counts.

use read_fonts::types::Tag;
use read_fonts::{FontRef, TableProvider};
use skrifa::charmap::Charmap;

use crate::Limits;
use crate::bytes::Error;

/// How a font's glyphs are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outlines {
    Glyf,
    Cff,
    Cff2,
    /// Bitmaps only (`sbix`, `CBDT` or `EBDT`).
    Bitmaps,
}

/// What validation learned about a font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub num_glyphs: u16,
    pub units_per_em: u16,
    pub outlines: Outlines,
    pub tables: usize,
}

const HEAD_MAGIC: u32 = 0x5F0F_3CF5;

/// Validate font `index` of `data` (an sfnt, or a collection).
pub fn validate(data: &[u8], index: u32, limits: &Limits) -> Result<Summary, Error> {
    if data.len() > limits.max_sfnt {
        return Err(Error("font larger than the limit"));
    }
    let font =
        FontRef::from_index(data, index).map_err(|_| Error("table directory does not parse"))?;
    let tables = font.table_directory().table_records().len();
    if tables > limits.max_tables {
        return Err(Error("more tables than the limit"));
    }
    let head = font
        .head()
        .map_err(|_| Error("head missing or malformed"))?;
    if head.magic_number() != HEAD_MAGIC {
        return Err(Error("head.magicNumber is wrong"));
    }
    let units_per_em = head.units_per_em();
    if !(16..=16_384).contains(&units_per_em) {
        return Err(Error("unitsPerEm out of range"));
    }
    let num_glyphs = font
        .maxp()
        .map_err(|_| Error("maxp missing or malformed"))?
        .num_glyphs();
    if num_glyphs == 0 {
        return Err(Error("no glyphs"));
    }
    let hhea = font
        .hhea()
        .map_err(|_| Error("hhea missing or malformed"))?;
    let num_hmetrics = hhea.number_of_h_metrics();
    if num_hmetrics == 0 || num_hmetrics > num_glyphs {
        return Err(Error("numberOfHMetrics out of range"));
    }
    let hmtx = font
        .hmtx()
        .map_err(|_| Error("hmtx missing or malformed"))?;
    if hmtx.h_metrics().len() != usize::from(num_hmetrics) {
        return Err(Error("hmtx shorter than numberOfHMetrics"));
    }
    font.cmap()
        .map_err(|_| Error("cmap missing or malformed"))?;
    if !Charmap::new(&font).has_map() {
        return Err(Error("cmap has no Unicode subtable"));
    }

    let has = |tag: &[u8; 4]| font.table_data(Tag::new(tag)).is_some();
    let outlines = if has(b"glyf") || has(b"loca") {
        let is_long = match head.index_to_loc_format() {
            0 => false,
            1 => true,
            _ => return Err(Error("unknown indexToLocFormat")),
        };
        let loca = font
            .loca(is_long)
            .map_err(|_| Error("loca missing or malformed"))?;
        let glyf = font
            .glyf()
            .map_err(|_| Error("glyf missing or malformed"))?;
        if loca.len() != usize::from(num_glyphs) {
            return Err(Error("loca's length disagrees with maxp.numGlyphs"));
        }
        if !loca.all_offsets_are_ascending() {
            return Err(Error("loca offsets are not ascending"));
        }
        let end = loca.get_raw(loca.len()).unwrap_or(0) as usize;
        if end > glyf.offset_data().len() {
            return Err(Error("loca points past the end of glyf"));
        }
        Outlines::Glyf
    } else if has(b"CFF ") {
        font.cff().map_err(|_| Error("CFF malformed"))?;
        Outlines::Cff
    } else if has(b"CFF2") {
        font.cff2().map_err(|_| Error("CFF2 malformed"))?;
        Outlines::Cff2
    } else if has(b"sbix") || has(b"CBDT") || has(b"EBDT") {
        Outlines::Bitmaps
    } else {
        return Err(Error("no outlines and no bitmaps"));
    };
    Ok(Summary {
        num_glyphs,
        units_per_em,
        outlines,
        tables,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testfont;

    #[test]
    fn accepts_the_test_font() {
        let summary = validate(&testfont::square_font(), 0, &Limits::default()).expect("valid");
        assert_eq!(summary.num_glyphs, 5);
        assert_eq!(summary.units_per_em, 1000);
        assert_eq!(summary.outlines, Outlines::Glyf);
    }

    #[test]
    fn rejects_a_font_without_cmap() {
        let (flavor, tables) = crate::sfnt::tables(&testfont::square_font()).expect("reads");
        let tables = tables.into_iter().filter(|t| &t.tag != b"cmap").collect();
        let font = crate::sfnt::write(flavor, tables).expect("writes");
        assert_eq!(
            validate(&font, 0, &Limits::default()),
            Err(Error("cmap missing or malformed"))
        );
    }
}

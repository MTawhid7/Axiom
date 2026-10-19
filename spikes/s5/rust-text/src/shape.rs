//! Shaping with HarfRust (TX9): one run of one font, script, language and
//! direction, with the rest of the paragraph as context.

use std::ops::Range;
use std::sync::Arc;

pub use harfrust::Font;
use harfrust::font::{Blob, Source, TableFunction, Variation};
use harfrust::{
    Buffer, BufferFlags, Direction, Language, ShapeError, ShapeOptions, ShaperFont, Tag,
};
use read_fonts::TableProvider;

/// One shaped glyph. Positions are in font units; `cluster` is a UTF-16
/// offset into the paragraph (TX4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub id: u32,
    pub cluster: u32,
    pub x_advance: i32,
    pub y_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

/// How to shape a run.
#[derive(Clone, Copy, Debug, Default)]
pub struct Params<'a> {
    /// The direction; guessed from the text's script if `None`.
    pub rtl: Option<bool>,
    /// A BCP 47 language tag.
    pub language: Option<&'a str>,
    /// The size in points, for AAT's `trak` table. `None` applies no tracking.
    pub point_size: Option<f32>,
}

/// Font `index` of `data` (an sfnt or a collection, in memory or mapped), at
/// the given variation axis values in user space, such as `(*b"wght", 700.0)`.
pub fn font(
    data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    index: u32,
    variations: &[([u8; 4], f32)],
) -> Option<Font> {
    let blob = Blob::Shared(data);
    Some(at_variations(Font::new(blob, index)?, variations))
}

fn at_variations(font: Font, variations: &[([u8; 4], f32)]) -> Font {
    if variations.is_empty() {
        return font;
    }
    let settings: Vec<Variation> = variations
        .iter()
        .map(|&(tag, value)| Variation::new(Tag::new(&tag), value))
        .collect();
    font.instance_builder().variations(settings).build()
}

/// [`font`], with the tables in `hidden` invisible to HarfRust. Axiom applies
/// size-dependent adjustments such as AAT tracking itself, after shaping
/// (TX10), so its shaper must not see `trak`.
pub fn font_hiding(
    data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    index: u32,
    variations: &[([u8; 4], f32)],
    hidden: &[[u8; 4]],
) -> Option<Font> {
    let len = (*data).as_ref().len();
    let records: Vec<([u8; 4], Range<usize>)> =
        read_fonts::FontRef::from_index((*data).as_ref(), index)
            .ok()?
            .table_directory()
            .table_records()
            .iter()
            .filter_map(|r| {
                let start = r.offset() as usize;
                let end = start
                    .checked_add(r.length() as usize)
                    .filter(|&end| end <= len)?;
                Some((r.tag().to_be_bytes(), start..end))
            })
            .filter(|(tag, _)| !hidden.contains(tag))
            .collect();
    let tables = move |tag: Tag| -> Option<Blob> {
        let tag = tag.to_be_bytes();
        let (_, range) = records.iter().find(|(t, _)| *t == tag)?;
        Some(Blob::Shared(Arc::new(Slice {
            data: Arc::clone(&data),
            range: range.clone(),
        })))
    };
    let source = Source::TableFunction(TableFunction::new(Arc::new(tables)));
    Some(at_variations(Font::new(source, 0)?, variations))
}

/// One table's bytes inside a shared font.
struct Slice {
    data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    range: Range<usize>,
}

impl AsRef<[u8]> for Slice {
    fn as_ref(&self) -> &[u8] {
        &(*self.data).as_ref()[self.range.clone()]
    }
}

/// The normal track's tracking at `point_size`, in font units: the
/// horizontal track whose value is 0 in Apple's `trak` table, interpolated
/// between the table's sizes as HarfRust interpolates them; zero when the font
/// has no normal track. (HarfRust instead applies a font's only track whatever
/// its value, and CoreText doesn't; S5-Q1.)
pub fn normal_tracking(data: &[u8], index: u32, point_size: f32) -> f32 {
    let Ok(font) = read_fonts::FontRef::from_index(data, index) else {
        return 0.0;
    };
    let Ok(trak) = font.trak() else {
        return 0.0;
    };
    let Some(Ok(horiz)) = trak.horiz() else {
        return 0.0;
    };
    let offset_data = trak.offset_data();
    let Some(sizes) = horiz.size_table(offset_data) else {
        return 0.0;
    };
    let Some(normal) = horiz
        .track_table()
        .iter()
        .find(|t| t.track().to_f32() == 0.0)
    else {
        return 0.0;
    };
    let Some(values) =
        normal.per_size_values(offset_data, u16::try_from(sizes.len()).unwrap_or(u16::MAX))
    else {
        return 0.0;
    };
    let n = sizes.len().min(values.len());
    for i in 0..n {
        let size = sizes[i].get().to_f32();
        if size >= point_size {
            let v1 = f32::from(values[i].get());
            if i == 0 {
                return v1;
            }
            let s0 = sizes[i - 1].get().to_f32();
            let v0 = f32::from(values[i - 1].get());
            if (size - s0).abs() < f32::EPSILON {
                return f32::midpoint(v0, v1);
            }
            return v0 + (point_size - s0) / (size - s0) * (v1 - v0);
        }
    }
    values
        .get(n.saturating_sub(1))
        .map_or(0.0, |v| f32::from(v.get()))
}

/// Add `tracking` font units to the advance of the first glyph of each
/// cluster, where HarfRust adds it.
pub fn apply_tracking(glyphs: &mut [Glyph], tracking: f32) {
    #[expect(clippy::cast_possible_truncation, reason = "tracking values are small")]
    let add = tracking.round() as i32;
    if add == 0 {
        return;
    }
    let mut previous = None;
    for glyph in glyphs {
        if previous != Some(glyph.cluster) {
            glyph.x_advance += add;
        }
        previous = Some(glyph.cluster);
    }
}

/// Shape the UTF-16 range `range` of `text`, with the rest of `text` as
/// context.
pub fn shape(
    font: &Font,
    text: &str,
    range: Range<usize>,
    params: &Params<'_>,
) -> Result<Vec<Glyph>, ShapeError> {
    shape_with(&ShaperFont::new(font), text, range, params)
}

/// [`shape`] with a prepared [`ShaperFont`], which callers that shape many
/// runs of one font keep (the speed measurement does).
pub fn shape_with(
    shaper: &ShaperFont<'_, '_>,
    text: &str,
    range: Range<usize>,
    params: &Params<'_>,
) -> Result<Vec<Glyph>, ShapeError> {
    let mut buffer = Buffer::new();
    let mut utf16 = 0;
    let mut bytes_start = None;
    let mut bytes_end = text.len();
    for (byte, ch) in text.char_indices() {
        if utf16 >= range.end {
            bytes_end = byte;
            break;
        }
        if utf16 >= range.start {
            bytes_start.get_or_insert(byte);
            buffer.push(u32::from(ch), u32::try_from(utf16).unwrap_or(u32::MAX));
        }
        utf16 += ch.len_utf16();
    }
    let bytes_start = bytes_start.unwrap_or(bytes_end);
    buffer.set_pre_context(&text[..bytes_start]);
    buffer.set_post_context(&text[bytes_end..]);
    if let Some(language) = params.language.and_then(|tag| tag.parse::<Language>().ok()) {
        buffer.set_language(Some(language));
    }
    buffer.guess_segment_properties();
    match params.rtl {
        Some(true) => buffer.set_direction(Direction::RightToLeft),
        Some(false) => buffer.set_direction(Direction::LeftToRight),
        None => {}
    }
    let mut flags = BufferFlags::default();
    if range.start == 0 {
        flags |= BufferFlags::BEGINNING_OF_TEXT;
    }
    if bytes_end == text.len() {
        flags |= BufferFlags::END_OF_TEXT;
    }
    buffer.set_flags(flags);
    harfrust::shape(
        shaper,
        &mut buffer,
        ShapeOptions::new().point_size(params.point_size),
    )?;
    Ok(buffer
        .glyph_infos()
        .iter()
        .zip(buffer.glyph_positions())
        .map(|(info, pos)| Glyph {
            id: info.glyph_id,
            cluster: info.cluster,
            x_advance: pos.x_advance,
            y_advance: pos.y_advance,
            x_offset: pos.x_offset,
            y_offset: pos.y_offset,
        })
        .collect())
}

/// The UTF-16 length of `text`.
pub fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bytes, sfnt, testfont};

    #[test]
    fn shapes_the_test_font_with_utf16_clusters() {
        let font = font(Arc::new(testfont::square_font()), 0, &[]).expect("parses");
        // "é" is one UTF-16 unit but two UTF-8 bytes; the test font lacks it,
        // so it shapes as .notdef.
        let text = "éA BC";
        let glyphs = shape(&font, text, 0..utf16_len(text), &Params::default()).expect("shapes");
        let ids: Vec<u32> = glyphs.iter().map(|g| g.id).collect();
        assert_eq!(
            ids,
            vec![
                0,
                testfont::GLYPH_A,
                testfont::GLYPH_SPACE,
                testfont::GLYPH_B,
                testfont::GLYPH_C
            ]
        );
        let clusters: Vec<u32> = glyphs.iter().map(|g| g.cluster).collect();
        assert_eq!(clusters, vec![0, 1, 2, 3, 4]);
        assert!(glyphs.iter().all(|g| g.x_advance == 700));
    }

    /// The test font with a `trak` table whose horizontal tracks are
    /// `tracks`: (track value, per-size values) at sizes 12 and 24 points.
    fn tracked_font(tracks: &[(i16, [i16; 2])]) -> Vec<u8> {
        let count = u16::try_from(tracks.len()).expect("few tracks");
        let size_table = 12 + 8 + 8 * count;
        let mut trak = Vec::new();
        bytes::put_u32(&mut trak, 0x0001_0000); // version
        bytes::put_u16(&mut trak, 0); // format
        bytes::put_u16(&mut trak, 12); // horizOffset
        bytes::put_u16(&mut trak, 0); // vertOffset
        bytes::put_u16(&mut trak, 0); // reserved
        bytes::put_u16(&mut trak, count); // nTracks
        bytes::put_u16(&mut trak, 2); // nSizes
        bytes::put_u32(&mut trak, u32::from(size_table));
        for (i, &(track, _)) in (0_u16..).zip(tracks) {
            bytes::put_u32(
                &mut trak,
                u32::from_be_bytes((i32::from(track) << 16).to_be_bytes()),
            );
            bytes::put_u16(&mut trak, 256 + i); // nameIndex
            bytes::put_u16(&mut trak, size_table + 8 + 4 * i);
        }
        for size in [12_u32, 24] {
            bytes::put_u32(&mut trak, size << 16);
        }
        for &(_, values) in tracks {
            for value in values {
                bytes::put_i16(&mut trak, value);
            }
        }
        let (flavor, mut tables) = sfnt::tables(&testfont::square_font()).expect("reads");
        tables.push(sfnt::Table {
            tag: *b"trak",
            data: trak,
        });
        sfnt::write(flavor, tables).expect("writes")
    }

    #[test]
    fn normal_tracking_interpolates_the_zero_track_only() {
        let data = tracked_font(&[(-1, [-40, -80]), (0, [10, 20])]);
        assert!((normal_tracking(&data, 0, 6.0) - 10.0).abs() < 1e-6);
        assert!((normal_tracking(&data, 0, 12.0) - 10.0).abs() < 1e-6);
        assert!((normal_tracking(&data, 0, 18.0) - 15.0).abs() < 1e-6);
        assert!((normal_tracking(&data, 0, 36.0) - 20.0).abs() < 1e-6);
        // A font with only a tight track has no normal tracking (CoreText
        // applies none; HarfRust would apply the tight track).
        let tight_only = tracked_font(&[(-1, [-40, -80])]);
        assert!(normal_tracking(&tight_only, 0, 18.0).abs() < f32::EPSILON);
        assert!(normal_tracking(&testfont::square_font(), 0, 18.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tracking_goes_to_the_first_glyph_of_each_cluster() {
        let glyph = |cluster| Glyph {
            id: 1,
            cluster,
            x_advance: 100,
            y_advance: 0,
            x_offset: 0,
            y_offset: 0,
        };
        let mut glyphs: Vec<Glyph> = [0, 0, 1, 2, 2].into_iter().map(glyph).collect();
        apply_tracking(&mut glyphs, 12.4);
        let advances: Vec<i32> = glyphs.iter().map(|g| g.x_advance).collect();
        assert_eq!(advances, vec![112, 100, 112, 112, 100]);
    }

    #[test]
    fn hidden_tables_are_invisible_to_the_shaper() {
        let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(testfont::square_font());
        let shaped = |font: &Font| -> Vec<u32> {
            shape(font, "A", 0..1, &Params::default())
                .expect("shapes")
                .iter()
                .map(|g| g.id)
                .collect()
        };
        let all = font_hiding(Arc::clone(&data), 0, &[], &[]).expect("parses");
        assert_eq!(shaped(&all), vec![testfont::GLYPH_A]);
        // Without `cmap`, no character maps to a glyph.
        let hidden = font_hiding(data, 0, &[], &[*b"cmap"]).expect("parses");
        assert_eq!(shaped(&hidden), vec![0]);
    }

    #[test]
    fn shapes_a_sub_range_with_context() {
        let font = font(Arc::new(testfont::square_font()), 0, &[]).expect("parses");
        let glyphs = shape(&font, "AB C", 1..3, &Params::default()).expect("shapes");
        let clusters: Vec<u32> = glyphs.iter().map(|g| g.cluster).collect();
        assert_eq!(clusters, vec![1, 2]);
    }
}

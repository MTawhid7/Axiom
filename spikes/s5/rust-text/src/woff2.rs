//! WOFF 2.0 (W3C Recommendation, updated 2024-08-08), decoded in safe Rust
//! (TX7), after the reference decoder's structure (google/woff2,
//! `src/woff2_dec.cc`).
//!
//! What is checked, as the specification and the reference require:
//! - the header: signature, `length`, a non-zero `numTables`; `reserved` and
//!   `totalSfntSize` are not trusted ("MUST NOT reject");
//! - the table directory's `UIntBase128` values, transform versions, and a
//!   zero `transformLength` for `loca`;
//! - a collection's directory, with each font's `glyf` and `loca` paired;
//! - the blocks' layout: compressed data, then metadata, then private data,
//!   with nothing extraneous and nothing overlapping;
//! - a plausible compression ratio, and a Brotli stream that decompresses to
//!   exactly the sum of the tables' lengths;
//! - the transformed `glyf` table's streams, bounding boxes (required for
//!   composite glyphs, forbidden for empty ones) and `loca`'s `origLength`;
//! - the transformed `hmtx` table's flags.
//!
//! Stricter than the reference, where it would silently truncate: a delta,
//! coordinate or `loca` offset that doesn't fit its field is rejected.

use brotli_decompressor::{BrotliResult, brotli_decode};

use crate::Limits;
use crate::bytes::{self, Error, Reader, Result};
use crate::sfnt::{self, Table};

/// The WOFF 2.0 signature, `wOF2`.
pub const SIGNATURE: u32 = 0x774F_4632;
const HEADER_LEN: usize = 48;
/// The reference decoder's limit: "Over 14k test fonts the max compression
/// ratio seen to date was ~20. >100 suggests you wrote a bad uncompressed
/// size."
const MAX_COMPRESSION_RATIO: usize = 100;

const GLYF: [u8; 4] = *b"glyf";
const LOCA: [u8; 4] = *b"loca";
const HMTX: [u8; 4] = *b"hmtx";
const HHEA: [u8; 4] = *b"hhea";
const HEAD: [u8; 4] = *b"head";

/// "Known Table Tags" (§5.1), indexed by the low six bits of a directory
/// entry's flags; 63 means an explicit tag follows.
const KNOWN_TAGS: [[u8; 4]; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ",
    *b"fpgm", *b"glyf", *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp",
    *b"hdmx", *b"kern", *b"LTSH", *b"PCLT", *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF",
    *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT", *b"CBLC", *b"COLR", *b"CPAL",
    *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar", *b"fdsc",
    *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx",
    *b"opbd", *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];
const EXPLICIT_TAG: u8 = 63;

/// A table directory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub tag: [u8; 4],
    /// The transform version, the flags' top two bits.
    pub version: u8,
    pub transformed: bool,
    pub orig_len: u32,
    /// The table's length in the decompressed stream: `transformLength` if
    /// transformed, otherwise `origLength`.
    pub stream_len: u32,
    pub stream_offset: usize,
}

/// One font of the file: the whole file, or one member of a collection.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Font {
    flavor: u32,
    /// Indices into the table directory.
    tables: Vec<usize>,
}

/// The header and directories, read before anything is decompressed.
#[derive(Clone, Debug)]
pub struct Directory {
    pub flavor: u32,
    pub entries: Vec<Entry>,
    fonts: Vec<Font>,
    /// Where the compressed stream starts and ends in the file.
    pub compressed: std::ops::Range<usize>,
    pub uncompressed_len: usize,
}

/// Decode a WOFF2 file into one sfnt per font: one for a single font, or one
/// for each member of a collection (each with its own copy of shared tables).
pub fn decode(data: &[u8], limits: &Limits) -> Result<Vec<Vec<u8>>> {
    let directory = read_directory(data, limits)?;
    let stream = decompress(
        &data[directory.compressed.clone()],
        directory.uncompressed_len,
    )?;
    directory
        .fonts
        .iter()
        .map(|font| reconstruct(&directory, font, &stream))
        .collect()
}

/// Read and check the header, the table directory, any collection directory,
/// and the blocks' layout.
pub fn read_directory(data: &[u8], limits: &Limits) -> Result<Directory> {
    if data.len() > limits.max_file {
        return Err(Error("file larger than the limit"));
    }
    let mut r = Reader::new(data);
    if r.u32()? != SIGNATURE {
        return Err(Error("signature is not wOF2"));
    }
    let flavor = r.u32()?;
    if r.u32()? as usize != data.len() {
        return Err(Error("length does not match the file"));
    }
    let num_tables = usize::from(r.u16()?);
    if num_tables == 0 {
        return Err(Error("numTables is zero"));
    }
    // reserved and totalSfntSize: "a decoder MUST NOT reject" over either.
    r.skip(6)?;
    let compressed_len = r.u32()? as usize;
    r.skip(4)?; // majorVersion, minorVersion: informative.
    let meta_offset = r.u32()? as usize;
    let meta_len = r.u32()? as usize;
    r.skip(4)?; // metaOrigLength: the metadata is never decompressed.
    let priv_offset = r.u32()? as usize;
    let priv_len = r.u32()? as usize;
    for (offset, len) in [(meta_offset, meta_len), (priv_offset, priv_len)] {
        if offset != 0 && (offset >= data.len() || data.len() - offset < len) {
            return Err(Error("metadata or private block outside the file"));
        }
    }

    let entries = read_entries(&mut r, num_tables)?;
    let uncompressed_len = entries
        .last()
        .map_or(0, |e| e.stream_offset + e.stream_len as usize);

    let fonts = if flavor == sfnt::COLLECTION {
        read_collection(&mut r, &entries)?
    } else {
        vec![Font {
            flavor,
            tables: (0..num_tables).collect(),
        }]
    };

    let compressed_start = r.position();
    let compressed_end = compressed_start
        .checked_add(compressed_len)
        .ok_or(Error("offset overflow"))?;
    check_blocks(
        data.len(),
        compressed_end,
        (meta_offset, meta_len),
        (priv_offset, priv_len),
    )?;
    if uncompressed_len == 0 {
        return Err(Error("no table data"));
    }
    if uncompressed_len > MAX_COMPRESSION_RATIO * data.len() {
        return Err(Error("implausible compression ratio"));
    }
    if uncompressed_len > limits.max_sfnt {
        return Err(Error("decoded font larger than the limit"));
    }
    Ok(Directory {
        flavor,
        entries,
        fonts,
        compressed: compressed_start..compressed_end,
        uncompressed_len,
    })
}

/// The table directory (§5.2). Each entry's offset in the decompressed stream
/// is the sum of the lengths before it.
fn read_entries(r: &mut Reader<'_>, num_tables: usize) -> Result<Vec<Entry>> {
    let mut entries = Vec::with_capacity(num_tables);
    let mut stream_offset: usize = 0;
    for _ in 0..num_tables {
        let flags = r.u8()?;
        let index = flags & 0x3F;
        let tag = if index == EXPLICIT_TAG {
            r.tag()?
        } else {
            KNOWN_TAGS[usize::from(index)]
        };
        let version = flags >> 6;
        // Version 0 is glyf and loca's transform, and every other table's
        // null transform (§5.2).
        let transformed = if tag == GLYF || tag == LOCA {
            version == 0
        } else {
            version != 0
        };
        let orig_len = r.base128()?;
        let stream_len = if transformed {
            let len = r.base128()?;
            if tag == LOCA && len != 0 {
                return Err(Error("transformed loca with a non-zero transformLength"));
            }
            len
        } else {
            orig_len
        };
        entries.push(Entry {
            tag,
            version,
            transformed,
            orig_len,
            stream_len,
            stream_offset,
        });
        stream_offset = stream_offset
            .checked_add(stream_len as usize)
            .ok_or(Error("table lengths overflow"))?;
    }
    Ok(entries)
}

/// The blocks' layout: the compressed stream, then the metadata block, then the
/// private block, each starting on a four-byte boundary right after the one
/// before, and the last ending the file (up to padding).
fn check_blocks(
    file_len: usize,
    compressed_end: usize,
    (meta_offset, meta_len): (usize, usize),
    (priv_offset, priv_len): (usize, usize),
) -> Result<()> {
    let mut next = bytes::pad4(compressed_end).ok_or(Error("offset overflow"))?;
    if next > file_len {
        return Err(Error("compressed data extends past the end of the file"));
    }
    if meta_offset != 0 {
        if next != meta_offset {
            return Err(Error("metadata block misplaced"));
        }
        next = bytes::pad4(meta_offset + meta_len).ok_or(Error("offset overflow"))?;
    }
    if priv_offset != 0 {
        if next != priv_offset {
            return Err(Error("private block misplaced"));
        }
        next = bytes::pad4(priv_offset + priv_len).ok_or(Error("offset overflow"))?;
    }
    if Some(next) != bytes::pad4(file_len) {
        return Err(Error("extraneous data between or after the blocks"));
    }
    Ok(())
}

/// The collection directory (§5.3).
fn read_collection(r: &mut Reader<'_>, entries: &[Entry]) -> Result<Vec<Font>> {
    let version = r.u32()?;
    if version != 0x0001_0000 && version != 0x0002_0000 {
        return Err(Error("unknown collection version"));
    }
    let num_fonts = r.u255()?;
    if num_fonts == 0 {
        return Err(Error("collection with no fonts"));
    }
    let mut fonts = Vec::with_capacity(usize::from(num_fonts));
    for _ in 0..num_fonts {
        let num_tables = r.u255()?;
        if num_tables == 0 {
            return Err(Error("collection font with no tables"));
        }
        let flavor = r.u32()?;
        let mut tables = Vec::with_capacity(usize::from(num_tables));
        for _ in 0..num_tables {
            let index = usize::from(r.u255()?);
            if index >= entries.len() {
                return Err(Error("collection font refers to a missing table"));
            }
            tables.push(index);
        }
        // "each loca table MUST immediately follow its corresponding glyf
        // table", so a font's glyf and loca are paired by position.
        let glyf = tables.iter().copied().find(|&i| entries[i].tag == GLYF);
        let loca = tables.iter().copied().find(|&i| entries[i].tag == LOCA);
        if let (Some(glyf), Some(loca)) = (glyf, loca)
            && loca != glyf + 1
        {
            return Err(Error("collection font's glyf and loca are not paired"));
        }
        fonts.push(Font { flavor, tables });
    }
    Ok(fonts)
}

fn decompress(compressed: &[u8], expected: usize) -> Result<Vec<u8>> {
    let mut out = vec![0_u8; expected];
    let info = brotli_decode(compressed, &mut out);
    match info.result {
        BrotliResult::ResultSuccess if info.decoded_size == expected => Ok(out),
        _ => Err(Error(
            "Brotli stream does not decompress to the tables' total length",
        )),
    }
}

/// What the transformed `glyf` table tells the `hmtx` reconstruction.
#[derive(Clone, Debug)]
struct GlyfInfo {
    num_glyphs: u16,
    index_format: u16,
    x_mins: Vec<i16>,
}

fn reconstruct(directory: &Directory, font: &Font, stream: &[u8]) -> Result<Vec<u8>> {
    let entries: Vec<&Entry> = font.tables.iter().map(|&i| &directory.entries[i]).collect();
    let mut tags: Vec<[u8; 4]> = entries.iter().map(|e| e.tag).collect();
    tags.sort_unstable();
    if tags.windows(2).any(|w| w[0] == w[1]) {
        return Err(Error("a font lists one tag twice"));
    }
    let find = |tag: [u8; 4]| entries.iter().copied().find(|e| e.tag == tag);
    let glyf = find(GLYF);
    let loca = find(LOCA);
    match (glyf, loca) {
        (Some(g), Some(l)) if g.transformed != l.transformed => {
            return Err(Error("glyf and loca must be transformed together"));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(Error("glyf without loca, or loca without glyf"));
        }
        _ => {}
    }
    let slice = |e: &Entry| {
        stream
            .get(e.stream_offset..e.stream_offset + e.stream_len as usize)
            .ok_or(Error("table outside the decompressed stream"))
    };

    let mut tables = Vec::with_capacity(entries.len());
    let mut glyf_info = None;
    for entry in &entries {
        if !entry.transformed {
            if entry.tag == HEAD && entry.stream_len < 12 {
                return Err(Error("head table shorter than 12 bytes"));
            }
            tables.push(Table {
                tag: entry.tag,
                data: slice(entry)?.to_vec(),
            });
            continue;
        }
        match entry.tag {
            GLYF => {
                let loca = loca.ok_or(Error("glyf without loca"))?;
                let (glyf_data, loca_data, info) = reconstruct_glyf(slice(entry)?, loca.orig_len)?;
                tables.push(Table {
                    tag: GLYF,
                    data: glyf_data,
                });
                tables.push(Table {
                    tag: LOCA,
                    data: loca_data,
                });
                glyf_info = Some(info);
            }
            // Written with glyf; hmtx needs glyf's bounding boxes first.
            LOCA | HMTX => {}
            _ => return Err(Error("unknown transform for this table")),
        }
    }
    if let Some(entry) = find(HMTX).filter(|e| e.transformed) {
        let info = glyf_info
            .as_ref()
            .ok_or(Error("transformed hmtx without a transformed glyf"))?;
        let hhea = find(HHEA).ok_or(Error("transformed hmtx without hhea"))?;
        let mut hhea = Reader::new(slice(hhea)?);
        hhea.skip(34)?;
        let num_hmetrics = hhea.u16()?;
        tables.push(Table {
            tag: HMTX,
            data: reconstruct_hmtx(slice(entry)?, info, num_hmetrics)?,
        });
    }
    if let Some(info) = &glyf_info
        && let Some(head) = tables.iter().find(|t| t.tag == HEAD)
    {
        let format = head
            .data
            .get(50..52)
            .map(|b| u16::from_be_bytes([b[0], b[1]]));
        if format != Some(info.index_format) {
            return Err(Error(
                "head.indexToLocFormat disagrees with the glyf transform",
            ));
        }
    }
    sfnt::write(font.flavor, tables)
}

/// Rebuild `glyf` and `loca` from the transformed `glyf` table (§5.1–§5.4).
fn reconstruct_glyf(data: &[u8], loca_orig_len: u32) -> Result<(Vec<u8>, Vec<u8>, GlyfInfo)> {
    let mut r = Reader::new(data);
    let _reserved = r.u16()?;
    let option_flags = r.u16()?;
    let num_glyphs = r.u16()?;
    let index_format = r.u16()?;
    if index_format > 1 {
        return Err(Error("unknown loca index format"));
    }
    let expected_loca = (u32::from(num_glyphs) + 1) * if index_format == 0 { 2 } else { 4 };
    if loca_orig_len != expected_loca {
        return Err(Error(
            "loca origLength does not match numGlyphs and indexFormat",
        ));
    }
    let mut streams = Streams::read(&mut r, num_glyphs, option_flags & 1 != 0)?;

    let mut glyf = Vec::new();
    let mut offsets = Vec::with_capacity(usize::from(num_glyphs) + 1);
    let mut x_mins = vec![0_i16; usize::from(num_glyphs)];
    for (i, x_min) in x_mins.iter_mut().enumerate() {
        offsets.push(glyf.len());
        let has_bbox = Streams::bit(streams.bbox_bitmap, i);
        match streams.n_contours.u16()? {
            0xFFFF if has_bbox => *x_min = streams.composite(&mut glyf)?,
            0xFFFF => return Err(Error("composite glyph without a bounding box")),
            0 if has_bbox => return Err(Error("empty glyph with a bounding box")),
            0 => {}
            contours => *x_min = streams.simple(i, contours, has_bbox, &mut glyf)?,
        }
        bytes::pad_to_4(&mut glyf);
    }
    offsets.push(glyf.len());

    let mut loca = Vec::with_capacity(loca_orig_len as usize);
    for offset in offsets {
        if index_format == 0 {
            let half =
                u16::try_from(offset / 2).map_err(|_| Error("glyf too large for short loca"))?;
            bytes::put_u16(&mut loca, half);
        } else {
            let offset = u32::try_from(offset).map_err(|_| Error("glyf larger than 4 GiB"))?;
            bytes::put_u32(&mut loca, offset);
        }
    }
    Ok((
        glyf,
        loca,
        GlyfInfo {
            num_glyphs,
            index_format,
            x_mins,
        },
    ))
}

/// The transformed `glyf` table's seven substreams and two bitmaps (§5.1).
struct Streams<'a> {
    n_contours: Reader<'a>,
    n_points: Reader<'a>,
    flags: Reader<'a>,
    glyphs: Reader<'a>,
    composites: Reader<'a>,
    bboxes: Reader<'a>,
    instructions: Reader<'a>,
    bbox_bitmap: &'a [u8],
    overlap_bitmap: Option<&'a [u8]>,
    /// Scratch space, reused from glyph to glyph.
    points: Vec<(i32, i32, bool)>,
    contour_points: Vec<usize>,
}

impl<'a> Streams<'a> {
    fn read(r: &mut Reader<'a>, num_glyphs: u16, has_overlap_bitmap: bool) -> Result<Self> {
        let mut sizes = [0_usize; 7];
        for size in &mut sizes {
            *size = r.u32()? as usize;
        }
        let mut take = |i: usize| r.take(sizes[i]).map(Reader::new);
        let n_contours = take(0)?;
        let n_points = take(1)?;
        let flags = take(2)?;
        let glyphs = take(3)?;
        let composites = take(4)?;
        let mut bboxes = take(5)?;
        let instructions = take(6)?;
        let overlap_bitmap = if has_overlap_bitmap {
            Some(r.take(usize::from(num_glyphs).div_ceil(8))?)
        } else {
            None
        };
        let bbox_bitmap = bboxes.take(usize::from(num_glyphs).div_ceil(32) * 4)?;
        Ok(Self {
            n_contours,
            n_points,
            flags,
            glyphs,
            composites,
            bboxes,
            instructions,
            bbox_bitmap,
            overlap_bitmap,
            points: Vec::new(),
            contour_points: Vec::new(),
        })
    }

    fn bit(bitmap: &[u8], i: usize) -> bool {
        bitmap[i >> 3] & (0x80 >> (i & 7)) != 0
    }

    /// Write a composite glyph; returns its `xMin`.
    fn composite(&mut self, out: &mut Vec<u8>) -> Result<i16> {
        let (components, has_instructions) = composite_records(&mut self.composites)?;
        bytes::put_i16(out, -1);
        let bbox = self.bboxes.take(8)?;
        out.extend_from_slice(bbox);
        out.extend_from_slice(components);
        if has_instructions {
            let len = self.glyphs.u255()?;
            bytes::put_u16(out, len);
            out.extend_from_slice(self.instructions.take(usize::from(len))?);
        }
        Ok(i16::from_be_bytes([bbox[0], bbox[1]]))
    }

    /// Write simple glyph `i` with `contours` contours; returns its `xMin`.
    fn simple(
        &mut self,
        i: usize,
        contours: u16,
        has_bbox: bool,
        out: &mut Vec<u8>,
    ) -> Result<i16> {
        let contours = i16::try_from(contours).map_err(|_| Error("contour count out of range"))?;
        self.contour_points.clear();
        let mut total: usize = 0;
        for _ in 0..contours {
            let count = usize::from(self.n_points.u255()?);
            self.contour_points.push(count);
            total += count;
            if total > 0x1_0000 {
                return Err(Error("too many points in a glyph"));
            }
        }
        let flags = self.flags.take(total)?;
        decode_triplets(flags, &mut self.glyphs, &mut self.points)?;
        let instruction_len = self.glyphs.u255()?;
        let instructions = self.instructions.take(usize::from(instruction_len))?;
        let bbox = if has_bbox {
            [
                self.bboxes.i16()?,
                self.bboxes.i16()?,
                self.bboxes.i16()?,
                self.bboxes.i16()?,
            ]
        } else {
            computed_bbox(&self.points)?
        };
        bytes::put_i16(out, contours);
        for v in bbox {
            bytes::put_i16(out, v);
        }
        let mut end: usize = 0;
        for &count in &self.contour_points {
            end += count;
            // An empty first contour would end at point -1, which OpenType
            // can't express.
            let last = end.checked_sub(1).ok_or(Error("empty contour"))?;
            bytes::put_u16(
                out,
                u16::try_from(last).map_err(|_| Error("too many points"))?,
            );
        }
        bytes::put_u16(out, instruction_len);
        out.extend_from_slice(instructions);
        let overlap = self
            .overlap_bitmap
            .is_some_and(|bitmap| Self::bit(bitmap, i));
        store_points(&self.points, overlap, out)?;
        Ok(bbox[0])
    }
}

/// A composite glyph's component records, as stored, and whether any of them
/// says instructions follow.
fn composite_records<'a>(stream: &mut Reader<'a>) -> Result<(&'a [u8], bool)> {
    const ARG_1_AND_2_ARE_WORDS: u16 = 1 << 0;
    const WE_HAVE_A_SCALE: u16 = 1 << 3;
    const MORE_COMPONENTS: u16 = 1 << 5;
    const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 1 << 6;
    const WE_HAVE_A_TWO_BY_TWO: u16 = 1 << 7;
    const WE_HAVE_INSTRUCTIONS: u16 = 1 << 8;
    let mut scan = stream.clone();
    let mut len = 0;
    let mut has_instructions = false;
    loop {
        let flags = scan.u16()?;
        has_instructions |= flags & WE_HAVE_INSTRUCTIONS != 0;
        let mut args = 2 + if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            4
        } else {
            2
        };
        if flags & WE_HAVE_A_SCALE != 0 {
            args += 2;
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            args += 4;
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            args += 8;
        }
        scan.skip(args)?;
        len += 2 + args;
        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }
    Ok((stream.take(len)?, has_instructions))
}

/// Decode a simple glyph's points from their flags and the glyph stream's
/// triplets (§5.2, "Triplet Encoding"), as the reference's `TripletDecode`.
fn decode_triplets(
    flags: &[u8],
    glyphs: &mut Reader<'_>,
    points: &mut Vec<(i32, i32, bool)>,
) -> Result<()> {
    fn with_sign(flag: u8, value: i32) -> i32 {
        if flag & 1 != 0 { value } else { -value }
    }
    points.clear();
    let (mut x, mut y) = (0_i32, 0_i32);
    for &raw in flags {
        let on_curve = raw & 0x80 == 0;
        let flag = raw & 0x7F;
        let data_bytes = match flag {
            0..84 => 1,
            84..120 => 2,
            120..124 => 3,
            _ => 4,
        };
        let data = glyphs.take(data_bytes)?;
        let b = |k: usize| i32::from(data[k]);
        let f = i32::from(flag);
        let (dx, dy) = match flag {
            0..10 => (0, with_sign(flag, ((f & 0x0E) << 7) + b(0))),
            10..20 => (with_sign(flag, (((f - 10) & 0x0E) << 7) + b(0)), 0),
            20..84 => {
                let b0 = f - 20;
                (
                    with_sign(flag, 1 + (b0 & 0x30) + (b(0) >> 4)),
                    with_sign(flag >> 1, 1 + ((b0 & 0x0C) << 2) + (b(0) & 0x0F)),
                )
            }
            84..120 => {
                let b0 = f - 84;
                (
                    with_sign(flag, 1 + ((b0 / 12) << 8) + b(0)),
                    with_sign(flag >> 1, 1 + (((b0 % 12) >> 2) << 8) + b(1)),
                )
            }
            120..124 => (
                with_sign(flag, (b(0) << 4) + (b(1) >> 4)),
                with_sign(flag >> 1, ((b(1) & 0x0F) << 8) + b(2)),
            ),
            _ => (
                with_sign(flag, (b(0) << 8) + b(1)),
                with_sign(flag >> 1, (b(2) << 8) + b(3)),
            ),
        };
        x = x.checked_add(dx).ok_or(Error("coordinate overflow"))?;
        y = y.checked_add(dy).ok_or(Error("coordinate overflow"))?;
        points.push((x, y, on_curve));
    }
    Ok(())
}

fn computed_bbox(points: &[(i32, i32, bool)]) -> Result<[i16; 4]> {
    let Some(&(x, y, _)) = points.first() else {
        return Ok([0; 4]);
    };
    let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
    for &(x, y, _) in points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let fit = |v: i32| i16::try_from(v).map_err(|_| Error("coordinate out of range"));
    Ok([fit(x0)?, fit(y0)?, fit(x1)?, fit(y1)?])
}

/// Write a simple glyph's flags and coordinates: one flag per point (no
/// repeats), short forms where a delta fits in a byte.
fn store_points(points: &[(i32, i32, bool)], overlap: bool, out: &mut Vec<u8>) -> Result<()> {
    const ON_CURVE: u8 = 1 << 0;
    const X_SHORT: u8 = 1 << 1;
    const Y_SHORT: u8 = 1 << 2;
    const X_SAME_OR_POSITIVE: u8 = 1 << 4;
    const Y_SAME_OR_POSITIVE: u8 = 1 << 5;
    const OVERLAP_SIMPLE: u8 = 1 << 6;
    let mut deltas = Vec::with_capacity(points.len());
    let (mut last_x, mut last_y) = (0_i32, 0_i32);
    for &(x, y, _) in points {
        let dx = i16::try_from(x - last_x).map_err(|_| Error("delta out of range"))?;
        let dy = i16::try_from(y - last_y).map_err(|_| Error("delta out of range"))?;
        deltas.push((dx, dy));
        (last_x, last_y) = (x, y);
    }
    for (i, (&(_, _, on_curve), &(dx, dy))) in points.iter().zip(&deltas).enumerate() {
        let mut flag = if on_curve { ON_CURVE } else { 0 };
        if overlap && i == 0 {
            flag |= OVERLAP_SIMPLE;
        }
        flag |= match dx {
            0 => X_SAME_OR_POSITIVE,
            1..=255 => X_SHORT | X_SAME_OR_POSITIVE,
            -255..=-1 => X_SHORT,
            _ => 0,
        };
        flag |= match dy {
            0 => Y_SAME_OR_POSITIVE,
            1..=255 => Y_SHORT | Y_SAME_OR_POSITIVE,
            -255..=-1 => Y_SHORT,
            _ => 0,
        };
        out.push(flag);
    }
    for (dx, _) in &deltas {
        push_coordinate(out, *dx);
    }
    for (_, dy) in &deltas {
        push_coordinate(out, *dy);
    }
    Ok(())
}

fn push_coordinate(out: &mut Vec<u8>, delta: i16) {
    match delta {
        0 => {}
        -255..=255 => out.push(delta.unsigned_abs().to_be_bytes()[1]),
        _ => bytes::put_i16(out, delta),
    }
}

/// Rebuild `hmtx` from its transform (§5.4): left side bearings come from
/// `glyf`'s bounding boxes where the flags say the arrays were dropped.
fn reconstruct_hmtx(data: &[u8], glyf: &GlyfInfo, num_hmetrics: u16) -> Result<Vec<u8>> {
    let mut r = Reader::new(data);
    let flags = r.u8()?;
    if flags & 0xFC != 0 {
        return Err(Error("hmtx transform flags use reserved bits"));
    }
    let proportional_lsbs = flags & 1 == 0;
    let monospace_lsbs = flags & 2 == 0;
    if proportional_lsbs && monospace_lsbs {
        return Err(Error("hmtx transform that drops nothing"));
    }
    if num_hmetrics == 0 || num_hmetrics > glyf.num_glyphs {
        return Err(Error("numberOfHMetrics out of range"));
    }
    let n = usize::from(num_hmetrics);
    let mut advances = Vec::with_capacity(n);
    for _ in 0..n {
        advances.push(r.u16()?);
    }
    let mut lsbs = Vec::with_capacity(usize::from(glyf.num_glyphs));
    for i in 0..usize::from(glyf.num_glyphs) {
        let stored = if i < n {
            proportional_lsbs
        } else {
            monospace_lsbs
        };
        lsbs.push(if stored { r.i16()? } else { glyf.x_mins[i] });
    }
    let mut out = Vec::with_capacity(4 * n + 2 * (lsbs.len() - n));
    for (i, lsb) in lsbs.iter().enumerate() {
        if i < n {
            bytes::put_u16(&mut out, advances[i]);
        }
        bytes::put_i16(&mut out, *lsb);
    }
    Ok(out)
}

/// A Brotli stream (RFC 7932) holding `data` in uncompressed meta-blocks. It
/// lets tests and the mutation corpus build WOFF2 files without a compressor.
pub fn store_brotli(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter::default();
    w.bits(0, 1); // WBITS = 16
    for chunk in data.chunks(1 << 16) {
        w.bits(0, 1); // ISLAST
        w.bits(0, 2); // MNIBBLES = 4
        w.bits(chunk.len() as u64 - 1, 16); // MLEN - 1
        w.bits(1, 1); // ISUNCOMPRESSED
        w.align();
        w.bytes(chunk);
    }
    w.bits(1, 1); // ISLAST
    w.bits(1, 1); // ISLASTEMPTY
    w.align();
    w.out
}

#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    current: u8,
    used: u32,
}

impl BitWriter {
    /// Append the low `count` bits of `value`, least significant first.
    fn bits(&mut self, value: u64, count: u32) {
        for i in 0..count {
            let bit = u8::from((value >> i) & 1 == 1);
            self.current |= bit << self.used;
            self.used += 1;
            if self.used == 8 {
                self.out.push(self.current);
                self.current = 0;
                self.used = 0;
            }
        }
    }

    fn align(&mut self) {
        if self.used > 0 {
            self.out.push(self.current);
            self.current = 0;
            self.used = 0;
        }
    }

    fn bytes(&mut self, data: &[u8]) {
        debug_assert_eq!(self.used, 0);
        self.out.extend_from_slice(data);
    }
}

/// Encode an sfnt as WOFF2 with no transforms (`glyf` and `loca` at version
/// 3) and a stored Brotli stream. For tests and the mutation corpus only.
pub fn encode_stored(font: &[u8]) -> Result<Vec<u8>> {
    let (flavor, mut tables) = sfnt::tables(font)?;
    tables.sort_by_key(|t| t.tag);
    let mut directory = Vec::new();
    let mut stream = Vec::new();
    let mut total_sfnt = 12 + 16 * tables.len();
    for table in &tables {
        let version = if table.tag == GLYF || table.tag == LOCA {
            3
        } else {
            0
        };
        if let Some(index) = KNOWN_TAGS.iter().position(|t| *t == table.tag) {
            directory.push((version << 6) | u8::try_from(index).unwrap_or(EXPLICIT_TAG));
        } else {
            directory.push((version << 6) | EXPLICIT_TAG);
            directory.extend_from_slice(&table.tag);
        }
        let len = u32::try_from(table.data.len()).map_err(|_| Error("table too large"))?;
        bytes::put_base128(&mut directory, len);
        stream.extend_from_slice(&table.data);
        total_sfnt += bytes::pad4(table.data.len()).ok_or(Error("too large"))?;
    }
    Ok(assemble(
        flavor,
        tables.len(),
        total_sfnt,
        &directory,
        &store_brotli(&stream),
    ))
}

/// A header, directories and compressed stream, with no metadata or private
/// block. The compressed block is padded to four bytes, as the reference
/// decoder requires.
fn assemble(
    flavor: u32,
    num_tables: usize,
    total_sfnt: usize,
    directories: &[u8],
    compressed: &[u8],
) -> Vec<u8> {
    let len = (HEADER_LEN + directories.len() + compressed.len()).next_multiple_of(4);
    let mut out = Vec::with_capacity(len);
    bytes::put_u32(&mut out, SIGNATURE);
    bytes::put_u32(&mut out, flavor);
    bytes::put_u32(&mut out, u32::try_from(len).unwrap_or(u32::MAX));
    bytes::put_u16(&mut out, u16::try_from(num_tables).unwrap_or(u16::MAX));
    bytes::put_u16(&mut out, 0);
    bytes::put_u32(&mut out, u32::try_from(total_sfnt).unwrap_or(u32::MAX));
    bytes::put_u32(
        &mut out,
        u32::try_from(compressed.len()).unwrap_or(u32::MAX),
    );
    bytes::put_u16(&mut out, 1);
    bytes::put_u16(&mut out, 0);
    out.extend_from_slice(&[0; 20]);
    out.extend_from_slice(directories);
    out.extend_from_slice(compressed);
    bytes::pad_to_4(&mut out);
    out
}

/// A WOFF2 file's decompressed table stream, and the bytes before its
/// compressed data (header and directories). The mutation corpus changes the
/// stream and rewraps it, so that mutations reach the transforms and the
/// parsers behind the Brotli layer.
pub fn split(data: &[u8], limits: &Limits) -> Result<(Directory, Vec<u8>)> {
    let directory = read_directory(data, limits)?;
    let stream = decompress(
        &data[directory.compressed.clone()],
        directory.uncompressed_len,
    )?;
    Ok((directory, stream))
}

/// Rebuild a WOFF2 file from an original's header and directories and a
/// (changed) table stream, stored uncompressed; any metadata and private block
/// are dropped.
pub fn rewrap(original: &[u8], directory: &Directory, stream: &[u8]) -> Vec<u8> {
    let compressed = store_brotli(stream);
    let mut out = original[..directory.compressed.start].to_vec();
    out.extend_from_slice(&compressed);
    bytes::pad_to_4(&mut out);
    let len = u32::try_from(out.len()).unwrap_or(u32::MAX);
    out[8..12].copy_from_slice(&len.to_be_bytes());
    let compressed_len = u32::try_from(compressed.len()).unwrap_or(u32::MAX);
    out[20..24].copy_from_slice(&compressed_len.to_be_bytes());
    out[28..48].fill(0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testfont;

    #[test]
    fn stored_brotli_decompresses() {
        for len in [0, 1, 1000, 65_536, 65_537, 200_000] {
            let data: Vec<u8> = (0..len)
                .map(|i| u8::try_from(i % 251).expect("small"))
                .collect();
            let stored = store_brotli(&data);
            if len > 0 {
                assert_eq!(
                    decompress(&stored, len).expect("decompresses"),
                    data,
                    "{len}"
                );
            }
        }
    }

    #[test]
    fn round_trips_a_font_with_null_transforms() {
        let font = testfont::square_font();
        let woff2 = encode_stored(&font).expect("encodes");
        let fonts = decode(&woff2, &Limits::default()).expect("decodes");
        assert_eq!(fonts.len(), 1);
        assert_eq!(
            sfnt::tables(&fonts[0]).expect("reads").1,
            sfnt::tables(&font).expect("reads").1
        );
    }

    #[test]
    fn decodes_triplets_like_the_reference() {
        // flag 1: dy = +b0; flag 10: dx = -b0; flag 20 + 0x10: dx = -(1 + 16
        // + hi), dy = -(1 + lo); flag 124 (two-byte deltas), x and y negative.
        let flags = [1, 10, 20 + 0x10, 0xFC]; // 0xFC: off-curve, flag 124
        let data = [5, 7, 0x23, 0x01, 0x00, 0x02, 0x00];
        let mut reader = Reader::new(&data);
        let mut points = Vec::new();
        decode_triplets(&flags, &mut reader, &mut points).expect("decodes");
        assert_eq!(
            points,
            vec![
                (0, 5, true),
                (-7, 5, true),
                (-7 - 19, 5 - 4, true),
                (-7 - 19 - 256, 5 - 4 - 512, false),
            ]
        );
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn rejects_streams_of_the_wrong_length() {
        let woff2 = encode_stored(&testfont::square_font()).expect("encodes");
        let limits = Limits::default();
        let (directory, mut stream) = split(&woff2, &limits).expect("splits");
        stream.push(0);
        let longer = rewrap(&woff2, &directory, &stream);
        assert!(decode(&longer, &limits).is_err());
        stream.truncate(stream.len() - 2);
        let shorter = rewrap(&woff2, &directory, &stream);
        assert!(decode(&shorter, &limits).is_err());
    }

    #[test]
    fn rejects_truncation_at_every_length() {
        let woff2 = encode_stored(&testfont::square_font()).expect("encodes");
        for len in 0..woff2.len() {
            assert!(decode(&woff2[..len], &Limits::default()).is_err(), "{len}");
        }
    }
}

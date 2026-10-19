//! WOFF 1.0 (W3C Recommendation, 2012-12-13), decoded in safe Rust (TX7).
//!
//! The decoder enforces the specification's conformance requirements for user
//! agents: the signature; `length`, `reserved` and `totalSfntSize`; table
//! directory entries in ascending tag order; tables stored immediately after
//! the directory, four-byte aligned, with no overlap, gap or extraneous data;
//! `compLength <= origLength`; zlib streams that decompress to exactly
//! `origLength`; and the metadata and private blocks in their places. The
//! metadata block's contents are never parsed: they "MUST NOT affect font
//! loading".

use miniz_oxide::deflate::compress_to_vec_zlib;
use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;

use crate::Limits;
use crate::bytes::{self, Error, Reader, Result};
use crate::sfnt::{self, Table};

/// The WOFF 1.0 signature, `wOFF`.
pub const SIGNATURE: u32 = 0x774F_4646;
const HEADER_LEN: usize = 44;
const ENTRY_LEN: usize = 20;

#[derive(Clone, Copy, Debug)]
struct Entry {
    tag: [u8; 4],
    offset: usize,
    comp_len: usize,
    orig_len: usize,
}

/// Decode a WOFF 1.0 file into an sfnt.
pub fn decode(data: &[u8], limits: &Limits) -> Result<Vec<u8>> {
    if data.len() > limits.max_file {
        return Err(Error("file larger than the limit"));
    }
    let mut r = Reader::new(data);
    if r.u32()? != SIGNATURE {
        return Err(Error("signature is not wOFF"));
    }
    let flavor = r.u32()?;
    if r.u32()? as usize != data.len() {
        return Err(Error("length does not match the file"));
    }
    let num_tables = usize::from(r.u16()?);
    if num_tables == 0 {
        return Err(Error("numTables is zero"));
    }
    if r.u16()? != 0 {
        return Err(Error("reserved field is not zero"));
    }
    let total_sfnt_size = r.u32()? as usize;
    r.skip(4)?; // majorVersion, minorVersion: informative.
    let meta_offset = r.u32()? as usize;
    let meta_len = r.u32()? as usize;
    r.skip(4)?; // metaOrigLength: the metadata is never decompressed.
    let priv_offset = r.u32()? as usize;
    let priv_len = r.u32()? as usize;

    let mut entries = Vec::with_capacity(num_tables);
    for _ in 0..num_tables {
        let tag = r.tag()?;
        let offset = r.u32()? as usize;
        let comp_len = r.u32()? as usize;
        let orig_len = r.u32()? as usize;
        let _orig_checksum = r.u32()?;
        if comp_len > orig_len {
            return Err(Error("compLength is greater than origLength"));
        }
        entries.push(Entry {
            tag,
            offset,
            comp_len,
            orig_len,
        });
    }
    if entries.windows(2).any(|w| w[0].tag >= w[1].tag) {
        return Err(Error("table directory not in ascending tag order"));
    }

    // totalSfntSize: "If this value is incorrect, a conforming user agent MUST
    // reject the file as invalid."
    let mut expected = 12 + 16 * num_tables;
    for entry in &entries {
        expected = bytes::pad4(entry.orig_len)
            .and_then(|len| expected.checked_add(len))
            .ok_or(Error("tables too large"))?;
    }
    if expected != total_sfnt_size {
        return Err(Error("totalSfntSize is incorrect"));
    }
    if total_sfnt_size > limits.max_sfnt {
        return Err(Error("decoded font larger than the limit"));
    }

    check_layout(
        data.len(),
        &entries,
        (meta_offset, meta_len),
        (priv_offset, priv_len),
    )?;

    let mut tables = Vec::with_capacity(num_tables);
    for entry in &entries {
        let stored = &data[entry.offset..entry.offset + entry.comp_len];
        let table = if entry.comp_len == entry.orig_len {
            stored.to_vec()
        } else {
            let inflated = decompress_to_vec_zlib_with_limit(stored, entry.orig_len)
                .map_err(|_| Error("a table's zlib stream failed to decompress"))?;
            if inflated.len() != entry.orig_len {
                return Err(Error(
                    "a table decompressed to a size other than origLength",
                ));
            }
            inflated
        };
        tables.push(Table {
            tag: entry.tag,
            data: table,
        });
    }
    sfnt::write(flavor, tables)
}

/// The blocks' layout: tables immediately after the directory, in some order,
/// each starting on a four-byte boundary right after the previous one; then
/// the metadata block, if any; then the private block, if any, ending at the
/// end of the file. Nothing overlaps and nothing else is present.
fn check_layout(
    file_len: usize,
    entries: &[Entry],
    (meta_offset, meta_len): (usize, usize),
    (priv_offset, priv_len): (usize, usize),
) -> Result<()> {
    let mut by_offset: Vec<&Entry> = entries.iter().collect();
    by_offset.sort_by_key(|e| e.offset);
    let mut next = HEADER_LEN + ENTRY_LEN * entries.len();
    let mut end = next;
    for entry in by_offset {
        if entry.offset != next {
            return Err(Error(
                "table data does not follow on without gaps or overlap",
            ));
        }
        end = entry
            .offset
            .checked_add(entry.comp_len)
            .filter(|&end| end <= file_len)
            .ok_or(Error("a table extends past the end of the file"))?;
        next = bytes::pad4(end).ok_or(Error("offset overflow"))?;
    }
    let mut last_end = end;
    if meta_offset != 0 || meta_len != 0 {
        if meta_offset != next {
            return Err(Error("metadata block misplaced"));
        }
        last_end = meta_offset
            .checked_add(meta_len)
            .filter(|&end| end <= file_len)
            .ok_or(Error("metadata block extends past the end of the file"))?;
        next = bytes::pad4(last_end).ok_or(Error("offset overflow"))?;
    }
    if priv_offset != 0 || priv_len != 0 {
        if priv_offset != next {
            return Err(Error("private block misplaced"));
        }
        let priv_end = priv_offset
            .checked_add(priv_len)
            .ok_or(Error("offset overflow"))?;
        if priv_end != file_len {
            return Err(Error("private block does not end the file"));
        }
        return Ok(());
    }
    // Up to three bytes of padding may follow the last block.
    if file_len < last_end || file_len > next {
        return Err(Error("extraneous data after the last block"));
    }
    Ok(())
}

/// Encode an sfnt as WOFF 1.0, compressing each table with zlib when that
/// makes it smaller (or storing every table, with `compress` false). For tests
/// and the mutation corpus only.
pub fn encode(font: &[u8], compress: bool) -> Result<Vec<u8>> {
    let (flavor, mut tables) = sfnt::tables(font)?;
    tables.sort_by_key(|t| t.tag);
    let stored: Vec<Vec<u8>> = tables
        .iter()
        .map(|t| {
            let packed = if compress {
                compress_to_vec_zlib(&t.data, 6)
            } else {
                Vec::new()
            };
            if compress && packed.len() < t.data.len() {
                packed
            } else {
                t.data.clone()
            }
        })
        .collect();
    let mut total_sfnt = 12 + 16 * tables.len();
    for t in &tables {
        total_sfnt += bytes::pad4(t.data.len()).ok_or(Error("too large"))?;
    }
    let mut offset = HEADER_LEN + ENTRY_LEN * tables.len();
    let mut directory = Vec::new();
    let mut body = Vec::new();
    for (table, data) in tables.iter().zip(&stored) {
        directory.extend_from_slice(&table.tag);
        bytes::put_u32(&mut directory, to_u32(offset)?);
        bytes::put_u32(&mut directory, to_u32(data.len())?);
        bytes::put_u32(&mut directory, to_u32(table.data.len())?);
        bytes::put_u32(&mut directory, bytes::checksum(&table.data));
        body.extend_from_slice(data);
        bytes::pad_to_4(&mut body);
        offset = HEADER_LEN + ENTRY_LEN * tables.len() + body.len();
    }
    let len = HEADER_LEN + directory.len() + body.len();
    let mut out = Vec::with_capacity(len);
    bytes::put_u32(&mut out, SIGNATURE);
    bytes::put_u32(&mut out, flavor);
    bytes::put_u32(&mut out, to_u32(len)?);
    bytes::put_u16(
        &mut out,
        u16::try_from(tables.len()).map_err(|_| Error("too many tables"))?,
    );
    bytes::put_u16(&mut out, 0);
    bytes::put_u32(&mut out, to_u32(total_sfnt)?);
    bytes::put_u16(&mut out, 1);
    bytes::put_u16(&mut out, 0);
    out.extend_from_slice(&[0; 20]); // No metadata or private block.
    out.extend_from_slice(&directory);
    out.extend_from_slice(&body);
    Ok(out)
}

fn to_u32(n: usize) -> Result<u32> {
    u32::try_from(n).map_err(|_| Error("larger than 4 GiB"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testfont;

    #[test]
    fn round_trips_compressed_and_stored() {
        let font = testfont::square_font();
        for compress in [true, false] {
            let woff = encode(&font, compress).expect("encodes");
            let back = decode(&woff, &Limits::default()).expect("decodes");
            assert_eq!(
                sfnt::tables(&back).expect("reads").1,
                sfnt::tables(&font).expect("reads").1,
                "compress = {compress}"
            );
        }
    }

    #[test]
    fn rejects_what_the_specification_rejects() {
        let woff = encode(&testfont::square_font(), true).expect("encodes");
        let limits = Limits::default();
        let mut bad = woff.clone();
        bad[0] = b'X';
        assert_eq!(decode(&bad, &limits), Err(Error("signature is not wOFF")));
        let mut bad = woff.clone();
        bad[14] = 1;
        assert_eq!(
            decode(&bad, &limits),
            Err(Error("reserved field is not zero"))
        );
        let mut bad = woff.clone();
        bad.extend_from_slice(&[0; 4]);
        let len = u32::try_from(bad.len()).expect("small");
        bad[8..12].copy_from_slice(&len.to_be_bytes());
        assert_eq!(
            decode(&bad, &limits),
            Err(Error("extraneous data after the last block"))
        );
        let mut bad = woff;
        bad[16..20].copy_from_slice(&[0, 0, 0, 4]);
        assert_eq!(
            decode(&bad, &limits),
            Err(Error("totalSfntSize is incorrect"))
        );
    }

    #[test]
    fn rejects_truncation_at_every_length() {
        let woff = encode(&testfont::square_font(), true).expect("encodes");
        for len in 0..woff.len() {
            assert!(decode(&woff[..len], &Limits::default()).is_err(), "{len}");
        }
    }
}

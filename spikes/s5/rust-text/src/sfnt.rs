//! Writing and reading sfnt (OpenType) table directories: the output of both
//! container decoders, and the input of the test encoders.

use crate::bytes::{self, Error, Reader, Result};

/// `sfntVersion` of a font with TrueType outlines.
pub const TRUETYPE: u32 = 0x0001_0000;
/// `sfntVersion` of a font with CFF outlines (`OTTO`).
pub const CFF: u32 = u32::from_be_bytes(*b"OTTO");
/// The tag of a font collection's header (`ttcf`).
pub const COLLECTION: u32 = u32::from_be_bytes(*b"ttcf");

const OFFSET_TABLE_LEN: usize = 12;
const TABLE_RECORD_LEN: usize = 16;
/// `head.checkSumAdjustment`'s offset within the table.
const CHECKSUM_ADJUSTMENT: usize = 8;

/// One font table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub tag: [u8; 4],
    pub data: Vec<u8>,
}

/// Write an sfnt: the offset table, and table records sorted by tag; each table
/// four-byte aligned and zero-padded, with its checksum; and `head`'s
/// `checkSumAdjustment` set (OpenType, "Font file" chapter).
///
/// Fails on duplicate tags, or a font too large for 32-bit offsets.
pub fn write(flavor: u32, mut tables: Vec<Table>) -> Result<Vec<u8>> {
    tables.sort_by_key(|t| t.tag);
    if tables.windows(2).any(|w| w[0].tag == w[1].tag) {
        return Err(Error("two tables with the same tag"));
    }
    let count = u16::try_from(tables.len()).map_err(|_| Error("too many tables"))?;
    // head's checkSumAdjustment is summed as zero, then fixed below.
    for table in &mut tables {
        if &table.tag == b"head" && table.data.len() >= CHECKSUM_ADJUSTMENT + 4 {
            table.data[CHECKSUM_ADJUSTMENT..CHECKSUM_ADJUSTMENT + 4].fill(0);
        }
    }

    let mut out = Vec::new();
    bytes::put_u32(&mut out, flavor);
    bytes::put_u16(&mut out, count);
    let entry_selector = if count == 0 { 0 } else { count.ilog2() };
    // At most 2^15 tables have tag-sorted records here, so these fit in u16.
    let search_range = (1_u32 << entry_selector) * 16;
    let range_shift = u32::from(count) * 16 - search_range.min(u32::from(count) * 16);
    bytes::put_u16(&mut out, u16::try_from(search_range).unwrap_or(u16::MAX));
    bytes::put_u16(&mut out, u16::try_from(entry_selector).unwrap_or(u16::MAX));
    bytes::put_u16(&mut out, u16::try_from(range_shift).unwrap_or(u16::MAX));

    let mut offset = OFFSET_TABLE_LEN + TABLE_RECORD_LEN * tables.len();
    let mut head_offset = None;
    for table in &tables {
        let offset32 = u32::try_from(offset).map_err(|_| Error("font larger than 4 GiB"))?;
        let len32 =
            u32::try_from(table.data.len()).map_err(|_| Error("table larger than 4 GiB"))?;
        out.extend_from_slice(&table.tag);
        bytes::put_u32(&mut out, bytes::checksum(&table.data));
        bytes::put_u32(&mut out, offset32);
        bytes::put_u32(&mut out, len32);
        if &table.tag == b"head" {
            head_offset = Some(offset);
        }
        offset = bytes::pad4(offset + table.data.len()).ok_or(Error("font larger than 4 GiB"))?;
    }
    for table in &tables {
        out.extend_from_slice(&table.data);
        bytes::pad_to_4(&mut out);
    }
    if let Some(head) = head_offset
        && tables
            .iter()
            .any(|t| &t.tag == b"head" && t.data.len() >= CHECKSUM_ADJUSTMENT + 4)
    {
        let adjustment = 0xB1B0_AFBA_u32.wrapping_sub(bytes::checksum(&out));
        let at = head + CHECKSUM_ADJUSTMENT;
        out[at..at + 4].copy_from_slice(&adjustment.to_be_bytes());
    }
    Ok(out)
}

/// A table record read from an sfnt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub tag: [u8; 4],
    pub offset: usize,
    pub len: usize,
}

/// Read an sfnt's flavor and table records, checking each table lies inside
/// the data. (The test encoders' input; the decoders never call this.)
pub fn read(data: &[u8]) -> Result<(u32, Vec<Record>)> {
    let mut r = Reader::new(data);
    let flavor = r.u32()?;
    let count = r.u16()?;
    r.skip(6)?;
    let mut records = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let tag = r.tag()?;
        let _checksum = r.u32()?;
        let offset = r.u32()? as usize;
        let len = r.u32()? as usize;
        if offset.checked_add(len).is_none_or(|end| end > data.len()) {
            return Err(Error("a table extends past the end of the font"));
        }
        records.push(Record { tag, offset, len });
    }
    Ok((flavor, records))
}

/// The tables of an sfnt, copied out.
pub fn tables(data: &[u8]) -> Result<(u32, Vec<Table>)> {
    let (flavor, records) = read(data)?;
    let tables = records
        .iter()
        .map(|r| Table {
            tag: r.tag,
            data: data[r.offset..r.offset + r.len].to_vec(),
        })
        .collect();
    Ok((flavor, tables))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head() -> Vec<u8> {
        let mut head = vec![0_u8; 54];
        head[12..16].copy_from_slice(&0x5F0F_3CF5_u32.to_be_bytes());
        head
    }

    #[test]
    fn writes_sorted_padded_tables_whose_checksum_adjustment_balances() {
        let tables = vec![
            Table {
                tag: *b"name",
                data: vec![1, 2, 3],
            },
            Table {
                tag: *b"head",
                data: head(),
            },
        ];
        let font = write(TRUETYPE, tables).expect("writes");
        let (flavor, records) = read(&font).expect("reads back");
        assert_eq!(flavor, TRUETYPE);
        assert_eq!(records[0].tag, *b"head");
        assert_eq!(records[1].tag, *b"name");
        assert!(records.iter().all(|r| r.offset % 4 == 0));
        assert_eq!(font.len() % 4, 0);
        // OpenType: the whole font sums to 0xB1B0AFBA once the adjustment is in.
        assert_eq!(bytes::checksum(&font), 0xB1B0_AFBA);
    }

    #[test]
    fn rejects_duplicate_tags() {
        let table = Table {
            tag: *b"cmap",
            data: vec![0; 4],
        };
        assert!(write(TRUETYPE, vec![table.clone(), table]).is_err());
    }
}

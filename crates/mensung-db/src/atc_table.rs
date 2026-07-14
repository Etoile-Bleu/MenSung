//! Reads ATC Code Table records: fixed 12-byte entries, contiguous per
//! drug, addressed by that drug's `atc_start_index`/`atc_count` in the
//! Drug Table, as defined in docs/DATABASE_FORMAT.md.

use crate::bytes::{read_u16, read_u32};
use crate::DbError;

pub(crate) const RECORD_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtcCodeRecord<'a> {
    code: &'a str,
    class_name: &'a str,
}

impl<'a> AtcCodeRecord<'a> {
    pub fn code(&self) -> &'a str {
        self.code
    }

    pub fn class_name(&self) -> &'a str {
        self.class_name
    }
}

pub(crate) fn parse_record<'a>(
    table: &'a [u8],
    strings: &'a [u8],
    index: u32,
) -> Result<AtcCodeRecord<'a>, DbError> {
    let start = index as usize * RECORD_LEN;
    let record = table
        .get(start..start + RECORD_LEN)
        .ok_or(DbError::Truncated)?;

    let code_bytes = record.get(0..5).ok_or(DbError::Truncated)?;
    let code = std::str::from_utf8(code_bytes).map_err(|_| DbError::InvalidStringTableEntry)?;

    let class_name_offset = read_u32(record, 6)? as usize;
    let class_name_len = read_u16(record, 10)? as usize;
    let class_name_bytes = strings
        .get(class_name_offset..class_name_offset + class_name_len)
        .ok_or(DbError::Truncated)?;
    let class_name =
        std::str::from_utf8(class_name_bytes).map_err(|_| DbError::InvalidStringTableEntry)?;

    Ok(AtcCodeRecord { code, class_name })
}

/// Iterates the `count` ATC Code Table entries starting at `start_index`,
/// zero-copy: each entry is parsed only when `next()` reaches it.
#[derive(Debug, Clone, Copy)]
pub struct AtcCodeIter<'a> {
    pub(crate) table: &'a [u8],
    pub(crate) strings: &'a [u8],
    pub(crate) next_index: u32,
    pub(crate) remaining: u16,
}

impl<'a> Iterator for AtcCodeIter<'a> {
    type Item = Result<AtcCodeRecord<'a>, DbError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let record = parse_record(self.table, self.strings, self.next_index);
        self.next_index += 1;
        self.remaining -= 1;
        Some(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_of(entries: &[(&str, &str)]) -> (Vec<u8>, Vec<u8>) {
        let mut strings = Vec::new();
        let mut table = Vec::new();
        for (code, class_name) in entries {
            let offset = strings.len() as u32;
            strings.extend_from_slice(class_name.as_bytes());
            table.extend_from_slice(code.as_bytes());
            table.push(0);
            table.extend_from_slice(&offset.to_le_bytes());
            table.extend_from_slice(&(class_name.len() as u16).to_le_bytes());
        }
        (table, strings)
    }

    #[test]
    fn parses_a_single_entry() {
        let (table, strings) = table_of(&[("B01AA", "Vitamin K antagonists")]);
        let record = parse_record(&table, &strings, 0).unwrap();
        assert_eq!(record.code(), "B01AA");
        assert_eq!(record.class_name(), "Vitamin K antagonists");
    }

    #[test]
    fn iterator_yields_every_entry_in_order() {
        let (table, strings) = table_of(&[
            ("A01AD", "Other agents for local oral treatment"),
            ("B01AC", "Platelet aggregation inhibitors excl. heparin"),
        ]);
        let iter = AtcCodeIter {
            table: &table,
            strings: &strings,
            next_index: 0,
            remaining: 2,
        };
        let codes: Vec<&str> = iter.map(|r| r.unwrap().code()).collect();
        assert_eq!(codes, vec!["A01AD", "B01AC"]);
    }

    #[test]
    fn rejects_an_entry_pointing_outside_the_string_table() {
        let mut table = Vec::new();
        table.extend_from_slice(b"B01AA");
        table.push(0);
        table.extend_from_slice(&0u32.to_le_bytes());
        table.extend_from_slice(&100u16.to_le_bytes());
        assert_eq!(
            parse_record(&table, &[], 0).unwrap_err(),
            DbError::Truncated
        );
    }
}

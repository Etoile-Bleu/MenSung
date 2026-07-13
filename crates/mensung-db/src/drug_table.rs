//! Reads Drug Table records: fixed 12-byte entries sorted by INN name bytes
//! for binary search, as defined in docs/DATABASE_FORMAT.md.

use mensung_domain::DrugId;

use crate::bytes::{read_u16, read_u32};
use crate::DbError;

pub(crate) const RECORD_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrugRecord<'a> {
    id: DrugId,
    name: &'a str,
}

impl<'a> DrugRecord<'a> {
    pub fn id(&self) -> DrugId {
        self.id
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}

pub(crate) fn parse_record<'a>(
    table: &[u8],
    strings: &'a [u8],
    index: u32,
) -> Result<DrugRecord<'a>, DbError> {
    let start = index as usize * RECORD_LEN;
    let record = table
        .get(start..start + RECORD_LEN)
        .ok_or(DbError::Truncated)?;

    let drug_id = read_u32(record, 0)?;
    let name_offset = read_u32(record, 4)? as usize;
    let name_len = read_u16(record, 8)? as usize;

    let name_bytes = strings
        .get(name_offset..name_offset + name_len)
        .ok_or(DbError::Truncated)?;
    let name = std::str::from_utf8(name_bytes).map_err(|_| DbError::InvalidDrugName { index })?;

    Ok(DrugRecord {
        id: DrugId::new(drug_id),
        name,
    })
}

pub(crate) fn find_by_name<'a>(
    table: &[u8],
    strings: &'a [u8],
    count: u32,
    target: &str,
) -> Result<Option<DrugRecord<'a>>, DbError> {
    let mut low = 0u32;
    let mut high = count;

    while low < high {
        let mid = low + (high - low) / 2;
        let record = parse_record(table, strings, mid)?;
        match record.name().cmp(target) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Ok(Some(record)),
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_of(names: &[&str]) -> (Vec<u8>, Vec<u8>) {
        let mut strings = Vec::new();
        let mut table = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let offset = strings.len() as u32;
            strings.extend_from_slice(name.as_bytes());
            table.extend_from_slice(&(i as u32).to_le_bytes());
            table.extend_from_slice(&offset.to_le_bytes());
            table.extend_from_slice(&(name.len() as u16).to_le_bytes());
            table.extend_from_slice(&0u16.to_le_bytes());
        }
        (table, strings)
    }

    #[test]
    fn parses_a_single_record() {
        let (table, strings) = table_of(&["Aspirin"]);
        let record = parse_record(&table, &strings, 0).unwrap();
        assert_eq!(record.id(), DrugId::new(0));
        assert_eq!(record.name(), "Aspirin");
    }

    #[test]
    fn binary_search_finds_every_entry_in_a_sorted_table() {
        let names = ["Amoxicillin", "Aspirin", "Paracetamol", "Warfarin"];
        let (table, strings) = table_of(&names);
        for (i, name) in names.iter().enumerate() {
            let found = find_by_name(&table, &strings, names.len() as u32, name)
                .unwrap()
                .unwrap();
            assert_eq!(found.id(), DrugId::new(i as u32));
        }
    }

    #[test]
    fn binary_search_returns_none_for_a_name_not_present() {
        let names = ["Amoxicillin", "Aspirin", "Warfarin"];
        let (table, strings) = table_of(&names);
        assert_eq!(
            find_by_name(&table, &strings, names.len() as u32, "Ibuprofen").unwrap(),
            None
        );
    }

    #[test]
    fn rejects_a_record_pointing_outside_the_string_table() {
        let mut table = Vec::new();
        table.extend_from_slice(&0u32.to_le_bytes());
        table.extend_from_slice(&0u32.to_le_bytes());
        table.extend_from_slice(&100u16.to_le_bytes()); // name_len far past an empty string table
        table.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            parse_record(&table, &[], 0).unwrap_err(),
            DbError::Truncated
        );
    }
}

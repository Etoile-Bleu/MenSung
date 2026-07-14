//! Reads Drug Fact Index records: fixed 16-byte entries sorted by the
//! tuple `(drug_id, kind)` for binary search, as defined in
//! docs/DATABASE_FORMAT.md. Unlike the Interaction Index, a lookup here
//! can match more than one entry: a drug can have several `DrugFact`s
//! (a contraindication and a boxed warning are different facts about the
//! same drug), so `find_by_drug` returns every contiguous entry for a
//! `drug_id` once the first one is found by binary search.

use mensung_domain::DrugId;

use crate::bytes::read_u32;
use crate::DbError;

pub(crate) const RECORD_LEN: usize = 16;

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexEntry {
    pub(crate) record_offset: u32,
    pub(crate) record_len: u32,
}

fn parse_entry(table: &[u8], index: u32) -> Result<(u32, IndexEntry), DbError> {
    let start = index as usize * RECORD_LEN;
    let entry = table
        .get(start..start + RECORD_LEN)
        .ok_or(DbError::Truncated)?;

    Ok((
        read_u32(entry, 0)?,
        IndexEntry {
            record_offset: read_u32(entry, 8)?,
            record_len: read_u32(entry, 12)?,
        },
    ))
}

/// Binary search for the first entry whose `drug_id` matches, then scan
/// forward while it still does, since the index is sorted by
/// `(drug_id, kind)` and every entry for a drug is therefore contiguous.
pub(crate) fn find_by_drug(
    table: &[u8],
    count: u32,
    drug_id: DrugId,
) -> Result<Vec<IndexEntry>, DbError> {
    let target = drug_id.value();
    let mut low = 0u32;
    let mut high = count;

    while low < high {
        let mid = low + (high - low) / 2;
        let (entry_drug_id, _) = parse_entry(table, mid)?;
        if entry_drug_id < target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    let mut entries = Vec::new();
    let mut index = low;
    while index < count {
        let (entry_drug_id, entry) = parse_entry(table, index)?;
        if entry_drug_id != target {
            break;
        }
        entries.push(entry);
        index += 1;
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(entries: &[(u32, u8)]) -> Vec<u8> {
        let mut table = Vec::new();
        for (i, (drug_id, kind)) in entries.iter().enumerate() {
            let record_offset = (i * 100) as u32;
            let record_len = 40u32;
            table.extend_from_slice(&drug_id.to_le_bytes());
            table.push(*kind);
            table.extend_from_slice(&[0u8; 3]);
            table.extend_from_slice(&record_offset.to_le_bytes());
            table.extend_from_slice(&record_len.to_le_bytes());
        }
        table
    }

    #[test]
    fn finds_all_entries_for_a_drug_with_multiple_facts() {
        let entries = [(0, 0), (1, 0), (1, 1), (1, 2), (2, 0)];
        let table = index_of(&entries);
        let found = find_by_drug(&table, entries.len() as u32, DrugId::new(1)).unwrap();
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn finds_a_single_entry_for_a_drug_with_one_fact() {
        let entries = [(0, 0), (1, 0), (2, 0)];
        let table = index_of(&entries);
        let found = find_by_drug(&table, entries.len() as u32, DrugId::new(0)).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn returns_empty_for_a_drug_with_no_facts() {
        let entries = [(0, 0), (2, 0)];
        let table = index_of(&entries);
        let found = find_by_drug(&table, entries.len() as u32, DrugId::new(1)).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn returns_empty_for_an_empty_index() {
        let found = find_by_drug(&[], 0, DrugId::new(0)).unwrap();
        assert!(found.is_empty());
    }
}

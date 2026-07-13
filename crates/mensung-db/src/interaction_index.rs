//! Reads Interaction Index records: fixed 16-byte entries sorted by the
//! canonical `(drug_id_lower, drug_id_higher)` pair key for binary search,
//! as defined in docs/DATABASE_FORMAT.md.

use mensung_domain::DrugPair;

use crate::bytes::read_u32;
use crate::DbError;

pub(crate) const RECORD_LEN: usize = 16;

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexEntry {
    pub(crate) record_offset: u32,
    pub(crate) record_len: u32,
}

fn parse_entry(table: &[u8], index: u32) -> Result<(u32, u32, IndexEntry), DbError> {
    let start = index as usize * RECORD_LEN;
    let entry = table
        .get(start..start + RECORD_LEN)
        .ok_or(DbError::Truncated)?;

    Ok((
        read_u32(entry, 0)?,
        read_u32(entry, 4)?,
        IndexEntry {
            record_offset: read_u32(entry, 8)?,
            record_len: read_u32(entry, 12)?,
        },
    ))
}

fn pair_key(lower: u32, higher: u32) -> u64 {
    (u64::from(lower) << 32) | u64::from(higher)
}

pub(crate) fn find_pair(
    table: &[u8],
    count: u32,
    pair: DrugPair,
) -> Result<Option<(u32, IndexEntry)>, DbError> {
    let (lower, higher) = pair.drugs();
    let target_key = pair_key(lower.value(), higher.value());

    let mut low = 0u32;
    let mut high = count;

    while low < high {
        let mid = low + (high - low) / 2;
        let (entry_lower, entry_higher, entry) = parse_entry(table, mid)?;
        let key = pair_key(entry_lower, entry_higher);
        match key.cmp(&target_key) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Ok(Some((mid, entry))),
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::DrugId;

    fn index_of(pairs: &[(u32, u32)]) -> Vec<u8> {
        let mut table = Vec::new();
        for (i, (lower, higher)) in pairs.iter().enumerate() {
            let record_offset = (i * 100) as u32;
            let record_len = 10u32;
            table.extend_from_slice(&lower.to_le_bytes());
            table.extend_from_slice(&higher.to_le_bytes());
            table.extend_from_slice(&record_offset.to_le_bytes());
            table.extend_from_slice(&record_len.to_le_bytes());
        }
        table
    }

    #[test]
    fn binary_search_finds_every_pair() {
        let pairs = [(1, 2), (1, 5), (3, 4), (10, 20)];
        let table = index_of(&pairs);
        for (lower, higher) in pairs {
            let pair = DrugPair::new(DrugId::new(lower), DrugId::new(higher)).unwrap();
            let (_, entry) = find_pair(&table, pairs.len() as u32, pair)
                .unwrap()
                .unwrap();
            assert_eq!(entry.record_len, 10);
        }
    }

    #[test]
    fn returns_none_for_a_pair_not_present() {
        let pairs = [(1, 2), (3, 4)];
        let table = index_of(&pairs);
        let pair = DrugPair::new(DrugId::new(1), DrugId::new(99)).unwrap();
        assert!(find_pair(&table, pairs.len() as u32, pair)
            .unwrap()
            .is_none());
    }

    #[test]
    fn lookup_is_order_independent_because_drugpair_canonicalizes() {
        let pairs = [(1, 2)];
        let table = index_of(&pairs);
        let a = DrugPair::new(DrugId::new(2), DrugId::new(1)).unwrap();
        assert!(find_pair(&table, 1, a).unwrap().is_some());
    }
}

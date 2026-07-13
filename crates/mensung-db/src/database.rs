//! The zero-copy .men database reader. `Database::open` validates the file
//! once (magic, version, header CRC32, payload SHA-256, section bounds);
//! every lookup after that reads directly from the underlying byte buffer
//! with no further allocation, per docs/DATABASE_FORMAT.md.

use mensung_domain::DrugPair;
use sha2::{Digest, Sha256};

use crate::drug_table::{self, DrugRecord};
use crate::header::Header;
use crate::interaction_index;
use crate::interaction_record::{self, InteractionRecord};
use crate::DbError;

#[derive(Debug)]
pub struct Database<'a> {
    bytes: &'a [u8],
    header: Header,
}

impl<'a> Database<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self, DbError> {
        let header = Header::parse(bytes)?;
        verify_payload_checksum(bytes, &header)?;
        Ok(Self { bytes, header })
    }

    pub fn drug_count(&self) -> u32 {
        self.header.drug_count
    }

    pub fn interaction_count(&self) -> u32 {
        self.header.interaction_count
    }

    pub fn find_drug_by_name(&self, name: &str) -> Result<Option<DrugRecord<'a>>, DbError> {
        drug_table::find_by_name(
            self.drug_table(),
            self.string_table(),
            self.header.drug_count,
            name,
        )
    }

    pub fn drugs(&self) -> DrugIter<'a> {
        DrugIter {
            table: self.drug_table(),
            strings: self.string_table(),
            count: self.header.drug_count,
            next: 0,
        }
    }

    pub fn find_interaction(
        &self,
        pair: DrugPair,
    ) -> Result<Option<InteractionRecord<'a>>, DbError> {
        let found = interaction_index::find_pair(
            self.interaction_index(),
            self.header.interaction_count,
            pair,
        )?;
        let Some((index, entry)) = found else {
            return Ok(None);
        };
        let record = interaction_record::parse(
            self.interaction_records(),
            entry.record_offset,
            entry.record_len,
            index,
        )?;
        Ok(Some(record))
    }

    fn string_table(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.string_table_offset,
            self.header.string_table_len,
        )
    }

    fn drug_table(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.drug_table_offset,
            self.header.drug_table_len,
        )
    }

    fn interaction_index(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.interaction_index_offset,
            self.header.interaction_index_len,
        )
    }

    fn interaction_records(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.interaction_records_offset,
            self.header.interaction_records_len,
        )
    }
}

pub struct DrugIter<'a> {
    table: &'a [u8],
    strings: &'a [u8],
    count: u32,
    next: u32,
}

impl<'a> Iterator for DrugIter<'a> {
    type Item = Result<DrugRecord<'a>, DbError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.count {
            return None;
        }
        let record = drug_table::parse_record(self.table, self.strings, self.next);
        self.next += 1;
        Some(record)
    }
}

fn slice_at(bytes: &[u8], offset: u64, len: u64) -> &[u8] {
    &bytes[offset as usize..(offset + len) as usize]
}

fn verify_payload_checksum(bytes: &[u8], header: &Header) -> Result<(), DbError> {
    let payload = bytes.get(header.header_len..).ok_or(DbError::Truncated)?;
    let computed: [u8; 32] = Sha256::digest(payload).into();
    if computed != header.payload_sha256 {
        return Err(DbError::PayloadChecksumMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::HEADER_LEN;
    use mensung_domain::{DrugId, Severity};

    fn build_test_database() -> Vec<u8> {
        let names = [("Aspirin", 0u32), ("Warfarin", 1u32)];

        let mut string_table = Vec::new();
        let mut drug_table = Vec::new();
        for (name, id) in &names {
            let offset = string_table.len() as u32;
            string_table.extend_from_slice(name.as_bytes());
            drug_table.extend_from_slice(&id.to_le_bytes());
            drug_table.extend_from_slice(&offset.to_le_bytes());
            drug_table.extend_from_slice(&(name.len() as u16).to_le_bytes());
            drug_table.extend_from_slice(&0u16.to_le_bytes());
        }

        let description = "Increased bleeding and hemorrhage probability.";
        let source = "WHO drug interaction reference";
        let mut interaction_record = Vec::new();
        interaction_record.extend_from_slice(&0u32.to_le_bytes());
        interaction_record.extend_from_slice(&0u32.to_le_bytes());
        interaction_record.extend_from_slice(&1u32.to_le_bytes());
        interaction_record.push(0);
        interaction_record.push(0);
        interaction_record.extend_from_slice(&0u16.to_le_bytes());
        interaction_record.extend_from_slice(&(description.len() as u32).to_le_bytes());
        interaction_record.extend_from_slice(description.as_bytes());
        interaction_record.extend_from_slice(&(source.len() as u32).to_le_bytes());
        interaction_record.extend_from_slice(source.as_bytes());

        let mut interaction_index = Vec::new();
        interaction_index.extend_from_slice(&0u32.to_le_bytes());
        interaction_index.extend_from_slice(&1u32.to_le_bytes());
        interaction_index.extend_from_slice(&0u32.to_le_bytes());
        interaction_index.extend_from_slice(&(interaction_record.len() as u32).to_le_bytes());

        let string_table_offset = HEADER_LEN as u64;
        let string_table_len = string_table.len() as u64;
        let drug_table_offset = string_table_offset + string_table_len;
        let drug_table_len = drug_table.len() as u64;
        let interaction_index_offset = drug_table_offset + drug_table_len;
        let interaction_index_len = interaction_index.len() as u64;
        let interaction_records_offset = interaction_index_offset + interaction_index_len;
        let interaction_records_len = interaction_record.len() as u64;

        let mut payload = Vec::new();
        payload.extend_from_slice(&string_table);
        payload.extend_from_slice(&drug_table);
        payload.extend_from_slice(&interaction_index);
        payload.extend_from_slice(&interaction_record);

        let payload_sha256: [u8; 32] = Sha256::digest(&payload).into();

        let mut header = vec![0u8; HEADER_LEN];
        header[0..4].copy_from_slice(b"MEN1");
        header[4..6].copy_from_slice(&1u16.to_le_bytes());
        header[6..8].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
        header[12..20].copy_from_slice(&0u64.to_le_bytes());
        header[20..52].copy_from_slice(&payload_sha256);
        header[52..56].copy_from_slice(&2u32.to_le_bytes());
        header[56..60].copy_from_slice(&1u32.to_le_bytes());
        header[60..68].copy_from_slice(&string_table_offset.to_le_bytes());
        header[68..76].copy_from_slice(&string_table_len.to_le_bytes());
        header[76..84].copy_from_slice(&drug_table_offset.to_le_bytes());
        header[84..92].copy_from_slice(&drug_table_len.to_le_bytes());
        header[92..100].copy_from_slice(&interaction_index_offset.to_le_bytes());
        header[100..108].copy_from_slice(&interaction_index_len.to_le_bytes());
        header[108..116].copy_from_slice(&interaction_records_offset.to_le_bytes());
        header[116..124].copy_from_slice(&interaction_records_len.to_le_bytes());
        header[124..128].copy_from_slice(&[0u8; 4]);

        let crc = {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(&header[0..8]);
            hasher.update(&header[12..]);
            hasher.finalize()
        };
        header[8..12].copy_from_slice(&crc.to_le_bytes());

        let mut full = header;
        full.extend_from_slice(&payload);
        full
    }

    #[test]
    fn opens_a_well_formed_database() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        assert_eq!(db.drug_count(), 2);
        assert_eq!(db.interaction_count(), 1);
    }

    #[test]
    fn finds_a_drug_by_exact_name() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let drug = db.find_drug_by_name("Aspirin").unwrap().unwrap();
        assert_eq!(drug.id(), DrugId::new(0));
    }

    #[test]
    fn returns_none_for_an_unknown_drug_name() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        assert!(db.find_drug_by_name("Ibuprofen").unwrap().is_none());
    }

    #[test]
    fn finds_the_known_interaction_regardless_of_argument_order() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();

        let pair = DrugPair::new(DrugId::new(1), DrugId::new(0)).unwrap();
        let interaction = db.find_interaction(pair).unwrap().unwrap();
        assert_eq!(interaction.severity(), Severity::Contraindicated);
        assert_eq!(
            interaction.description(),
            "Increased bleeding and hemorrhage probability."
        );
    }

    #[test]
    fn returns_none_for_a_pair_with_no_known_interaction() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let pair = DrugPair::new(DrugId::new(0), DrugId::new(99)).unwrap();
        assert!(db.find_interaction(pair).unwrap().is_none());
    }

    #[test]
    fn iterates_every_drug_in_table_order() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let names: Vec<&str> = db.drugs().map(|r| r.unwrap().name()).collect();
        assert_eq!(names, vec!["Aspirin", "Warfarin"]);
    }

    #[test]
    fn rejects_a_database_with_a_corrupted_payload() {
        let mut bytes = build_test_database();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        assert_eq!(
            Database::open(&bytes).unwrap_err(),
            DbError::PayloadChecksumMismatch
        );
    }

    #[test]
    fn never_panics_on_any_single_bit_flip_of_a_valid_file() {
        let original = build_test_database();
        for byte_index in 0..original.len() {
            for bit in 0..8u8 {
                let mut mutated = original.clone();
                mutated[byte_index] ^= 1 << bit;
                let opened = Database::open(&mutated);
                if let Ok(db) = opened {
                    for drug in db.drugs() {
                        let _ = drug;
                    }
                    let pair = DrugPair::new(DrugId::new(0), DrugId::new(1)).unwrap();
                    let _ = db.find_interaction(pair);
                }
            }
        }
    }

    #[test]
    fn never_panics_on_a_truncated_file_at_any_length() {
        let original = build_test_database();
        for len in 0..original.len() {
            let _ = Database::open(&original[..len]);
        }
    }
}

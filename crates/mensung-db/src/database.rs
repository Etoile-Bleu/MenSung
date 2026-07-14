//! The .men database reader. `Database::open` validates the file once
//! (magic, version, header CRC32, payload SHA-256, section bounds); every
//! lookup after that reads directly from the underlying byte buffer with
//! no further allocation for the binary-search path, per
//! docs/DATABASE_FORMAT.md. Resolving a matched record's claims allocates
//! a small `Vec` (see `interaction_record.rs`'s header for why); this
//! only happens for the handful of records an actual lookup returns, not
//! while searching.

use mensung_domain::{DrugId, DrugPair};
use sha2::{Digest, Sha256};

use crate::drug_fact::{self, DrugFactRecord};
use crate::drug_fact_index;
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

    pub fn drug_fact_count(&self) -> u32 {
        self.header.drug_fact_count
    }

    pub fn atc_code_count(&self) -> u32 {
        self.header.atc_table_count
    }

    pub fn find_drug_by_name(&self, name: &str) -> Result<Option<DrugRecord<'a>>, DbError> {
        drug_table::find_by_name(
            self.drug_table(),
            self.string_table(),
            self.atc_table(),
            self.header.drug_count,
            name,
        )
    }

    pub fn drugs(&self) -> DrugIter<'a> {
        DrugIter {
            table: self.drug_table(),
            strings: self.string_table(),
            atc_table: self.atc_table(),
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
            self.string_table(),
            entry.record_offset,
            entry.record_len,
            index,
        )?;
        Ok(Some(record))
    }

    /// Every `DrugFact` known for `drug_id`: a contraindication, a boxed
    /// warning, and so on can all coexist for the same drug, so this
    /// returns every match, not just one.
    pub fn drug_facts(&self, drug_id: DrugId) -> Result<Vec<DrugFactRecord<'a>>, DbError> {
        let entries = drug_fact_index::find_by_drug(
            self.drug_fact_index(),
            self.header.drug_fact_count,
            drug_id,
        )?;
        entries
            .into_iter()
            .map(|entry| {
                drug_fact::parse(
                    self.drug_fact_records(),
                    self.string_table(),
                    entry.record_offset,
                    entry.record_len,
                )
            })
            .collect()
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

    fn atc_table(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.atc_table_offset,
            self.header.atc_table_len,
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

    fn drug_fact_index(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.drug_fact_index_offset,
            self.header.drug_fact_index_len,
        )
    }

    fn drug_fact_records(&self) -> &'a [u8] {
        slice_at(
            self.bytes,
            self.header.drug_fact_records_offset,
            self.header.drug_fact_records_len,
        )
    }
}

pub struct DrugIter<'a> {
    table: &'a [u8],
    strings: &'a [u8],
    atc_table: &'a [u8],
    count: u32,
    next: u32,
}

impl<'a> Iterator for DrugIter<'a> {
    type Item = Result<DrugRecord<'a>, DbError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.count {
            return None;
        }
        let record = drug_table::parse_record(self.table, self.strings, self.atc_table, self.next);
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
    use crate::test_support::{build_men_file, TestClaim, TestDrug, TestDrugFact, TestInteraction};
    use mensung_domain::{DrugFactKind, DrugId, Severity};

    fn ddinter_claim(severity: Severity, rationale: &'static str) -> TestClaim {
        TestClaim {
            source_id: "ddinter",
            source_name: "DDInter",
            tier: 2,
            severity,
            evidence: mensung_domain::EvidenceLevel::Established,
            confidence: 2,
            year: 2026,
            month: 7,
            day: 14,
            rationale,
        }
    }

    fn build_test_database() -> Vec<u8> {
        build_men_file(
            vec![
                TestDrug {
                    id: 0,
                    name: "Aspirin",
                    rxcui: None,
                    pubchem_cid: None,
                    molecular_formula: None,
                    molecular_weight: None,
                    iupac_name: None,
                    atc_codes: vec![],
                },
                TestDrug {
                    id: 1,
                    name: "Warfarin",
                    rxcui: Some("11289"),
                    pubchem_cid: Some(54678486),
                    molecular_formula: Some("C19H16O4"),
                    molecular_weight: Some("308.3"),
                    iupac_name: None,
                    atc_codes: vec![("B01AA", "Vitamin K antagonists")],
                },
            ],
            &[TestInteraction {
                id: 0,
                drug_a: 0,
                drug_b: 1,
                claims: vec![ddinter_claim(
                    Severity::Contraindicated,
                    "Increased bleeding and hemorrhage probability.",
                )],
            }],
            &[TestDrugFact {
                id: 0,
                drug: 1,
                kind: DrugFactKind::BoxedWarning,
                claims: vec![ddinter_claim(
                    Severity::Contraindicated,
                    "Warfarin sodium can cause major or fatal bleeding.",
                )],
            }],
        )
    }

    #[test]
    fn opens_a_well_formed_database() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        assert_eq!(db.drug_count(), 2);
        assert_eq!(db.interaction_count(), 1);
        assert_eq!(db.drug_fact_count(), 1);
    }

    #[test]
    fn finds_a_drug_by_exact_name_with_its_cross_reference_data() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let drug = db.find_drug_by_name("Warfarin").unwrap().unwrap();
        assert_eq!(drug.id(), DrugId::new(1));
        assert_eq!(drug.rxcui(), Some("11289"));
        assert_eq!(drug.pubchem_cid(), Some(54678486));
        assert_eq!(drug.molecular_formula(), Some("C19H16O4"));
        let codes: Vec<&str> = drug.atc_codes().map(|c| c.unwrap().code()).collect();
        assert_eq!(codes, vec!["B01AA"]);
    }

    #[test]
    fn a_drug_with_no_cross_reference_data_reports_none() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let drug = db.find_drug_by_name("Aspirin").unwrap().unwrap();
        assert_eq!(drug.rxcui(), None);
        assert_eq!(drug.pubchem_cid(), None);
        assert_eq!(drug.atc_codes().count(), 0);
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
        assert_eq!(interaction.claims().len(), 1);
    }

    #[test]
    fn returns_none_for_a_pair_with_no_known_interaction() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let pair = DrugPair::new(DrugId::new(0), DrugId::new(99)).unwrap();
        assert!(db.find_interaction(pair).unwrap().is_none());
    }

    #[test]
    fn finds_drug_facts_for_a_drug() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let facts = db.drug_facts(DrugId::new(1)).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].kind(), DrugFactKind::BoxedWarning);
        assert_eq!(
            facts[0].primary_claim().rationale(),
            "Warfarin sodium can cause major or fatal bleeding."
        );
    }

    #[test]
    fn returns_no_drug_facts_for_a_drug_with_none() {
        let bytes = build_test_database();
        let db = Database::open(&bytes).unwrap();
        let facts = db.drug_facts(DrugId::new(0)).unwrap();
        assert!(facts.is_empty());
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
                    for drug in db.drugs().flatten() {
                        for atc in drug.atc_codes() {
                            let _ = atc;
                        }
                    }
                    let pair = DrugPair::new(DrugId::new(0), DrugId::new(1)).unwrap();
                    let _ = db.find_interaction(pair);
                    let _ = db.drug_facts(DrugId::new(1));
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

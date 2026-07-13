//! Test-only .men file builder. Used by this crate's own tests and, via the
//! `test-support` feature, by other workspace crates' tests, so no crate
//! duplicates this byte-encoding logic while writing its own tests.
//! `mensung-builder` becomes the real, production encoder in Phase 5; this
//! stays a test fixture builder even after that lands.

use mensung_domain::{EvidenceLevel, Severity};
use sha2::{Digest, Sha256};

use crate::header::HEADER_LEN;

pub struct TestDrug {
    pub id: u32,
    pub name: &'static str,
}

pub struct TestInteraction {
    pub id: u32,
    pub drug_a: u32,
    pub drug_b: u32,
    pub severity: Severity,
    pub description: &'static str,
    pub evidence: EvidenceLevel,
    pub source: &'static str,
}

fn severity_byte(severity: Severity) -> u8 {
    match severity {
        Severity::Contraindicated => 0,
        Severity::HighRisk => 1,
        Severity::Moderate => 2,
        Severity::Minor => 3,
        Severity::Unknown => 4,
    }
}

fn evidence_byte(evidence: EvidenceLevel) -> u8 {
    match evidence {
        EvidenceLevel::Established => 0,
        EvidenceLevel::Probable => 1,
        EvidenceLevel::Theoretical => 2,
    }
}

pub fn build_men_file(mut drugs: Vec<TestDrug>, interactions: &[TestInteraction]) -> Vec<u8> {
    drugs.sort_by_key(|d| d.name);

    let mut string_table = Vec::new();
    let mut drug_table = Vec::new();
    for drug in &drugs {
        let offset = string_table.len() as u32;
        string_table.extend_from_slice(drug.name.as_bytes());
        drug_table.extend_from_slice(&drug.id.to_le_bytes());
        drug_table.extend_from_slice(&offset.to_le_bytes());
        drug_table.extend_from_slice(&(drug.name.len() as u16).to_le_bytes());
        drug_table.extend_from_slice(&0u16.to_le_bytes());
    }

    let mut sorted_interactions: Vec<&TestInteraction> = interactions.iter().collect();
    sorted_interactions.sort_by_key(|i| (i.drug_a.min(i.drug_b), i.drug_a.max(i.drug_b)));

    let mut interaction_index = Vec::new();
    let mut interaction_records = Vec::new();
    for interaction in &sorted_interactions {
        let lower = interaction.drug_a.min(interaction.drug_b);
        let higher = interaction.drug_a.max(interaction.drug_b);

        let record_offset = interaction_records.len() as u32;
        let mut record = Vec::new();
        record.extend_from_slice(&interaction.id.to_le_bytes());
        record.extend_from_slice(&lower.to_le_bytes());
        record.extend_from_slice(&higher.to_le_bytes());
        record.push(severity_byte(interaction.severity));
        record.push(evidence_byte(interaction.evidence));
        record.extend_from_slice(&0u16.to_le_bytes());
        record.extend_from_slice(&(interaction.description.len() as u32).to_le_bytes());
        record.extend_from_slice(interaction.description.as_bytes());
        record.extend_from_slice(&(interaction.source.len() as u32).to_le_bytes());
        record.extend_from_slice(interaction.source.as_bytes());

        interaction_index.extend_from_slice(&lower.to_le_bytes());
        interaction_index.extend_from_slice(&higher.to_le_bytes());
        interaction_index.extend_from_slice(&record_offset.to_le_bytes());
        interaction_index.extend_from_slice(&(record.len() as u32).to_le_bytes());

        interaction_records.extend_from_slice(&record);
    }

    let string_table_offset = HEADER_LEN as u64;
    let string_table_len = string_table.len() as u64;
    let drug_table_offset = string_table_offset + string_table_len;
    let drug_table_len = drug_table.len() as u64;
    let interaction_index_offset = drug_table_offset + drug_table_len;
    let interaction_index_len = interaction_index.len() as u64;
    let interaction_records_offset = interaction_index_offset + interaction_index_len;
    let interaction_records_len = interaction_records.len() as u64;

    let mut payload = Vec::new();
    payload.extend_from_slice(&string_table);
    payload.extend_from_slice(&drug_table);
    payload.extend_from_slice(&interaction_index);
    payload.extend_from_slice(&interaction_records);

    let payload_sha256: [u8; 32] = Sha256::digest(&payload).into();

    let mut header = vec![0u8; HEADER_LEN];
    header[0..4].copy_from_slice(b"MEN1");
    header[4..6].copy_from_slice(&1u16.to_le_bytes());
    header[6..8].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
    header[12..20].copy_from_slice(&0u64.to_le_bytes());
    header[20..52].copy_from_slice(&payload_sha256);
    header[52..56].copy_from_slice(&(drugs.len() as u32).to_le_bytes());
    header[56..60].copy_from_slice(&(sorted_interactions.len() as u32).to_le_bytes());
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

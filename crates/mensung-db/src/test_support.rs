//! Test-only .men file builder. Used by this crate's own tests and, via the
//! `test-support` feature, by other workspace crates' tests, so no crate
//! duplicates this byte-encoding logic while writing its own tests.
//! `mensung-builder` is the real, production encoder; this stays a
//! separate, deliberately simple test fixture builder (no string
//! deduplication, unlike the real writer) even after that lands, so a bug
//! shared between the two would still be caught by one producing output
//! the other cannot read.

use mensung_domain::{DrugFactKind, EvidenceLevel, Severity};
use sha2::{Digest, Sha256};

use crate::header::HEADER_LEN;
use crate::layout;

pub struct TestClaim {
    pub source_id: &'static str,
    pub source_name: &'static str,
    pub tier: u8,
    pub severity: Severity,
    pub evidence: EvidenceLevel,
    pub confidence: u8,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub rationale: &'static str,
}

pub struct TestDrug {
    pub id: u32,
    pub name: &'static str,
    pub rxcui: Option<&'static str>,
    pub pubchem_cid: Option<u32>,
    pub molecular_formula: Option<&'static str>,
    pub molecular_weight: Option<&'static str>,
    pub iupac_name: Option<&'static str>,
    pub atc_codes: Vec<(&'static str, &'static str)>,
}

impl TestDrug {
    /// A drug with no RxCUI, chemical properties, or ATC codes: what most
    /// callers testing lookup/fuzzy-match/interaction behavior actually
    /// need, since that behavior does not depend on the cross-reference
    /// fields.
    pub fn plain(id: u32, name: &'static str) -> Self {
        Self {
            id,
            name,
            rxcui: None,
            pubchem_cid: None,
            molecular_formula: None,
            molecular_weight: None,
            iupac_name: None,
            atc_codes: Vec::new(),
        }
    }
}

pub struct TestInteraction {
    pub id: u32,
    pub drug_a: u32,
    pub drug_b: u32,
    pub claims: Vec<TestClaim>,
}

impl TestInteraction {
    /// A single-claim interaction, the shape most callers testing
    /// resolved-severity behavior actually need rather than a full
    /// multi-source scenario.
    pub fn simple(
        id: u32,
        drug_a: u32,
        drug_b: u32,
        severity: Severity,
        evidence: EvidenceLevel,
        source: &'static str,
        rationale: &'static str,
    ) -> Self {
        Self {
            id,
            drug_a,
            drug_b,
            claims: vec![TestClaim {
                source_id: "test-source",
                source_name: source,
                tier: 2,
                severity,
                evidence,
                confidence: 1,
                year: 2026,
                month: 1,
                day: 1,
                rationale,
            }],
        }
    }
}

pub struct TestDrugFact {
    pub id: u32,
    pub drug: u32,
    pub kind: DrugFactKind,
    pub claims: Vec<TestClaim>,
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

fn drug_fact_kind_byte(kind: DrugFactKind) -> u8 {
    match kind {
        DrugFactKind::Contraindication => 0,
        DrugFactKind::Warning => 1,
        DrugFactKind::BoxedWarning => 2,
        DrugFactKind::Pregnancy => 3,
        DrugFactKind::Breastfeeding => 4,
        DrugFactKind::Dosage => 5,
        DrugFactKind::Indication => 6,
    }
}

fn intern(table: &mut Vec<u8>, s: &str) -> (u32, u32) {
    let offset = table.len() as u32;
    table.extend_from_slice(s.as_bytes());
    (offset, s.len() as u32)
}

fn encode_claim(string_table: &mut Vec<u8>, claim: &TestClaim) -> Vec<u8> {
    let (source_id_offset, source_id_len) = intern(string_table, claim.source_id);
    let (source_name_offset, source_name_len) = intern(string_table, claim.source_name);
    let (rationale_offset, rationale_len) = intern(string_table, claim.rationale);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&source_id_offset.to_le_bytes());
    bytes.extend_from_slice(&(source_id_len as u16).to_le_bytes());
    bytes.extend_from_slice(&source_name_offset.to_le_bytes());
    bytes.extend_from_slice(&(source_name_len as u16).to_le_bytes());
    bytes.push(claim.tier);
    bytes.push(severity_byte(claim.severity));
    bytes.push(evidence_byte(claim.evidence));
    bytes.push(claim.confidence);
    bytes.extend_from_slice(&claim.year.to_le_bytes());
    bytes.push(claim.month);
    bytes.push(claim.day);
    bytes.extend_from_slice(&rationale_offset.to_le_bytes());
    bytes.extend_from_slice(&rationale_len.to_le_bytes());
    bytes
}

pub fn build_men_file(
    mut drugs: Vec<TestDrug>,
    interactions: &[TestInteraction],
    drug_facts: &[TestDrugFact],
) -> Vec<u8> {
    drugs.sort_by_key(|d| d.name);

    let mut string_table = Vec::new();
    let mut drug_table = Vec::new();
    let mut atc_table = Vec::new();

    for drug in &drugs {
        let (name_offset, name_len) = intern(&mut string_table, drug.name);
        let (rxcui_offset, rxcui_len) = drug
            .rxcui
            .map(|r| intern(&mut string_table, r))
            .unwrap_or((0, 0));
        let pubchem_cid = drug.pubchem_cid.unwrap_or(0);
        let (formula_offset, formula_len) = drug
            .molecular_formula
            .map(|f| intern(&mut string_table, f))
            .unwrap_or((0, 0));
        let (weight_offset, weight_len) = drug
            .molecular_weight
            .map(|w| intern(&mut string_table, w))
            .unwrap_or((0, 0));
        let (iupac_offset, iupac_len) = drug
            .iupac_name
            .map(|i| intern(&mut string_table, i))
            .unwrap_or((0, 0));

        let atc_start_index = (atc_table.len() / 12) as u32;
        for (code, class_name) in &drug.atc_codes {
            let (class_offset, class_len) = intern(&mut string_table, class_name);
            atc_table.extend_from_slice(code.as_bytes());
            atc_table.push(0);
            atc_table.extend_from_slice(&class_offset.to_le_bytes());
            atc_table.extend_from_slice(&(class_len as u16).to_le_bytes());
        }
        let atc_count = drug.atc_codes.len() as u16;

        drug_table.extend_from_slice(&drug.id.to_le_bytes());
        drug_table.extend_from_slice(&name_offset.to_le_bytes());
        drug_table.extend_from_slice(&(name_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&0u16.to_le_bytes());
        drug_table.extend_from_slice(&rxcui_offset.to_le_bytes());
        drug_table.extend_from_slice(&(rxcui_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&pubchem_cid.to_le_bytes());
        drug_table.extend_from_slice(&formula_offset.to_le_bytes());
        drug_table.extend_from_slice(&(formula_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&weight_offset.to_le_bytes());
        drug_table.extend_from_slice(&(weight_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&iupac_offset.to_le_bytes());
        drug_table.extend_from_slice(&(iupac_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&atc_start_index.to_le_bytes());
        drug_table.extend_from_slice(&atc_count.to_le_bytes());
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
        record.extend_from_slice(&(interaction.claims.len() as u16).to_le_bytes());
        record.extend_from_slice(&0u16.to_le_bytes());
        for claim in &interaction.claims {
            record.extend_from_slice(&encode_claim(&mut string_table, claim));
        }

        interaction_index.extend_from_slice(&lower.to_le_bytes());
        interaction_index.extend_from_slice(&higher.to_le_bytes());
        interaction_index.extend_from_slice(&record_offset.to_le_bytes());
        interaction_index.extend_from_slice(&(record.len() as u32).to_le_bytes());

        interaction_records.extend_from_slice(&record);
    }

    let mut sorted_facts: Vec<&TestDrugFact> = drug_facts.iter().collect();
    sorted_facts.sort_by_key(|f| (f.drug, drug_fact_kind_byte(f.kind)));

    let mut drug_fact_index = Vec::new();
    let mut drug_fact_records = Vec::new();
    for fact in &sorted_facts {
        let record_offset = drug_fact_records.len() as u32;
        let mut record = Vec::new();
        record.extend_from_slice(&fact.id.to_le_bytes());
        record.extend_from_slice(&fact.drug.to_le_bytes());
        record.push(drug_fact_kind_byte(fact.kind));
        record.push(0);
        record.extend_from_slice(&(fact.claims.len() as u16).to_le_bytes());
        for claim in &fact.claims {
            record.extend_from_slice(&encode_claim(&mut string_table, claim));
        }

        drug_fact_index.extend_from_slice(&fact.drug.to_le_bytes());
        drug_fact_index.push(drug_fact_kind_byte(fact.kind));
        drug_fact_index.extend_from_slice(&[0u8; 3]);
        drug_fact_index.extend_from_slice(&record_offset.to_le_bytes());
        drug_fact_index.extend_from_slice(&(record.len() as u32).to_le_bytes());

        drug_fact_records.extend_from_slice(&record);
    }

    let string_table_offset = HEADER_LEN as u64;
    let string_table_len = string_table.len() as u64;
    let drug_table_offset = string_table_offset + string_table_len;
    let drug_table_len = drug_table.len() as u64;
    let atc_table_offset = drug_table_offset + drug_table_len;
    let atc_table_len = atc_table.len() as u64;
    let interaction_index_offset = atc_table_offset + atc_table_len;
    let interaction_index_len = interaction_index.len() as u64;
    let interaction_records_offset = interaction_index_offset + interaction_index_len;
    let interaction_records_len = interaction_records.len() as u64;
    let drug_fact_index_offset = interaction_records_offset + interaction_records_len;
    let drug_fact_index_len = drug_fact_index.len() as u64;
    let drug_fact_records_offset = drug_fact_index_offset + drug_fact_index_len;
    let drug_fact_records_len = drug_fact_records.len() as u64;

    let mut payload = Vec::new();
    payload.extend_from_slice(&string_table);
    payload.extend_from_slice(&drug_table);
    payload.extend_from_slice(&atc_table);
    payload.extend_from_slice(&interaction_index);
    payload.extend_from_slice(&interaction_records);
    payload.extend_from_slice(&drug_fact_index);
    payload.extend_from_slice(&drug_fact_records);

    let payload_sha256: [u8; 32] = Sha256::digest(&payload).into();

    let mut header = vec![0u8; HEADER_LEN];
    header[layout::MAGIC].copy_from_slice(b"MEN1");
    header[layout::FORMAT_VERSION].copy_from_slice(&2u16.to_le_bytes());
    header[layout::HEADER_LEN_FIELD].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
    header[layout::BUILD_TIMESTAMP].copy_from_slice(&0u64.to_le_bytes());
    header[layout::PAYLOAD_SHA256].copy_from_slice(&payload_sha256);
    header[layout::DRUG_COUNT].copy_from_slice(&(drugs.len() as u32).to_le_bytes());
    header[layout::INTERACTION_COUNT]
        .copy_from_slice(&(sorted_interactions.len() as u32).to_le_bytes());
    header[layout::STRING_TABLE_OFFSET].copy_from_slice(&string_table_offset.to_le_bytes());
    header[layout::STRING_TABLE_LEN].copy_from_slice(&string_table_len.to_le_bytes());
    header[layout::DRUG_TABLE_OFFSET].copy_from_slice(&drug_table_offset.to_le_bytes());
    header[layout::DRUG_TABLE_LEN].copy_from_slice(&drug_table_len.to_le_bytes());
    header[layout::INTERACTION_INDEX_OFFSET]
        .copy_from_slice(&interaction_index_offset.to_le_bytes());
    header[layout::INTERACTION_INDEX_LEN].copy_from_slice(&interaction_index_len.to_le_bytes());
    header[layout::INTERACTION_RECORDS_OFFSET]
        .copy_from_slice(&interaction_records_offset.to_le_bytes());
    header[layout::INTERACTION_RECORDS_LEN].copy_from_slice(&interaction_records_len.to_le_bytes());
    header[layout::RESERVED_V1].copy_from_slice(&[0u8; 4]);
    header[layout::ATC_TABLE_OFFSET].copy_from_slice(&atc_table_offset.to_le_bytes());
    header[layout::ATC_TABLE_LEN].copy_from_slice(&atc_table_len.to_le_bytes());
    header[layout::ATC_TABLE_COUNT].copy_from_slice(&((atc_table.len() / 12) as u32).to_le_bytes());
    header[layout::DRUG_FACT_INDEX_OFFSET].copy_from_slice(&drug_fact_index_offset.to_le_bytes());
    header[layout::DRUG_FACT_INDEX_LEN].copy_from_slice(&drug_fact_index_len.to_le_bytes());
    header[layout::DRUG_FACT_COUNT].copy_from_slice(&(sorted_facts.len() as u32).to_le_bytes());
    header[layout::DRUG_FACT_RECORDS_OFFSET]
        .copy_from_slice(&drug_fact_records_offset.to_le_bytes());
    header[layout::DRUG_FACT_RECORDS_LEN].copy_from_slice(&drug_fact_records_len.to_le_bytes());
    header[layout::RESERVED].copy_from_slice(&[0u8; 8]);

    let crc = {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&header[..layout::HEADER_CRC32.start]);
        hasher.update(&header[layout::HEADER_CRC32.end..]);
        hasher.finalize()
    };
    header[layout::HEADER_CRC32].copy_from_slice(&crc.to_le_bytes());

    let mut full = header;
    full.extend_from_slice(&payload);
    full
}

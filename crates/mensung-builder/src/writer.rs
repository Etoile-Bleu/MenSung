//! Compiles a validated set of drugs and interactions into a .men byte
//! buffer, following docs/DATABASE_FORMAT.md exactly. Assumes the caller
//! already validated the dataset with `validate::validate` and got back no
//! issues; this does not re-check duplicate names or dangling references,
//! it only encodes what it is given. `build_timestamp` honors
//! `SOURCE_DATE_EPOCH` when set, so builds stay byte-for-byte reproducible
//! in CI and release pipelines that set it; it falls back to wall-clock
//! time for local, non-reproducible development builds.

use mensung_domain::{Drug, EvidenceLevel, Interaction, Severity};
use sha2::{Digest, Sha256};

const HEADER_LEN: usize = 128;

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

fn build_timestamp() -> u64 {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        })
}

pub fn compile(mut drugs: Vec<Drug>, interactions: &[Interaction]) -> Vec<u8> {
    drugs.sort_by(|a, b| a.inn_name().as_str().cmp(b.inn_name().as_str()));

    let mut string_table = Vec::new();
    let mut drug_table = Vec::new();
    for drug in &drugs {
        let name = drug.inn_name().as_str();
        let offset = string_table.len() as u32;
        string_table.extend_from_slice(name.as_bytes());
        drug_table.extend_from_slice(&drug.id().value().to_le_bytes());
        drug_table.extend_from_slice(&offset.to_le_bytes());
        drug_table.extend_from_slice(&(name.len() as u16).to_le_bytes());
        drug_table.extend_from_slice(&0u16.to_le_bytes());
    }

    let mut sorted_interactions: Vec<&Interaction> = interactions.iter().collect();
    sorted_interactions.sort_by_key(|i| i.pair().drugs());

    let mut interaction_index = Vec::new();
    let mut interaction_records = Vec::new();
    for interaction in &sorted_interactions {
        let (lower, higher) = interaction.pair().drugs();
        let record_offset = interaction_records.len() as u32;

        let description = interaction.description();
        let source = interaction.source();

        let mut record = Vec::new();
        record.extend_from_slice(&interaction.id().value().to_le_bytes());
        record.extend_from_slice(&lower.value().to_le_bytes());
        record.extend_from_slice(&higher.value().to_le_bytes());
        record.push(severity_byte(interaction.severity()));
        record.push(evidence_byte(interaction.evidence()));
        record.extend_from_slice(&0u16.to_le_bytes());
        record.extend_from_slice(&(description.len() as u32).to_le_bytes());
        record.extend_from_slice(description.as_bytes());
        record.extend_from_slice(&(source.len() as u32).to_le_bytes());
        record.extend_from_slice(source.as_bytes());

        interaction_index.extend_from_slice(&lower.value().to_le_bytes());
        interaction_index.extend_from_slice(&higher.value().to_le_bytes());
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
    header[12..20].copy_from_slice(&build_timestamp().to_le_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::{DrugId, DrugPair, InnName, InteractionId};

    #[test]
    fn compiled_output_opens_and_round_trips_through_mensung_db() {
        let aspirin = DrugId::new(0);
        let warfarin = DrugId::new(1);
        let drugs = vec![
            Drug::new(aspirin, InnName::parse("Aspirin").unwrap()),
            Drug::new(warfarin, InnName::parse("Warfarin").unwrap()),
        ];
        let interactions = vec![Interaction::new(
            InteractionId::new(0),
            DrugPair::new(aspirin, warfarin).unwrap(),
            Severity::Contraindicated,
            "Increased bleeding and hemorrhage probability.",
            EvidenceLevel::Established,
            "WHO drug interaction reference",
        )
        .unwrap()];

        let bytes = compile(drugs, &interactions);
        let db = mensung_db::Database::open(&bytes).unwrap();

        assert_eq!(db.drug_count(), 2);
        let drug = db.find_drug_by_name("Aspirin").unwrap().unwrap();
        let pair = DrugPair::new(drug.id(), warfarin).unwrap();
        let interaction = db.find_interaction(pair).unwrap().unwrap();
        assert_eq!(interaction.severity(), Severity::Contraindicated);
    }
}

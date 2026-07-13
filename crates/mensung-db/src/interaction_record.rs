//! Reads a single variable-length Interaction Record, addressed by the
//! offset and length carried in its Interaction Index entry, as defined in
//! docs/DATABASE_FORMAT.md.

use mensung_domain::{DrugId, DrugPair, EvidenceLevel, InteractionId, Severity};

use crate::bytes::read_u32;
use crate::DbError;

const PREFIX_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionRecord<'a> {
    id: InteractionId,
    pair: DrugPair,
    severity: Severity,
    description: &'a str,
    evidence: EvidenceLevel,
    source: &'a str,
}

impl<'a> InteractionRecord<'a> {
    pub fn id(&self) -> InteractionId {
        self.id
    }

    pub fn pair(&self) -> DrugPair {
        self.pair
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn description(&self) -> &'a str {
        self.description
    }

    pub fn evidence(&self) -> EvidenceLevel {
        self.evidence
    }

    pub fn source(&self) -> &'a str {
        self.source
    }
}

fn parse_severity(byte: u8, index: u32) -> Result<Severity, DbError> {
    match byte {
        0 => Ok(Severity::Contraindicated),
        1 => Ok(Severity::HighRisk),
        2 => Ok(Severity::Moderate),
        3 => Ok(Severity::Minor),
        4 => Ok(Severity::Unknown),
        value => Err(DbError::InvalidSeverity { index, value }),
    }
}

fn parse_evidence(byte: u8, index: u32) -> Result<EvidenceLevel, DbError> {
    match byte {
        0 => Ok(EvidenceLevel::Established),
        1 => Ok(EvidenceLevel::Probable),
        2 => Ok(EvidenceLevel::Theoretical),
        value => Err(DbError::InvalidEvidence { index, value }),
    }
}

pub(crate) fn parse<'a>(
    records: &'a [u8],
    offset: u32,
    declared_len: u32,
    index: u32,
) -> Result<InteractionRecord<'a>, DbError> {
    let start = offset as usize;

    let interaction_id = read_u32(records, start)?;
    let drug_id_lower = read_u32(records, start + 4)?;
    let drug_id_higher = read_u32(records, start + 8)?;
    let severity_byte = *records.get(start + 12).ok_or(DbError::Truncated)?;
    let evidence_byte = *records.get(start + 13).ok_or(DbError::Truncated)?;
    let description_len = read_u32(records, start + 16)? as usize;

    let description_start = start + PREFIX_LEN;
    let description_end = description_start + description_len;
    let description_bytes = records
        .get(description_start..description_end)
        .ok_or(DbError::Truncated)?;
    let description = std::str::from_utf8(description_bytes)
        .map_err(|_| DbError::InvalidInteractionText { index })?;

    let source_len = read_u32(records, description_end)? as usize;
    let source_start = description_end + 4;
    let source_end = source_start + source_len;
    let source_bytes = records
        .get(source_start..source_end)
        .ok_or(DbError::Truncated)?;
    let source =
        std::str::from_utf8(source_bytes).map_err(|_| DbError::InvalidInteractionText { index })?;

    let actual_len = (PREFIX_LEN + description_len + 4 + source_len) as u32;
    if actual_len != declared_len {
        return Err(DbError::InteractionRecordLengthMismatch {
            offset: offset as u64,
            declared: declared_len,
            actual: actual_len,
        });
    }

    if drug_id_lower == drug_id_higher {
        return Err(DbError::CorruptPair { index });
    }
    let pair = DrugPair::new(DrugId::new(drug_id_lower), DrugId::new(drug_id_higher))
        .expect("drug_id_lower != drug_id_higher was just checked above");

    Ok(InteractionRecord {
        id: InteractionId::new(interaction_id),
        pair,
        severity: parse_severity(severity_byte, index)?,
        description,
        evidence: parse_evidence(evidence_byte, index)?,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_record(
        id: u32,
        lower: u32,
        higher: u32,
        severity: u8,
        evidence: u8,
        description: &str,
        source: &str,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&lower.to_le_bytes());
        bytes.extend_from_slice(&higher.to_le_bytes());
        bytes.push(severity);
        bytes.push(evidence);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&(description.len() as u32).to_le_bytes());
        bytes.extend_from_slice(description.as_bytes());
        bytes.extend_from_slice(&(source.len() as u32).to_le_bytes());
        bytes.extend_from_slice(source.as_bytes());
        bytes
    }

    #[test]
    fn parses_a_well_formed_record() {
        let bytes = encode_record(
            1,
            1,
            2,
            0,
            0,
            "Increased bleeding and hemorrhage probability.",
            "WHO drug interaction reference",
        );
        let record = parse(&bytes, 0, bytes.len() as u32, 0).unwrap();
        assert_eq!(record.id(), InteractionId::new(1));
        assert_eq!(record.severity(), Severity::Contraindicated);
        assert_eq!(record.evidence(), EvidenceLevel::Established);
        assert_eq!(
            record.description(),
            "Increased bleeding and hemorrhage probability."
        );
        assert_eq!(record.source(), "WHO drug interaction reference");
    }

    #[test]
    fn rejects_a_declared_length_that_does_not_match_the_actual_length() {
        let bytes = encode_record(1, 1, 2, 0, 0, "desc", "src");
        let err = parse(&bytes, 0, bytes.len() as u32 + 1, 0).unwrap_err();
        assert!(matches!(
            err,
            DbError::InteractionRecordLengthMismatch { .. }
        ));
    }

    #[test]
    fn rejects_an_unrecognized_severity_byte() {
        let bytes = encode_record(1, 1, 2, 200, 0, "desc", "src");
        assert_eq!(
            parse(&bytes, 0, bytes.len() as u32, 7).unwrap_err(),
            DbError::InvalidSeverity {
                index: 7,
                value: 200
            }
        );
    }

    #[test]
    fn rejects_an_unrecognized_evidence_byte() {
        let bytes = encode_record(1, 1, 2, 0, 200, "desc", "src");
        assert_eq!(
            parse(&bytes, 0, bytes.len() as u32, 7).unwrap_err(),
            DbError::InvalidEvidence {
                index: 7,
                value: 200
            }
        );
    }

    #[test]
    fn rejects_a_drug_paired_with_itself() {
        let bytes = encode_record(1, 5, 5, 0, 0, "desc", "src");
        assert_eq!(
            parse(&bytes, 0, bytes.len() as u32, 3).unwrap_err(),
            DbError::CorruptPair { index: 3 }
        );
    }
}

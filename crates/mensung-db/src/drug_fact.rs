//! Reads a single variable-length Drug Fact Record, addressed by the
//! offset and length carried in its Drug Fact Index entry, as defined in
//! docs/DATABASE_FORMAT.md. Same shape as an Interaction Record (see
//! `interaction_record.rs`), except keyed to one drug instead of a pair,
//! and carrying the fact's `kind`.

use mensung_domain::{DrugFactId, DrugFactKind, DrugId};

use crate::bytes::read_u32;
use crate::claim_record::{self, ClaimRecord};
use crate::DbError;

const PREFIX_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrugFactRecord<'a> {
    id: DrugFactId,
    drug: DrugId,
    kind: DrugFactKind,
    claims: Vec<ClaimRecord<'a>>,
}

impl<'a> DrugFactRecord<'a> {
    pub fn id(&self) -> DrugFactId {
        self.id
    }

    pub fn drug(&self) -> DrugId {
        self.drug
    }

    pub fn kind(&self) -> DrugFactKind {
        self.kind
    }

    /// Every claim contributing to this fact, one per source. Never
    /// empty: `mensung_domain::DrugFact::new` rejects zero claims.
    pub fn claims(&self) -> &[ClaimRecord<'a>] {
        &self.claims
    }

    /// The claim to treat as authoritative, same tie-break rule as
    /// `InteractionRecord::primary_claim`.
    pub fn primary_claim(&self) -> ClaimRecord<'a> {
        claim_record::primary_claim(&self.claims)
    }
}

fn parse_kind(byte: u8) -> Result<DrugFactKind, DbError> {
    match byte {
        0 => Ok(DrugFactKind::Contraindication),
        1 => Ok(DrugFactKind::Warning),
        2 => Ok(DrugFactKind::BoxedWarning),
        3 => Ok(DrugFactKind::Pregnancy),
        4 => Ok(DrugFactKind::Breastfeeding),
        5 => Ok(DrugFactKind::Dosage),
        6 => Ok(DrugFactKind::Indication),
        value => Err(DbError::InvalidDrugFactKind(value)),
    }
}

pub(crate) fn parse<'a>(
    records: &'a [u8],
    strings: &'a [u8],
    offset: u32,
    declared_len: u32,
) -> Result<DrugFactRecord<'a>, DbError> {
    let start = offset as usize;

    let drug_fact_id = read_u32(records, start)?;
    let drug_id = read_u32(records, start + 4)?;
    let kind = parse_kind(*records.get(start + 8).ok_or(DbError::Truncated)?)?;
    let claim_count = {
        let bytes = records
            .get(start + 10..start + 12)
            .ok_or(DbError::Truncated)?;
        u16::from_le_bytes(bytes.try_into().expect("slice of exactly 2 bytes"))
    };

    let claims_start = start + PREFIX_LEN;
    let claims_len = claim_count as usize * claim_record::CLAIM_LEN;
    let claims_bytes = records
        .get(claims_start..claims_start + claims_len)
        .ok_or(DbError::Truncated)?;

    let mut claims = Vec::with_capacity(claim_count as usize);
    for chunk in claims_bytes.chunks_exact(claim_record::CLAIM_LEN) {
        claims.push(claim_record::parse(chunk, strings)?);
    }

    let actual_len = (PREFIX_LEN + claims_len) as u32;
    if actual_len != declared_len {
        return Err(DbError::DrugFactRecordLengthMismatch {
            offset: offset as u64,
            declared: declared_len,
            actual: actual_len,
        });
    }

    if claims.is_empty() {
        return Err(DbError::Truncated);
    }

    Ok(DrugFactRecord {
        id: DrugFactId::new(drug_fact_id),
        drug: DrugId::new(drug_id),
        kind,
        claims,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim_record::tests::encode_claim;
    use mensung_domain::{EvidenceLevel, Severity};

    fn encode_record(drug_fact_id: u32, drug_id: u32, kind: u8, claims: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&drug_fact_id.to_le_bytes());
        bytes.extend_from_slice(&drug_id.to_le_bytes());
        bytes.push(kind);
        bytes.push(0);
        bytes.extend_from_slice(&(claims.len() as u16).to_le_bytes());
        for claim in claims {
            bytes.extend_from_slice(claim);
        }
        bytes
    }

    #[test]
    fn parses_a_well_formed_record() {
        let mut strings = Vec::new();
        let claim = encode_claim(
            &mut strings,
            "openfda-label",
            "OpenFDA Drug Labeling",
            2,
            0,
            0,
            1,
            2026,
            7,
            14,
            "Warfarin sodium can cause major or fatal bleeding.",
        );
        let record = encode_record(0, 5, 2, std::slice::from_ref(&claim));

        let parsed = parse(&record, &strings, 0, record.len() as u32).unwrap();
        assert_eq!(parsed.id(), DrugFactId::new(0));
        assert_eq!(parsed.drug(), DrugId::new(5));
        assert_eq!(parsed.kind(), DrugFactKind::BoxedWarning);
        assert_eq!(parsed.primary_claim().severity(), Severity::Contraindicated);
        assert_eq!(
            parsed.primary_claim().evidence(),
            EvidenceLevel::Established
        );
        assert_eq!(parsed.claims().len(), 1);
    }

    #[test]
    fn rejects_an_unrecognized_kind_byte() {
        let mut strings = Vec::new();
        let claim = encode_claim(&mut strings, "x", "x", 0, 0, 0, 0, 2026, 1, 1, "r");
        let record = encode_record(0, 0, 200, std::slice::from_ref(&claim));
        assert_eq!(
            parse(&record, &strings, 0, record.len() as u32).unwrap_err(),
            DbError::InvalidDrugFactKind(200)
        );
    }

    #[test]
    fn rejects_a_declared_length_that_does_not_match_the_actual_length() {
        let mut strings = Vec::new();
        let claim = encode_claim(&mut strings, "x", "x", 0, 0, 0, 0, 2026, 1, 1, "r");
        let record = encode_record(0, 0, 0, std::slice::from_ref(&claim));
        let err = parse(&record, &strings, 0, record.len() as u32 + 1).unwrap_err();
        assert!(matches!(err, DbError::DrugFactRecordLengthMismatch { .. }));
    }
}

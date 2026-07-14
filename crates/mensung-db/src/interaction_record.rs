//! Reads a single variable-length Interaction Record, addressed by the
//! offset and length carried in its Interaction Index entry, as defined in
//! docs/DATABASE_FORMAT.md. A record carries one or more `ClaimRecord`s,
//! one per source asserting something about this drug pair; `severity()`,
//! `description()`, `evidence()`, and `source()` are convenience
//! accessors backed by the primary claim (the most authoritative source
//! tier present, most severe on a same-tier tie), the same resolved view
//! `mensung_domain::InteractionFact::resolve()` produces, kept under
//! their version-1 names since every caller in `mensung-core` and
//! `mensung-client` already uses them that way. `claims()` exposes the
//! full multi-source list for anything that wants more than the resolved
//! view.

use mensung_domain::{DrugId, DrugPair, EvidenceLevel, InteractionId, Severity};

use crate::bytes::read_u32;
use crate::claim_record::{self, ClaimRecord};
use crate::DbError;

const PREFIX_LEN: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionRecord<'a> {
    id: InteractionId,
    pair: DrugPair,
    claims: Vec<ClaimRecord<'a>>,
}

impl<'a> InteractionRecord<'a> {
    pub fn id(&self) -> InteractionId {
        self.id
    }

    pub fn pair(&self) -> DrugPair {
        self.pair
    }

    /// Every claim contributing to this interaction, one per source. Never
    /// empty: `mensung_domain::InteractionFact::new` rejects zero claims,
    /// so a record reaching this point already has at least one.
    pub fn claims(&self) -> &[ClaimRecord<'a>] {
        &self.claims
    }

    /// The claim to treat as authoritative: the most trusted source tier
    /// present, the most severe of those on a tie. See
    /// `mensung_domain::InteractionFact::primary_claim`'s header for the
    /// full reasoning; this mirrors it exactly.
    pub fn primary_claim(&self) -> ClaimRecord<'a> {
        claim_record::primary_claim(&self.claims)
    }

    pub fn severity(&self) -> Severity {
        self.primary_claim().severity()
    }

    pub fn description(&self) -> &'a str {
        self.primary_claim().rationale()
    }

    pub fn evidence(&self) -> EvidenceLevel {
        self.primary_claim().evidence()
    }

    pub fn source(&self) -> &'a str {
        self.primary_claim().source_name()
    }
}

pub(crate) fn parse<'a>(
    records: &'a [u8],
    strings: &'a [u8],
    offset: u32,
    declared_len: u32,
    index: u32,
) -> Result<InteractionRecord<'a>, DbError> {
    let start = offset as usize;

    let interaction_id = read_u32(records, start)?;
    let drug_id_lower = read_u32(records, start + 4)?;
    let drug_id_higher = read_u32(records, start + 8)?;
    let claim_count = {
        let bytes = records
            .get(start + 12..start + 14)
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

    if claims.is_empty() {
        return Err(DbError::Truncated);
    }

    Ok(InteractionRecord {
        id: InteractionId::new(interaction_id),
        pair,
        claims,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim_record::tests::encode_claim;
    use mensung_domain::SourceTier;

    fn encode_record(
        id: u32,
        lower: u32,
        higher: u32,
        strings: &mut Vec<u8>,
        claims: &[Vec<u8>],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&lower.to_le_bytes());
        bytes.extend_from_slice(&higher.to_le_bytes());
        bytes.extend_from_slice(&(claims.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        for claim in claims {
            bytes.extend_from_slice(claim);
        }
        let _ = strings;
        bytes
    }

    #[test]
    fn parses_a_well_formed_single_claim_record() {
        let mut strings = Vec::new();
        let claim = encode_claim(
            &mut strings,
            "ddinter",
            "DDInter (http://ddinter.scbdd.com/)",
            2,
            0,
            0,
            2,
            2026,
            7,
            14,
            "Increased bleeding and hemorrhage probability.",
        );
        let record = encode_record(1, 1, 2, &mut strings, std::slice::from_ref(&claim));

        let parsed = parse(&record, &strings, 0, record.len() as u32, 0).unwrap();
        assert_eq!(parsed.id(), InteractionId::new(1));
        assert_eq!(parsed.severity(), Severity::Contraindicated);
        assert_eq!(parsed.evidence(), EvidenceLevel::Established);
        assert_eq!(
            parsed.description(),
            "Increased bleeding and hemorrhage probability."
        );
        assert_eq!(parsed.source(), "DDInter (http://ddinter.scbdd.com/)");
        assert_eq!(parsed.claims().len(), 1);
    }

    #[test]
    fn resolves_the_primary_claim_among_several_without_losing_the_others() {
        let mut strings = Vec::new();
        let ddinter = encode_claim(
            &mut strings,
            "ddinter",
            "DDInter",
            2,
            3,
            0,
            1,
            2026,
            1,
            1,
            "minor per DDInter",
        );
        let fda = encode_claim(
            &mut strings,
            "fda-label",
            "FDA Label",
            0,
            0,
            0,
            2,
            2026,
            1,
            1,
            "contraindicated per FDA",
        );
        let record = encode_record(1, 1, 2, &mut strings, &[ddinter, fda]);

        let parsed = parse(&record, &strings, 0, record.len() as u32, 0).unwrap();
        assert_eq!(parsed.severity(), Severity::Contraindicated);
        assert_eq!(parsed.source(), "FDA Label");
        assert_eq!(parsed.claims().len(), 2);
        assert!(parsed
            .claims()
            .iter()
            .any(|c| c.source_tier() == SourceTier::CuratedDatabase));
    }

    #[test]
    fn rejects_a_declared_length_that_does_not_match_the_actual_length() {
        let mut strings = Vec::new();
        let claim = encode_claim(&mut strings, "x", "x", 0, 0, 0, 0, 2026, 1, 1, "r");
        let record = encode_record(1, 1, 2, &mut strings, std::slice::from_ref(&claim));
        let err = parse(&record, &strings, 0, record.len() as u32 + 1, 0).unwrap_err();
        assert!(matches!(
            err,
            DbError::InteractionRecordLengthMismatch { .. }
        ));
    }

    #[test]
    fn rejects_a_drug_paired_with_itself() {
        let mut strings = Vec::new();
        let claim = encode_claim(&mut strings, "x", "x", 0, 0, 0, 0, 2026, 1, 1, "r");
        let record = encode_record(1, 5, 5, &mut strings, std::slice::from_ref(&claim));
        assert_eq!(
            parse(&record, &strings, 0, record.len() as u32, 3).unwrap_err(),
            DbError::CorruptPair { index: 3 }
        );
    }
}

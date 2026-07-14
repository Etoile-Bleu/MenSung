//! Reads the fixed 28-byte Claim encoding shared by Interaction Records and
//! Drug Fact Records, as defined in docs/DATABASE_FORMAT.md's Claim
//! Encoding section. Used identically by both, so this logic lives in one
//! place rather than being duplicated the way `mensung-domain`'s
//! `InteractionFact`/`DrugFact` deliberately duplicate their
//! `primary_claim` logic (a claim's binary shape has no equivalent reason
//! to vary between the two).
//!
//! `ClaimRecord` stays zero-copy and `Copy`, the same style as
//! `DrugRecord`, rather than constructing an owned `mensung_domain::Claim`:
//! every text field is a borrow into the String Table. The calendar date is
//! still validated on read (`ClaimDate::new` is a cheap, non-allocating
//! check), but the source id and rationale are only checked for valid
//! UTF-8, not re-validated against `mensung_domain`'s stricter slug/
//! non-empty rules, the same lighter level of validation `drug_table.rs`
//! already applies to a drug's name.

use mensung_domain::{ClaimDate, Confidence, EvidenceLevel, Severity, SourceTier};

use crate::bytes::{read_u16, read_u32};
use crate::DbError;

pub(crate) const CLAIM_LEN: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimRecord<'a> {
    source_id: &'a str,
    source_name: &'a str,
    tier: SourceTier,
    severity: Severity,
    evidence: EvidenceLevel,
    confidence: Confidence,
    last_updated: ClaimDate,
    rationale: &'a str,
}

impl<'a> ClaimRecord<'a> {
    pub fn source_id(&self) -> &'a str {
        self.source_id
    }

    pub fn source_name(&self) -> &'a str {
        self.source_name
    }

    pub fn source_tier(&self) -> SourceTier {
        self.tier
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn evidence(&self) -> EvidenceLevel {
        self.evidence
    }

    pub fn confidence(&self) -> Confidence {
        self.confidence
    }

    pub fn last_updated(&self) -> ClaimDate {
        self.last_updated
    }

    pub fn rationale(&self) -> &'a str {
        self.rationale
    }
}

fn parse_tier(byte: u8) -> Result<SourceTier, DbError> {
    match byte {
        0 => Ok(SourceTier::Regulatory),
        1 => Ok(SourceTier::ClinicalGuideline),
        2 => Ok(SourceTier::CuratedDatabase),
        3 => Ok(SourceTier::Secondary),
        value => Err(DbError::InvalidSourceTier(value)),
    }
}

pub(crate) fn parse_severity(byte: u8) -> Result<Severity, DbError> {
    match byte {
        0 => Ok(Severity::Contraindicated),
        1 => Ok(Severity::HighRisk),
        2 => Ok(Severity::Moderate),
        3 => Ok(Severity::Minor),
        4 => Ok(Severity::Unknown),
        value => Err(DbError::InvalidSeverity(value)),
    }
}

pub(crate) fn parse_evidence(byte: u8) -> Result<EvidenceLevel, DbError> {
    match byte {
        0 => Ok(EvidenceLevel::Established),
        1 => Ok(EvidenceLevel::Probable),
        2 => Ok(EvidenceLevel::Theoretical),
        value => Err(DbError::InvalidEvidence(value)),
    }
}

fn parse_confidence(byte: u8) -> Result<Confidence, DbError> {
    match byte {
        0 => Ok(Confidence::Low),
        1 => Ok(Confidence::Medium),
        2 => Ok(Confidence::High),
        value => Err(DbError::InvalidConfidence(value)),
    }
}

fn string_at(strings: &[u8], offset: u32, len: u16) -> Result<&str, DbError> {
    let start = offset as usize;
    let end = start + len as usize;
    let bytes = strings.get(start..end).ok_or(DbError::Truncated)?;
    std::str::from_utf8(bytes).map_err(|_| DbError::InvalidStringTableEntry)
}

fn string_at_u32(strings: &[u8], offset: u32, len: u32) -> Result<&str, DbError> {
    let start = offset as usize;
    let end = start + len as usize;
    let bytes = strings.get(start..end).ok_or(DbError::Truncated)?;
    std::str::from_utf8(bytes).map_err(|_| DbError::InvalidStringTableEntry)
}

/// Parses one 28-byte Claim entry starting at `bytes[0..28]`, resolving
/// every string field against `strings` (the String Table).
pub(crate) fn parse<'a>(bytes: &[u8], strings: &'a [u8]) -> Result<ClaimRecord<'a>, DbError> {
    let record = bytes.get(..CLAIM_LEN).ok_or(DbError::Truncated)?;

    let source_id_offset = read_u32(record, 0)?;
    let source_id_len = read_u16(record, 4)?;
    let source_name_offset = read_u32(record, 6)?;
    let source_name_len = read_u16(record, 10)?;
    let tier = parse_tier(*record.get(12).ok_or(DbError::Truncated)?)?;
    let severity = parse_severity(*record.get(13).ok_or(DbError::Truncated)?)?;
    let evidence = parse_evidence(*record.get(14).ok_or(DbError::Truncated)?)?;
    let confidence = parse_confidence(*record.get(15).ok_or(DbError::Truncated)?)?;
    let year = read_u16(record, 16)?;
    let month = *record.get(18).ok_or(DbError::Truncated)?;
    let day = *record.get(19).ok_or(DbError::Truncated)?;
    let rationale_offset = read_u32(record, 20)?;
    let rationale_len = read_u32(record, 24)?;

    let source_id = string_at(strings, source_id_offset, source_id_len)?;
    let source_name = string_at(strings, source_name_offset, source_name_len)?;
    let rationale = string_at_u32(strings, rationale_offset, rationale_len)?;
    let last_updated =
        ClaimDate::new(year, month, day).map_err(|_| DbError::InvalidStringTableEntry)?;

    Ok(ClaimRecord {
        source_id,
        source_name,
        tier,
        severity,
        evidence,
        confidence,
        last_updated,
        rationale,
    })
}

/// Picks the claim to treat as authoritative from a non-empty slice: the
/// claim from the most trusted source tier present, the most severe of
/// them if more than one claim shares that tier. Mirrors
/// `mensung_domain::InteractionFact::primary_claim`'s tie-break rule
/// exactly, duplicated here for the same reason that accessor's own logic
/// is duplicated between `InteractionFact` and `DrugFact`: this is a
/// lighter, zero-copy `ClaimRecord`, not the owned domain `Claim`, so it
/// cannot reuse that method directly.
pub(crate) fn primary_claim<'a>(claims: &[ClaimRecord<'a>]) -> ClaimRecord<'a> {
    let top_tier = claims
        .iter()
        .map(|claim| claim.source_tier())
        .min()
        .expect("claims is non-empty by construction: a record with zero claims is rejected before this is called");

    *claims
        .iter()
        .filter(|claim| claim.source_tier() == top_tier)
        .min_by_key(|claim| claim.severity())
        .expect("at least one claim has tier == top_tier by construction")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Encodes one Claim's fixed fields and appends its three strings
    /// (source id, source name, rationale) to `strings`, returning the
    /// 28-byte record. Mirrors what `mensung-builder`'s writer does, used
    /// here to build fixtures for this module's own tests.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn encode_claim(
        strings: &mut Vec<u8>,
        source_id: &str,
        source_name: &str,
        tier: u8,
        severity: u8,
        evidence: u8,
        confidence: u8,
        year: u16,
        month: u8,
        day: u8,
        rationale: &str,
    ) -> Vec<u8> {
        let source_id_offset = strings.len() as u32;
        strings.extend_from_slice(source_id.as_bytes());
        let source_name_offset = strings.len() as u32;
        strings.extend_from_slice(source_name.as_bytes());
        let rationale_offset = strings.len() as u32;
        strings.extend_from_slice(rationale.as_bytes());

        let mut record = Vec::with_capacity(CLAIM_LEN);
        record.extend_from_slice(&source_id_offset.to_le_bytes());
        record.extend_from_slice(&(source_id.len() as u16).to_le_bytes());
        record.extend_from_slice(&source_name_offset.to_le_bytes());
        record.extend_from_slice(&(source_name.len() as u16).to_le_bytes());
        record.push(tier);
        record.push(severity);
        record.push(evidence);
        record.push(confidence);
        record.extend_from_slice(&year.to_le_bytes());
        record.push(month);
        record.push(day);
        record.extend_from_slice(&rationale_offset.to_le_bytes());
        record.extend_from_slice(&(rationale.len() as u32).to_le_bytes());
        assert_eq!(record.len(), CLAIM_LEN);
        record
    }

    #[test]
    fn parses_a_well_formed_claim() {
        let mut strings = Vec::new();
        let record = encode_claim(
            &mut strings,
            "ddinter",
            "DDInter",
            2,
            0,
            0,
            2,
            2026,
            7,
            14,
            "Increased bleeding risk.",
        );
        let claim = parse(&record, &strings).unwrap();
        assert_eq!(claim.source_id(), "ddinter");
        assert_eq!(claim.source_name(), "DDInter");
        assert_eq!(claim.source_tier(), SourceTier::CuratedDatabase);
        assert_eq!(claim.severity(), Severity::Contraindicated);
        assert_eq!(claim.evidence(), EvidenceLevel::Established);
        assert_eq!(claim.confidence(), Confidence::High);
        assert_eq!(claim.rationale(), "Increased bleeding risk.");
        assert_eq!(claim.last_updated(), ClaimDate::new(2026, 7, 14).unwrap());
    }

    #[test]
    fn rejects_an_unrecognized_tier_byte() {
        let mut strings = Vec::new();
        let record = encode_claim(&mut strings, "x", "x", 200, 0, 0, 0, 2026, 1, 1, "r");
        assert_eq!(
            parse(&record, &strings).unwrap_err(),
            DbError::InvalidSourceTier(200)
        );
    }

    #[test]
    fn rejects_an_unrecognized_confidence_byte() {
        let mut strings = Vec::new();
        let record = encode_claim(&mut strings, "x", "x", 0, 0, 0, 200, 2026, 1, 1, "r");
        assert_eq!(
            parse(&record, &strings).unwrap_err(),
            DbError::InvalidConfidence(200)
        );
    }

    #[test]
    fn rejects_an_impossible_calendar_date() {
        let mut strings = Vec::new();
        let record = encode_claim(&mut strings, "x", "x", 0, 0, 0, 0, 2026, 2, 30, "r");
        assert_eq!(
            parse(&record, &strings).unwrap_err(),
            DbError::InvalidStringTableEntry
        );
    }

    #[test]
    fn rejects_a_string_reference_outside_the_string_table() {
        let record = encode_claim(&mut Vec::new(), "x", "x", 0, 0, 0, 0, 2026, 1, 1, "r");
        assert_eq!(parse(&record, &[]).unwrap_err(), DbError::Truncated);
    }

    #[test]
    fn primary_claim_prefers_the_most_authoritative_tier() {
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
            "minor",
        );
        let fda = encode_claim(
            &mut strings,
            "fda",
            "FDA Label",
            0,
            0,
            0,
            2,
            2026,
            1,
            1,
            "contraindicated",
        );
        let claims = vec![
            parse(&ddinter, &strings).unwrap(),
            parse(&fda, &strings).unwrap(),
        ];
        let primary = primary_claim(&claims);
        assert_eq!(primary.source_id(), "fda");
        assert_eq!(primary.severity(), Severity::Contraindicated);
    }

    #[test]
    fn primary_claim_breaks_a_same_tier_tie_toward_more_severe() {
        let mut strings = Vec::new();
        let fda = encode_claim(
            &mut strings,
            "fda",
            "FDA",
            0,
            2,
            0,
            1,
            2026,
            1,
            1,
            "moderate",
        );
        let ema = encode_claim(
            &mut strings,
            "ema",
            "EMA",
            0,
            0,
            0,
            1,
            2026,
            1,
            1,
            "contraindicated",
        );
        let claims = vec![
            parse(&fda, &strings).unwrap(),
            parse(&ema, &strings).unwrap(),
        ];
        assert_eq!(primary_claim(&claims).severity(), Severity::Contraindicated);
    }
}

//! A single source's assertion about a clinical fact: how severe it says
//! the fact is, how strong the evidence is, how confident the source
//! itself is, its supporting rationale, and when it was last confirmed.
//! `InteractionFact` and `DrugFact` hold a collection of these, one per
//! contributing source, and never collapse or discard one in favor of
//! another; see `interaction_fact.rs` for how a "resolved" view is derived
//! without losing any of them.

use crate::{ClaimDate, Confidence, DomainError, EvidenceLevel, Severity, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    source: Source,
    severity: Severity,
    evidence: EvidenceLevel,
    confidence: Confidence,
    rationale: String,
    last_updated: ClaimDate,
}

impl Claim {
    pub fn new(
        source: Source,
        severity: Severity,
        evidence: EvidenceLevel,
        confidence: Confidence,
        rationale: impl Into<String>,
        last_updated: ClaimDate,
    ) -> Result<Self, DomainError> {
        let rationale = rationale.into();
        if rationale.trim().is_empty() {
            return Err(DomainError::EmptyRationale(source.id().to_string()));
        }

        Ok(Self {
            source,
            severity,
            evidence,
            confidence,
            rationale,
            last_updated,
        })
    }

    pub fn source(&self) -> &Source {
        &self.source
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

    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    pub fn last_updated(&self) -> ClaimDate {
        self.last_updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceId;
    use crate::SourceTier;

    fn ddinter_source() -> Source {
        Source::new(
            SourceId::parse("ddinter").unwrap(),
            "DDInter",
            SourceTier::CuratedDatabase,
        )
        .unwrap()
    }

    #[test]
    fn builds_a_well_formed_claim() {
        let claim = Claim::new(
            ddinter_source(),
            Severity::Contraindicated,
            EvidenceLevel::Established,
            Confidence::High,
            "Increased bleeding risk.",
            ClaimDate::new(2026, 7, 14).unwrap(),
        )
        .unwrap();

        assert_eq!(claim.severity(), Severity::Contraindicated);
        assert_eq!(claim.confidence(), Confidence::High);
        assert_eq!(claim.rationale(), "Increased bleeding risk.");
    }

    #[test]
    fn rejects_an_empty_rationale() {
        let err = Claim::new(
            ddinter_source(),
            Severity::Contraindicated,
            EvidenceLevel::Established,
            Confidence::High,
            "   ",
            ClaimDate::new(2026, 7, 14).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, DomainError::EmptyRationale("ddinter".to_string()));
    }
}

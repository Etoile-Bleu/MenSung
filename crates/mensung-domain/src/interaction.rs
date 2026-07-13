//! A drug-drug interaction: the unordered pair of drugs involved, its
//! clinical severity, and the evidence backing it. `DrugPair` canonicalizes
//! its two ids so that the pair (Aspirin, Warfarin) and the pair (Warfarin,
//! Aspirin) always compare and hash equal, which is what the lookup index
//! in mensung-db relies on.

use crate::{DomainError, DrugId, EvidenceLevel, InteractionId, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrugPair {
    lower: DrugId,
    higher: DrugId,
}

impl DrugPair {
    pub fn new(a: DrugId, b: DrugId) -> Result<Self, DomainError> {
        if a == b {
            return Err(DomainError::SelfInteraction(a));
        }
        let (lower, higher) = if a < b { (a, b) } else { (b, a) };
        Ok(Self { lower, higher })
    }

    pub fn drugs(&self) -> (DrugId, DrugId) {
        (self.lower, self.higher)
    }

    pub fn contains(&self, id: DrugId) -> bool {
        self.lower == id || self.higher == id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interaction {
    id: InteractionId,
    pair: DrugPair,
    severity: Severity,
    description: String,
    evidence: EvidenceLevel,
    source: String,
}

impl Interaction {
    pub fn new(
        id: InteractionId,
        pair: DrugPair,
        severity: Severity,
        description: impl Into<String>,
        evidence: EvidenceLevel,
        source: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let description = description.into();
        if description.trim().is_empty() {
            return Err(DomainError::EmptyDescription(id));
        }

        let source = source.into();
        if source.trim().is_empty() {
            return Err(DomainError::EmptySource(id));
        }

        Ok(Self {
            id,
            pair,
            severity,
            description,
            evidence,
            source,
        })
    }

    pub fn id(&self) -> InteractionId {
        self.id
    }

    pub fn pair(&self) -> DrugPair {
        self.pair
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn evidence(&self) -> EvidenceLevel {
        self.evidence
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_interaction(pair: DrugPair) -> Result<Interaction, DomainError> {
        Interaction::new(
            InteractionId::new(1),
            pair,
            Severity::Contraindicated,
            "Increased bleeding and hemorrhage probability.",
            EvidenceLevel::Established,
            "WHO drug interaction reference",
        )
    }

    #[test]
    fn a_drug_cannot_interact_with_itself() {
        let id = DrugId::new(1);
        assert_eq!(
            DrugPair::new(id, id).unwrap_err(),
            DomainError::SelfInteraction(id)
        );
    }

    #[test]
    fn pair_order_does_not_affect_equality() {
        let a = DrugId::new(1);
        let b = DrugId::new(2);
        assert_eq!(DrugPair::new(a, b).unwrap(), DrugPair::new(b, a).unwrap());
    }

    #[test]
    fn pair_contains_both_of_its_drugs() {
        let a = DrugId::new(1);
        let b = DrugId::new(2);
        let pair = DrugPair::new(a, b).unwrap();
        assert!(pair.contains(a));
        assert!(pair.contains(b));
        assert!(!pair.contains(DrugId::new(3)));
    }

    #[test]
    fn rejects_an_empty_description() {
        let pair = DrugPair::new(DrugId::new(1), DrugId::new(2)).unwrap();
        let err = Interaction::new(
            InteractionId::new(1),
            pair,
            Severity::Contraindicated,
            "   ",
            EvidenceLevel::Established,
            "WHO drug interaction reference",
        )
        .unwrap_err();
        assert_eq!(err, DomainError::EmptyDescription(InteractionId::new(1)));
    }

    #[test]
    fn rejects_an_empty_source() {
        let pair = DrugPair::new(DrugId::new(1), DrugId::new(2)).unwrap();
        let err = Interaction::new(
            InteractionId::new(1),
            pair,
            Severity::Contraindicated,
            "Increased bleeding risk.",
            EvidenceLevel::Established,
            "",
        )
        .unwrap_err();
        assert_eq!(err, DomainError::EmptySource(InteractionId::new(1)));
    }

    #[test]
    fn accepts_a_well_formed_interaction() {
        let pair = DrugPair::new(DrugId::new(1), DrugId::new(2)).unwrap();
        let interaction = sample_interaction(pair).unwrap();
        assert_eq!(interaction.severity(), Severity::Contraindicated);
        assert_eq!(interaction.pair(), pair);
    }
}

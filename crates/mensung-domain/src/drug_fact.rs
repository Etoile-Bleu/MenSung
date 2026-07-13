//! A fact about a single drug, as opposed to `InteractionFact`'s facts
//! about a pair: a contraindication, a boxed warning, pregnancy or
//! breastfeeding guidance, dosage, or an approved indication. Modeled the
//! same way as `InteractionFact`, one or more claims, none discarded when
//! sources disagree, because label data (DailyMed, OpenFDA) attaches these
//! to a single drug rather than to a drug pair the way DDInter's
//! interaction data does.

use crate::{Claim, DomainError, DrugFactId, DrugId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrugFactKind {
    Contraindication,
    Warning,
    BoxedWarning,
    Pregnancy,
    Breastfeeding,
    Dosage,
    Indication,
}

impl std::fmt::Display for DrugFactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            DrugFactKind::Contraindication => "Contraindication",
            DrugFactKind::Warning => "Warning",
            DrugFactKind::BoxedWarning => "Boxed warning",
            DrugFactKind::Pregnancy => "Pregnancy",
            DrugFactKind::Breastfeeding => "Breastfeeding",
            DrugFactKind::Dosage => "Dosage",
            DrugFactKind::Indication => "Indication",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrugFact {
    id: DrugFactId,
    drug: DrugId,
    kind: DrugFactKind,
    claims: Vec<Claim>,
}

impl DrugFact {
    pub fn new(
        id: DrugFactId,
        drug: DrugId,
        kind: DrugFactKind,
        claims: Vec<Claim>,
    ) -> Result<Self, DomainError> {
        if claims.is_empty() {
            return Err(DomainError::NoClaimsForDrugFact(id));
        }

        Ok(Self {
            id,
            drug,
            kind,
            claims,
        })
    }

    pub fn id(&self) -> DrugFactId {
        self.id
    }

    pub fn drug(&self) -> DrugId {
        self.drug
    }

    pub fn kind(&self) -> DrugFactKind {
        self.kind
    }

    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    /// The claim from the most trusted source tier present; the most
    /// severe of them if more than one claim shares that tier. Same
    /// resolution rule as `InteractionFact::primary_claim`, kept
    /// consistent across both fact types deliberately.
    pub fn primary_claim(&self) -> &Claim {
        let top_tier = self
            .claims
            .iter()
            .map(|claim| claim.source().tier())
            .min()
            .expect("claims is non-empty, enforced by DrugFact::new");

        self.claims
            .iter()
            .filter(|claim| claim.source().tier() == top_tier)
            .min_by_key(|claim| claim.severity())
            .expect("at least one claim has tier == top_tier by construction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClaimDate, Confidence, EvidenceLevel, Severity, Source, SourceId, SourceTier};

    fn source(id: &str, tier: SourceTier) -> Source {
        Source::new(SourceId::parse(id).unwrap(), id, tier).unwrap()
    }

    fn claim(source: Source, severity: Severity) -> Claim {
        Claim::new(
            source,
            severity,
            EvidenceLevel::Established,
            Confidence::High,
            "rationale",
            ClaimDate::new(2026, 7, 14).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn rejects_zero_claims() {
        let err = DrugFact::new(
            DrugFactId::new(0),
            DrugId::new(0),
            DrugFactKind::Contraindication,
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, DomainError::NoClaimsForDrugFact(DrugFactId::new(0)));
    }

    #[test]
    fn boxed_warning_from_a_regulatory_source_outranks_a_curated_database_claim() {
        let openfda = source("openfda-label", SourceTier::CuratedDatabase);
        let fda = source("fda-label", SourceTier::Regulatory);
        let fact = DrugFact::new(
            DrugFactId::new(0),
            DrugId::new(0),
            DrugFactKind::BoxedWarning,
            vec![
                claim(openfda, Severity::Moderate),
                claim(fda, Severity::Contraindicated),
            ],
        )
        .unwrap();

        assert_eq!(fact.primary_claim().source().id().as_str(), "fda-label");
        assert_eq!(fact.claims().len(), 2);
    }

    #[test]
    fn kind_displays_a_human_readable_label() {
        assert_eq!(DrugFactKind::BoxedWarning.to_string(), "Boxed warning");
        assert_eq!(
            DrugFactKind::Contraindication.to_string(),
            "Contraindication"
        );
    }
}

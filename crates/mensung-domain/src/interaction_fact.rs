//! An interaction between two drugs as asserted by one or more sources.
//! Every claim from every source is kept; nothing is ever silently
//! discarded when sources disagree, per MEDICAL_DATA_POLICY.md. `resolve`
//! derives the single-severity `Interaction` the current `.men` format and
//! CLI/TUI display still use, without deleting the other claims: they stay
//! reachable through `claims()` for anything built against the richer
//! multi-source model later.

use crate::{Claim, DomainError, DrugPair, Interaction, InteractionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionFact {
    id: InteractionId,
    pair: DrugPair,
    claims: Vec<Claim>,
}

impl InteractionFact {
    pub fn new(id: InteractionId, pair: DrugPair, claims: Vec<Claim>) -> Result<Self, DomainError> {
        if claims.is_empty() {
            return Err(DomainError::NoClaimsForInteraction(id));
        }

        Ok(Self { id, pair, claims })
    }

    pub fn id(&self) -> InteractionId {
        self.id
    }

    pub fn pair(&self) -> DrugPair {
        self.pair
    }

    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    /// The claim to treat as authoritative: among the claims from the most
    /// trusted source tier present, the most severe one. Ties within a
    /// tier are broken toward severity rather than arbitrarily, so two
    /// equally authoritative sources that disagree can never resolve to
    /// the milder reading, per the zero false negative policy in
    /// MEDICAL_DATA_POLICY.md. This is always one of the real claims,
    /// never a synthesized value.
    pub fn primary_claim(&self) -> &Claim {
        let top_tier = self
            .claims
            .iter()
            .map(|claim| claim.source().tier())
            .min()
            .expect("claims is non-empty, enforced by InteractionFact::new");

        self.claims
            .iter()
            .filter(|claim| claim.source().tier() == top_tier)
            .min_by_key(|claim| claim.severity())
            .expect("at least one claim has tier == top_tier by construction")
    }

    /// Collapses this fact down to the single-severity `Interaction` shape
    /// the current `.men` format compiles, using `primary_claim`.
    pub fn resolve(&self) -> Interaction {
        let primary = self.primary_claim();
        Interaction::new(
            self.id,
            self.pair,
            primary.severity(),
            primary.rationale().to_string(),
            primary.evidence(),
            primary.source().name().to_string(),
        )
        .expect("a valid Claim's fields already satisfy Interaction::new's invariants")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ClaimDate, Confidence, DrugId, EvidenceLevel, Severity, Source, SourceId, SourceTier,
    };

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

    fn pair() -> DrugPair {
        DrugPair::new(DrugId::new(0), DrugId::new(1)).unwrap()
    }

    #[test]
    fn rejects_zero_claims() {
        let err = InteractionFact::new(InteractionId::new(0), pair(), vec![]).unwrap_err();
        assert_eq!(
            err,
            DomainError::NoClaimsForInteraction(InteractionId::new(0))
        );
    }

    #[test]
    fn a_single_claim_is_its_own_primary_claim() {
        let ddinter = source("ddinter", SourceTier::CuratedDatabase);
        let fact = InteractionFact::new(
            InteractionId::new(0),
            pair(),
            vec![claim(ddinter, Severity::Moderate)],
        )
        .unwrap();
        assert_eq!(fact.primary_claim().severity(), Severity::Moderate);
    }

    #[test]
    fn a_regulatory_claim_outranks_a_curated_database_claim() {
        let ddinter = source("ddinter", SourceTier::CuratedDatabase);
        let fda = source("fda-label", SourceTier::Regulatory);
        let fact = InteractionFact::new(
            InteractionId::new(0),
            pair(),
            vec![
                claim(ddinter, Severity::Minor),
                claim(fda, Severity::Contraindicated),
            ],
        )
        .unwrap();

        let primary = fact.primary_claim();
        assert_eq!(primary.source().id().as_str(), "fda-label");
        assert_eq!(primary.severity(), Severity::Contraindicated);
    }

    #[test]
    fn two_equally_authoritative_claims_resolve_to_the_more_severe_one() {
        let fda = source("fda-label", SourceTier::Regulatory);
        let ema = source("ema-label", SourceTier::Regulatory);
        let fact = InteractionFact::new(
            InteractionId::new(0),
            pair(),
            vec![
                claim(fda, Severity::Moderate),
                claim(ema, Severity::Contraindicated),
            ],
        )
        .unwrap();

        assert_eq!(fact.primary_claim().severity(), Severity::Contraindicated);
    }

    #[test]
    fn resolve_produces_an_interaction_from_the_primary_claim_without_losing_other_claims() {
        let ddinter = source("ddinter", SourceTier::CuratedDatabase);
        let fda = source("fda-label", SourceTier::Regulatory);
        let fact = InteractionFact::new(
            InteractionId::new(0),
            pair(),
            vec![
                claim(ddinter, Severity::Moderate),
                claim(fda, Severity::Contraindicated),
            ],
        )
        .unwrap();

        let resolved = fact.resolve();
        assert_eq!(resolved.severity(), Severity::Contraindicated);
        assert_eq!(resolved.source(), "fda-label");

        // Both claims are still there; resolving never deletes evidence.
        assert_eq!(fact.claims().len(), 2);
    }
}

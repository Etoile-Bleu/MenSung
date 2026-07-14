//! Wraps DDInter's single-claim `Interaction` values as single-claim
//! `InteractionFact`s, the shape the .men format v2 writer needs (see
//! docs/DATABASE_FORMAT.md). Kept separate from `ddinter.rs` deliberately:
//! `ddinter.rs`'s own import logic and tests are entirely unaffected by
//! the .men format's evolution, and this adapter is the one place that
//! would need to change if a future source ever needed to combine with
//! DDInter's claims for the same interaction pair.

use mensung_domain::{
    Claim, ClaimDate, Confidence, Interaction, InteractionFact, Source, SourceId, SourceTier,
};

use crate::ddinter::DDINTER_SOURCE;

pub const DDINTER_SOURCE_ID: &str = "ddinter";

pub fn ddinter_source() -> Source {
    Source::new(
        SourceId::parse(DDINTER_SOURCE_ID).expect("DDINTER_SOURCE_ID is a valid slug literal"),
        DDINTER_SOURCE,
        SourceTier::CuratedDatabase,
    )
    .expect("DDINTER_SOURCE is a valid, non-empty literal")
}

/// DDInter's public bulk export was captured for this project's mirror on
/// this date (see MEDICAL_DATA_POLICY.md's Data Sources section, GitHub
/// Release `ddinter-mirror-2025-08-30`); used as every DDInter claim's
/// `last_updated` date, since that is when this project last confirmed
/// the data against DDInter, not merely when a given build happened to
/// run.
fn ddinter_claim_date() -> ClaimDate {
    ClaimDate::new(2025, 8, 30).expect("2025-08-30 is a valid calendar date")
}

/// Wraps each DDInter `Interaction` as an `InteractionFact` carrying
/// exactly one claim, sourced as DDInter with `Confidence::Medium`: like
/// OpenFDA's label data, DDInter's severity levels are a curated
/// third-party aggregation, not primary regulatory review, so this does
/// not claim more confidence than that.
pub fn wrap_as_claims(interactions: Vec<Interaction>) -> Vec<InteractionFact> {
    let source = ddinter_source();

    interactions
        .into_iter()
        .map(|interaction| {
            let claim = Claim::new(
                source.clone(),
                interaction.severity(),
                interaction.evidence(),
                Confidence::Medium,
                interaction.description().to_string(),
                ddinter_claim_date(),
            )
            .expect("an Interaction's fields already satisfy Claim::new's invariants");

            InteractionFact::new(interaction.id(), interaction.pair(), vec![claim])
                .expect("a single valid claim already satisfies InteractionFact::new's invariants")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::{DrugId, DrugPair, EvidenceLevel, InteractionId, Severity};

    #[test]
    fn wraps_an_interaction_into_a_single_ddinter_claim() {
        let interaction = Interaction::new(
            InteractionId::new(0),
            DrugPair::new(DrugId::new(0), DrugId::new(1)).unwrap(),
            Severity::Contraindicated,
            "Increased bleeding risk.",
            EvidenceLevel::Established,
            DDINTER_SOURCE,
        )
        .unwrap();

        let facts = wrap_as_claims(vec![interaction]);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].claims().len(), 1);

        let claim = &facts[0].claims()[0];
        assert_eq!(claim.source().id().as_str(), DDINTER_SOURCE_ID);
        assert_eq!(claim.severity(), Severity::Contraindicated);
        assert_eq!(claim.confidence(), Confidence::Medium);
        assert_eq!(claim.rationale(), "Increased bleeding risk.");
    }
}

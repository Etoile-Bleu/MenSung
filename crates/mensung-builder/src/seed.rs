//! A small, hand-authored bootstrap dataset: five drugs and three
//! well-established textbook interactions. This exists only so
//! mensung-client has a real database to embed and test against before the
//! real OpenFDA/RxNorm/WHO importers (ROADMAP.md Phase 5, still open) exist.
//! Do not treat this as clinically exhaustive or as the shipped dataset;
//! MEDICAL_DATA_POLICY.md's real dataset lands in Phase 11.

use mensung_domain::{
    DomainError, Drug, DrugId, DrugPair, EvidenceLevel, InnName, Interaction, InteractionId,
    Severity,
};

const BOOTSTRAP_SOURCE: &str =
    "Standard pharmacology reference (bootstrap seed, not yet sourced from OpenFDA/RxNorm/WHO)";

pub fn seed_dataset() -> Result<(Vec<Drug>, Vec<Interaction>), DomainError> {
    let aspirin = DrugId::new(0);
    let warfarin = DrugId::new(1);
    let paracetamol = DrugId::new(2);
    let amoxicillin = DrugId::new(3);
    let ibuprofen = DrugId::new(4);

    let drugs = vec![
        Drug::new(aspirin, InnName::parse("Aspirin")?),
        Drug::new(warfarin, InnName::parse("Warfarin")?),
        Drug::new(paracetamol, InnName::parse("Paracetamol")?),
        Drug::new(amoxicillin, InnName::parse("Amoxicillin")?),
        Drug::new(ibuprofen, InnName::parse("Ibuprofen")?),
    ];

    let interactions = vec![
        Interaction::new(
            InteractionId::new(0),
            DrugPair::new(aspirin, warfarin)?,
            Severity::Contraindicated,
            "Increased bleeding and hemorrhage probability.",
            EvidenceLevel::Established,
            BOOTSTRAP_SOURCE,
        )?,
        Interaction::new(
            InteractionId::new(1),
            DrugPair::new(warfarin, amoxicillin)?,
            Severity::Moderate,
            "Amoxicillin may potentiate warfarin's anticoagulant effect, increasing INR.",
            EvidenceLevel::Established,
            BOOTSTRAP_SOURCE,
        )?,
        Interaction::new(
            InteractionId::new(2),
            DrugPair::new(aspirin, ibuprofen)?,
            Severity::Moderate,
            "Ibuprofen may reduce aspirin's antiplatelet cardioprotective effect if taken before it.",
            EvidenceLevel::Established,
            BOOTSTRAP_SOURCE,
        )?,
    ];

    Ok((drugs, interactions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_dataset_is_internally_valid() {
        let (drugs, interactions) = seed_dataset().unwrap();
        assert_eq!(drugs.len(), 5);
        assert_eq!(interactions.len(), 3);
        assert!(crate::validate::validate(&drugs, &interactions).is_empty());
    }

    #[test]
    fn the_seed_dataset_compiles_and_self_verifies() {
        let (drugs, interactions) = seed_dataset().unwrap();
        let (bytes, report) = crate::build_database(drugs, interactions).unwrap();
        assert_eq!(report.errors, 0);
        assert_eq!(report.interactions, 3);
        assert!(mensung_db::Database::open(&bytes).is_ok());
    }
}

//! Cross-record validation for a dataset before it is compiled into a .men
//! file: duplicate drug names, duplicate interaction pairs, and
//! interactions or drug facts referencing a drug that is not in the
//! dataset. Per-record invariants (INN name format, non-empty rationale,
//! no self-interaction, at least one claim) are already enforced by
//! mensung-domain's constructors; this only checks things a single record
//! cannot know about itself.

use std::collections::{HashMap, HashSet};

use mensung_domain::{Drug, DrugFact, DrugId, InteractionFact};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    DuplicateDrugName {
        name: String,
        first: DrugId,
        duplicate: DrugId,
    },
    DuplicateInteractionPair {
        drug_a: DrugId,
        drug_b: DrugId,
    },
    UnknownDrugInInteraction {
        drug: DrugId,
    },
    UnknownDrugInDrugFact {
        drug: DrugId,
    },
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationIssue::DuplicateDrugName {
                name,
                first,
                duplicate,
            } => write!(
                f,
                "duplicate drug name '{name}': already assigned to {first:?}, also claimed by {duplicate:?}"
            ),
            ValidationIssue::DuplicateInteractionPair { drug_a, drug_b } => {
                write!(f, "duplicate interaction for pair ({drug_a:?}, {drug_b:?})")
            }
            ValidationIssue::UnknownDrugInInteraction { drug } => write!(
                f,
                "interaction references drug {drug:?}, which is not in the drug list"
            ),
            ValidationIssue::UnknownDrugInDrugFact { drug } => write!(
                f,
                "drug fact references drug {drug:?}, which is not in the drug list"
            ),
        }
    }
}

pub fn validate(
    drugs: &[Drug],
    interactions: &[InteractionFact],
    drug_facts: &[DrugFact],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    let mut seen_names: HashMap<&str, DrugId> = HashMap::new();
    let mut known_ids: HashSet<DrugId> = HashSet::new();
    for drug in drugs {
        known_ids.insert(drug.id());
        if let Some(&first) = seen_names.get(drug.inn_name().as_str()) {
            issues.push(ValidationIssue::DuplicateDrugName {
                name: drug.inn_name().as_str().to_string(),
                first,
                duplicate: drug.id(),
            });
        } else {
            seen_names.insert(drug.inn_name().as_str(), drug.id());
        }
    }

    let mut seen_pairs: HashSet<(DrugId, DrugId)> = HashSet::new();
    for fact in interactions {
        let (a, b) = fact.pair().drugs();
        if !known_ids.contains(&a) {
            issues.push(ValidationIssue::UnknownDrugInInteraction { drug: a });
        }
        if !known_ids.contains(&b) {
            issues.push(ValidationIssue::UnknownDrugInInteraction { drug: b });
        }
        if !seen_pairs.insert((a, b)) {
            issues.push(ValidationIssue::DuplicateInteractionPair {
                drug_a: a,
                drug_b: b,
            });
        }
    }

    for fact in drug_facts {
        if !known_ids.contains(&fact.drug()) {
            issues.push(ValidationIssue::UnknownDrugInDrugFact { drug: fact.drug() });
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::{
        Claim, ClaimDate, Confidence, DrugFactId, DrugFactKind, DrugPair, EvidenceLevel, InnName,
        InteractionId, Severity, Source, SourceId, SourceTier,
    };

    fn drug(id: u32, name: &str) -> Drug {
        Drug::new(DrugId::new(id), InnName::parse(name).unwrap())
    }

    fn source() -> Source {
        Source::new(
            SourceId::parse("test-source").unwrap(),
            "Test Source",
            SourceTier::CuratedDatabase,
        )
        .unwrap()
    }

    fn claim(severity: Severity) -> Claim {
        Claim::new(
            source(),
            severity,
            EvidenceLevel::Established,
            Confidence::Medium,
            "rationale",
            ClaimDate::new(2026, 7, 14).unwrap(),
        )
        .unwrap()
    }

    fn interaction(id: u32, a: u32, b: u32) -> InteractionFact {
        InteractionFact::new(
            InteractionId::new(id),
            DrugPair::new(DrugId::new(a), DrugId::new(b)).unwrap(),
            vec![claim(Severity::Moderate)],
        )
        .unwrap()
    }

    fn drug_fact(id: u32, drug_id: u32) -> DrugFact {
        DrugFact::new(
            DrugFactId::new(id),
            DrugId::new(drug_id),
            DrugFactKind::Warning,
            vec![claim(Severity::Moderate)],
        )
        .unwrap()
    }

    #[test]
    fn a_clean_dataset_has_no_issues() {
        let drugs = vec![drug(0, "Aspirin"), drug(1, "Warfarin")];
        let interactions = vec![interaction(0, 0, 1)];
        assert!(validate(&drugs, &interactions, &[]).is_empty());
    }

    #[test]
    fn flags_a_duplicate_drug_name() {
        let drugs = vec![drug(0, "Aspirin"), drug(1, "Aspirin")];
        let issues = validate(&drugs, &[], &[]);
        assert_eq!(
            issues,
            vec![ValidationIssue::DuplicateDrugName {
                name: "Aspirin".to_string(),
                first: DrugId::new(0),
                duplicate: DrugId::new(1),
            }]
        );
    }

    #[test]
    fn flags_a_duplicate_interaction_pair() {
        let drugs = vec![drug(0, "Aspirin"), drug(1, "Warfarin")];
        let interactions = vec![interaction(0, 0, 1), interaction(1, 1, 0)];
        let issues = validate(&drugs, &interactions, &[]);
        assert_eq!(
            issues,
            vec![ValidationIssue::DuplicateInteractionPair {
                drug_a: DrugId::new(0),
                drug_b: DrugId::new(1),
            }]
        );
    }

    #[test]
    fn flags_an_interaction_referencing_an_unknown_drug() {
        let drugs = vec![drug(0, "Aspirin")];
        let interactions = vec![interaction(0, 0, 99)];
        let issues = validate(&drugs, &interactions, &[]);
        assert_eq!(
            issues,
            vec![ValidationIssue::UnknownDrugInInteraction {
                drug: DrugId::new(99)
            }]
        );
    }

    #[test]
    fn flags_a_drug_fact_referencing_an_unknown_drug() {
        let drugs = vec![drug(0, "Aspirin")];
        let drug_facts = vec![drug_fact(0, 99)];
        let issues = validate(&drugs, &[], &drug_facts);
        assert_eq!(
            issues,
            vec![ValidationIssue::UnknownDrugInDrugFact {
                drug: DrugId::new(99)
            }]
        );
    }

    #[test]
    fn a_drug_fact_referencing_a_known_drug_has_no_issues() {
        let drugs = vec![drug(0, "Warfarin")];
        let drug_facts = vec![drug_fact(0, 0)];
        assert!(validate(&drugs, &[], &drug_facts).is_empty());
    }
}

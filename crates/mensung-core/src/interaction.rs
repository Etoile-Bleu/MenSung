//! Checks every pairwise interaction among a set of confirmed drugs. Every
//! pair present in the database is returned, sorted most severe first; none
//! are dropped, per the zero false negative policy in
//! MEDICAL_DATA_POLICY.md.

use mensung_db::{Database, InteractionRecord};
use mensung_domain::{DrugId, DrugPair};

use crate::CoreError;

pub fn check_interactions<'a>(
    db: &Database<'a>,
    drugs: &[DrugId],
) -> Result<Vec<InteractionRecord<'a>>, CoreError> {
    if drugs.len() < 2 {
        return Err(CoreError::NotEnoughDrugs);
    }

    let mut found = Vec::new();
    for i in 0..drugs.len() {
        for j in (i + 1)..drugs.len() {
            let pair = DrugPair::new(drugs[i], drugs[j])
                .map_err(|_| CoreError::DuplicateDrug(drugs[i]))?;
            if let Some(record) = db.find_interaction(pair)? {
                found.push(record);
            }
        }
    }

    found.sort_by_key(|record| record.severity());
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_db::test_support::{build_men_file, TestDrug, TestInteraction};
    use mensung_domain::{EvidenceLevel, Severity};

    fn three_drug_database() -> Vec<u8> {
        build_men_file(
            vec![
                TestDrug {
                    id: 0,
                    name: "Aspirin",
                },
                TestDrug {
                    id: 1,
                    name: "Warfarin",
                },
                TestDrug {
                    id: 2,
                    name: "Paracetamol",
                },
            ],
            &[
                TestInteraction {
                    id: 0,
                    drug_a: 0,
                    drug_b: 1,
                    severity: Severity::Contraindicated,
                    description: "Increased bleeding and hemorrhage probability.",
                    evidence: EvidenceLevel::Established,
                    source: "WHO drug interaction reference",
                },
                TestInteraction {
                    id: 1,
                    drug_a: 1,
                    drug_b: 2,
                    severity: Severity::Minor,
                    description: "Rare, clinically minor effect on anticoagulation.",
                    evidence: EvidenceLevel::Theoretical,
                    source: "OpenFDA",
                },
            ],
        )
    }

    #[test]
    fn rejects_fewer_than_two_drugs() {
        let bytes = three_drug_database();
        let db = Database::open(&bytes).unwrap();
        let err = check_interactions(&db, &[DrugId::new(0)]).unwrap_err();
        assert_eq!(err, CoreError::NotEnoughDrugs);
    }

    #[test]
    fn rejects_the_same_drug_listed_twice() {
        let bytes = three_drug_database();
        let db = Database::open(&bytes).unwrap();
        let err = check_interactions(&db, &[DrugId::new(0), DrugId::new(0)]).unwrap_err();
        assert_eq!(err, CoreError::DuplicateDrug(DrugId::new(0)));
    }

    #[test]
    fn finds_every_pairwise_interaction_among_three_drugs() {
        let bytes = three_drug_database();
        let db = Database::open(&bytes).unwrap();
        let drugs = [DrugId::new(0), DrugId::new(1), DrugId::new(2)];
        let found = check_interactions(&db, &drugs).unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn most_severe_interaction_is_returned_first() {
        let bytes = three_drug_database();
        let db = Database::open(&bytes).unwrap();
        let drugs = [DrugId::new(0), DrugId::new(1), DrugId::new(2)];
        let found = check_interactions(&db, &drugs).unwrap();
        assert_eq!(found[0].severity(), Severity::Contraindicated);
        assert_eq!(found[1].severity(), Severity::Minor);
    }

    #[test]
    fn a_pair_with_no_known_interaction_is_simply_absent_not_an_error() {
        let bytes = three_drug_database();
        let db = Database::open(&bytes).unwrap();
        let drugs = [DrugId::new(0), DrugId::new(2)];
        let found = check_interactions(&db, &drugs).unwrap();
        assert!(found.is_empty());
    }
}

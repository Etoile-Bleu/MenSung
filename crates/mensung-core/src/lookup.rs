//! Resolves a user-typed drug name against the database: an exact match
//! returns immediately, anything else returns ranked fuzzy candidates for
//! confirmation. There is no third path where the engine guesses and
//! proceeds, per the no-silent-correction rule in MEDICAL_DATA_POLICY.md.

use mensung_db::{Database, DrugRecord};

use crate::fuzzy;
use crate::{Candidate, CoreError};

#[derive(Debug, Clone, PartialEq)]
pub enum LookupOutcome<'a> {
    ExactMatch(DrugRecord<'a>),
    Candidates(Vec<Candidate<'a>>),
    NoMatch,
}

pub fn lookup_drug<'a>(db: &Database<'a>, query: &str) -> Result<LookupOutcome<'a>, CoreError> {
    if let Some(drug) = db.find_drug_by_name(query)? {
        return Ok(LookupOutcome::ExactMatch(drug));
    }

    let candidates = fuzzy::rank_candidates(db, query)?;
    if candidates.is_empty() {
        Ok(LookupOutcome::NoMatch)
    } else {
        Ok(LookupOutcome::Candidates(candidates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_db::test_support::{build_men_file, TestDrug};

    fn database_with(names: &[(&'static str, u32)]) -> Vec<u8> {
        let drugs = names
            .iter()
            .map(|&(name, id)| TestDrug { id, name })
            .collect();
        build_men_file(drugs, &[])
    }

    #[test]
    fn an_exact_name_resolves_immediately() {
        let bytes = database_with(&[("Warfarin", 0)]);
        let db = Database::open(&bytes).unwrap();
        let outcome = lookup_drug(&db, "Warfarin").unwrap();
        assert!(matches!(outcome, LookupOutcome::ExactMatch(drug) if drug.name() == "Warfarin"));
    }

    #[test]
    fn a_typo_returns_candidates_instead_of_guessing() {
        let bytes = database_with(&[("Amoxicillin", 0)]);
        let db = Database::open(&bytes).unwrap();
        let outcome = lookup_drug(&db, "Amoxilin").unwrap();
        match outcome {
            LookupOutcome::Candidates(candidates) => {
                assert_eq!(candidates[0].drug().name(), "Amoxicillin");
            }
            other => panic!("expected Candidates, got {other:?}"),
        }
    }

    #[test]
    fn a_completely_unknown_name_returns_no_match() {
        let bytes = database_with(&[("Amoxicillin", 0)]);
        let db = Database::open(&bytes).unwrap();
        let outcome = lookup_drug(&db, "Zzzzzzzzzzzz").unwrap();
        assert_eq!(outcome, LookupOutcome::NoMatch);
    }
}

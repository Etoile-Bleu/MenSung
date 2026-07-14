//! Ranks every drug name in the database against a typed query by
//! Jaro-Winkler similarity, from the `strsim` crate. This is deliberately a
//! spelling-correction metric, not a fuzzy-finder/substring filter: MenSung
//! needs a confidence score for "did you mean Amoxicillin", the same shape
//! as a spell checker, not an interactive fuzzy-find experience. That is why
//! `strsim` fits this requirement better than a fuzzy-finder crate such as
//! `nucleo`, which is built for filtering a list as the user types rather
//! than scoring a single typo against a closed vocabulary.
//!
//! This runs only on the fallback path, after an exact match has already
//! missed, so the allocations here (case normalization) are an accepted
//! cost outside the `<5ms` exact-lookup budget that governs the hot path.

use mensung_db::Database;

use crate::{Candidate, CoreError};

const MAX_CANDIDATES: usize = 5;
const MIN_SIMILARITY: f32 = 0.6;

pub(crate) fn rank_candidates<'a>(
    db: &Database<'a>,
    query: &str,
) -> Result<Vec<Candidate<'a>>, CoreError> {
    let normalized_query = query.to_lowercase();
    let mut scored = Vec::new();

    for drug in db.drugs() {
        let drug = drug?;
        let normalized_name = drug.name().to_lowercase();
        let similarity = strsim::jaro_winkler(&normalized_query, &normalized_name) as f32;
        if similarity >= MIN_SIMILARITY {
            scored.push(Candidate::new(drug, similarity));
        }
    }

    scored.sort_by(|a, b| {
        b.similarity()
            .partial_cmp(&a.similarity())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(MAX_CANDIDATES);

    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_db::test_support::{build_men_file, TestDrug};

    fn database_with(names: &[(&'static str, u32)]) -> Vec<u8> {
        let drugs = names
            .iter()
            .map(|&(name, id)| TestDrug::plain(id, name))
            .collect();
        build_men_file(drugs, &[], &[])
    }

    #[test]
    fn common_misspellings_of_amoxicillin_rank_it_as_the_top_candidate() {
        let bytes = database_with(&[
            ("Amoxicillin", 0),
            ("Aspirin", 1),
            ("Warfarin", 2),
            ("Paracetamol", 3),
        ]);
        let db = Database::open(&bytes).unwrap();

        for typo in ["Amoxilin", "Amoxicilin", "Amoxycillin"] {
            let candidates = rank_candidates(&db, typo).unwrap();
            assert!(
                !candidates.is_empty(),
                "expected at least one candidate for {typo}"
            );
            assert_eq!(
                candidates[0].drug().name(),
                "Amoxicillin",
                "expected Amoxicillin to be the top candidate for {typo}"
            );
        }
    }

    #[test]
    fn candidates_never_include_an_exact_input_match_score_of_one_for_a_typo() {
        let bytes = database_with(&[("Amoxicillin", 0)]);
        let db = Database::open(&bytes).unwrap();
        let candidates = rank_candidates(&db, "Amoxilin").unwrap();
        assert!(candidates[0].similarity() < 1.0);
    }

    #[test]
    fn an_unrelated_query_returns_no_candidates() {
        let bytes = database_with(&[("Amoxicillin", 0), ("Warfarin", 1)]);
        let db = Database::open(&bytes).unwrap();
        let candidates = rank_candidates(&db, "Xyzzyplugh").unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidate_list_is_capped_and_sorted_most_similar_first() {
        let names = [
            ("Amoxicillin", 0u32),
            ("Amoxapine", 1),
            ("Amoxapina", 2),
            ("Amoxiclav", 3),
            ("Amoxidin", 4),
            ("Amoxifen", 5),
            ("Amoxinol", 6),
        ];
        let bytes = database_with(&names);
        let db = Database::open(&bytes).unwrap();
        let candidates = rank_candidates(&db, "Amoxilin").unwrap();

        assert!(candidates.len() <= MAX_CANDIDATES);
        for window in candidates.windows(2) {
            assert!(window[0].similarity() >= window[1].similarity());
        }
    }
}

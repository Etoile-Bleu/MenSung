//! Golden medical test suite: checks the shipped dataset against
//! tests/golden_cases.json at the workspace root. A build that drops or
//! weakens one of these cases fails here, per MEDICAL_DATA_POLICY.md and
//! ROADMAP.md Phase 8. This runs against the bootstrap seed dataset today;
//! it will point at the real dataset once ROADMAP.md Phase 11 lands, and
//! this file does not need to change when that happens, only the dataset
//! `build_database` is given.

use mensung_db::Database;
use mensung_domain::{DrugPair, Severity};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenCase {
    drug_a: String,
    drug_b: String,
    expected_severity: Option<String>,
}

fn parse_severity(label: &str) -> Severity {
    match label {
        "Contraindicated" => Severity::Contraindicated,
        "HighRisk" => Severity::HighRisk,
        "Moderate" => Severity::Moderate,
        "Minor" => Severity::Minor,
        "Unknown" => Severity::Unknown,
        other => panic!("golden_cases.json: unrecognized severity label '{other}'"),
    }
}

#[test]
fn shipped_dataset_matches_golden_cases() {
    let (drugs, interactions) = mensung_builder::seed_dataset().unwrap();
    let (bytes, _report) = mensung_builder::build_database(drugs, interactions).unwrap();
    let db = Database::open(&bytes).unwrap();

    let golden_json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden_cases.json"
    ));
    let cases: Vec<GoldenCase> = serde_json::from_str(golden_json).unwrap();
    assert!(!cases.is_empty(), "golden_cases.json must not be empty");

    for case in &cases {
        let drug_a = db
            .find_drug_by_name(&case.drug_a)
            .unwrap()
            .unwrap_or_else(|| {
                panic!(
                    "golden case drug '{}' was not found in the dataset under test",
                    case.drug_a
                )
            });
        let drug_b = db
            .find_drug_by_name(&case.drug_b)
            .unwrap()
            .unwrap_or_else(|| {
                panic!(
                    "golden case drug '{}' was not found in the dataset under test",
                    case.drug_b
                )
            });

        let pair = DrugPair::new(drug_a.id(), drug_b.id())
            .expect("golden_cases.json must not pair a drug with itself");
        let found = db.find_interaction(pair).unwrap();

        match (&case.expected_severity, found) {
            (None, None) => {}
            (None, Some(interaction)) => panic!(
                "golden case expects no interaction between {} and {}, but found {}",
                case.drug_a,
                case.drug_b,
                interaction.severity()
            ),
            (Some(expected), None) => panic!(
                "golden case expects a {expected} interaction between {} and {}, but none was found -- this is exactly the zero false negative violation this suite exists to catch",
                case.drug_a, case.drug_b
            ),
            (Some(expected), Some(interaction)) => {
                let expected_severity = parse_severity(expected);
                assert_eq!(
                    interaction.severity(),
                    expected_severity,
                    "golden case {} + {}: expected {expected_severity}, found {}",
                    case.drug_a,
                    case.drug_b,
                    interaction.severity()
                );
            }
        }
    }
}

//! Golden medical test suite: checks a dataset against tests/golden_cases.json
//! at the workspace root. A build that drops or weakens one of these cases
//! fails here, per MEDICAL_DATA_POLICY.md and ROADMAP.md Phase 8. There is
//! no dataset embedded in the workspace to check by default: the real
//! DDInter dataset is installed by `mensung-client` at runtime, not present
//! during `cargo test`. This builds a small fixture matching
//! golden_cases.json's entries exactly, so what this test actually proves
//! is that the checking machinery (this file plus mensung-builder and
//! mensung-db) correctly validates a dataset against the golden cases; it
//! does not by itself prove the real installed dataset passes. Run this
//! same check by hand against a real `.men` file before a release.

use mensung_db::Database;
use mensung_domain::{
    Drug, DrugId, DrugPair, EvidenceLevel, InnName, Interaction, InteractionId, Severity,
};
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

/// A fixture covering exactly the pairs tests/golden_cases.json exercises,
/// including one pair (Paracetamol + Amoxicillin) deliberately left out of
/// the interaction list, since a golden case can also assert that no
/// interaction is reported.
fn fixture_dataset() -> (Vec<Drug>, Vec<Interaction>) {
    let aspirin = DrugId::new(0);
    let warfarin = DrugId::new(1);
    let paracetamol = DrugId::new(2);
    let amoxicillin = DrugId::new(3);
    let ibuprofen = DrugId::new(4);

    let drugs = vec![
        Drug::new(aspirin, InnName::parse("Aspirin").unwrap()),
        Drug::new(warfarin, InnName::parse("Warfarin").unwrap()),
        Drug::new(paracetamol, InnName::parse("Paracetamol").unwrap()),
        Drug::new(amoxicillin, InnName::parse("Amoxicillin").unwrap()),
        Drug::new(ibuprofen, InnName::parse("Ibuprofen").unwrap()),
    ];

    let interactions = vec![
        Interaction::new(
            InteractionId::new(0),
            DrugPair::new(aspirin, warfarin).unwrap(),
            Severity::Contraindicated,
            "Increased bleeding and hemorrhage probability.",
            EvidenceLevel::Established,
            "golden_cases.json fixture",
        )
        .unwrap(),
        Interaction::new(
            InteractionId::new(1),
            DrugPair::new(warfarin, amoxicillin).unwrap(),
            Severity::Moderate,
            "Amoxicillin may potentiate warfarin's anticoagulant effect, increasing INR.",
            EvidenceLevel::Established,
            "golden_cases.json fixture",
        )
        .unwrap(),
        Interaction::new(
            InteractionId::new(2),
            DrugPair::new(aspirin, ibuprofen).unwrap(),
            Severity::Moderate,
            "Ibuprofen may reduce aspirin's antiplatelet cardioprotective effect.",
            EvidenceLevel::Established,
            "golden_cases.json fixture",
        )
        .unwrap(),
    ];

    (drugs, interactions)
}

#[test]
fn fixture_dataset_matches_golden_cases() {
    let (drugs, interactions) = fixture_dataset();
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

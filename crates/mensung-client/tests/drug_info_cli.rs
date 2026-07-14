//! Proves `mensung info <drug-name>` shows a single drug's cross-reference
//! data and drug-specific facts by running the real compiled `mensung`
//! binary against a hand-built database, matching the same
//! real-binary-verification practice as `tests/multi_claim_display.rs`.

use std::process::Command;

use mensung_db::test_support::{build_men_file, TestClaim, TestDrug, TestDrugFact};
use mensung_domain::{DrugFactKind, EvidenceLevel, Severity};

fn boxed_warning_claim() -> TestClaim {
    TestClaim {
        source_id: "openfda-label",
        source_name: "OpenFDA Drug Labeling",
        tier: 2,
        severity: Severity::HighRisk,
        evidence: EvidenceLevel::Established,
        confidence: 1,
        year: 2026,
        month: 7,
        day: 14,
        rationale: "Warfarin sodium can cause major or fatal bleeding.",
    }
}

fn richly_enriched_database() -> Vec<u8> {
    build_men_file(
        vec![TestDrug {
            id: 0,
            name: "Warfarin",
            rxcui: Some("11289"),
            pubchem_cid: Some(54678486),
            molecular_formula: Some("C19H16O4"),
            molecular_weight: Some("308.3"),
            iupac_name: None,
            atc_codes: vec![("B01AA", "Vitamin K antagonists")],
        }],
        &[],
        &[TestDrugFact {
            id: 0,
            drug: 0,
            kind: DrugFactKind::BoxedWarning,
            claims: vec![boxed_warning_claim()],
        }],
    )
}

fn write_database(dir_name: &str, bytes: Vec<u8>) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("medical_database.men"), bytes).unwrap();
    dir
}

#[test]
fn info_shows_cross_reference_data_and_drug_facts() {
    let dir = write_database("mensung-drug-info-cli-test", richly_enriched_database());

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["info", "Warfarin"])
        .output()
        .expect("running the real mensung binary should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Warfarin"), "stdout was:\n{stdout}");
    assert!(stdout.contains("RxCUI: 11289"), "stdout was:\n{stdout}");
    assert!(
        stdout.contains("ATC: B01AA (Vitamin K antagonists)"),
        "stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("Chemical: C19H16O4, 308.3 g/mol"),
        "stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("Boxed warning") || stdout.contains("HIGH RISK"),
        "stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("Warfarin sodium can cause major or fatal bleeding."),
        "stdout was:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn info_on_a_drug_with_no_extra_data_says_so_plainly() {
    let dir = write_database(
        "mensung-drug-info-cli-plain-test",
        build_men_file(vec![TestDrug::plain(0, "Aspirin")], &[], &[]),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["info", "Aspirin"])
        .output()
        .expect("running the real mensung binary should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No additional reference information"),
        "stdout was:\n{stdout}"
    );
    assert!(!stdout.contains("RxCUI"), "stdout was:\n{stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn info_with_no_argument_prints_usage_and_exits_with_code_2() {
    let dir = write_database(
        "mensung-drug-info-cli-usage-test",
        richly_enriched_database(),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["info"])
        .output()
        .expect("running the real mensung binary should succeed");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage: mensung info"),
        "stderr was:\n{stderr}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn info_on_an_unknown_drug_reports_no_match() {
    let dir = write_database(
        "mensung-drug-info-cli-unknown-test",
        richly_enriched_database(),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["info", "Zzzzzzzzzzzz"])
        .output()
        .expect("running the real mensung binary should succeed");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Unknown drug"), "stdout was:\n{stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

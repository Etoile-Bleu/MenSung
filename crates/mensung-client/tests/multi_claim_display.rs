//! Proves the CLI actually prints every claim on a multi-source
//! interaction, not just the resolved primary one, by running the real
//! compiled `mensung` binary against a hand-built multi-claim `.men`
//! file. Verifying this against the real binary rather than only the
//! underlying data (already covered by mensung-db's own tests) matches
//! this project's established practice for the CLI/TUI layer: ROADMAP.md
//! Phase 7 notes a real terminal catching a display bug the unit tests
//! missed.

use std::process::Command;

use mensung_db::test_support::{build_men_file, TestClaim, TestDrug, TestInteraction};
use mensung_domain::{EvidenceLevel, Severity};

fn multi_claim_database() -> Vec<u8> {
    build_men_file(
        vec![TestDrug::plain(0, "Aspirin"), TestDrug::plain(1, "Warfarin")],
        &[TestInteraction {
            id: 0,
            drug_a: 0,
            drug_b: 1,
            claims: vec![
                TestClaim {
                    source_id: "ddinter",
                    source_name: "DDInter",
                    tier: 2,
                    severity: Severity::Minor,
                    evidence: EvidenceLevel::Established,
                    confidence: 1,
                    year: 2025,
                    month: 8,
                    day: 30,
                    rationale: "DDInter classifies this as a minor interaction.",
                },
                TestClaim {
                    source_id: "fda-label",
                    source_name: "FDA Label",
                    tier: 0,
                    severity: Severity::Contraindicated,
                    evidence: EvidenceLevel::Established,
                    confidence: 2,
                    year: 2026,
                    month: 1,
                    day: 1,
                    rationale: "Warfarin sodium can cause major or fatal bleeding when combined with aspirin.",
                },
            ],
        }],
        &[],
    )
}

#[test]
fn cli_output_includes_every_claim_not_just_the_resolved_one() {
    let dir = std::env::temp_dir().join("mensung-multi-claim-display-test");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("medical_database.men"), multi_claim_database()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["Aspirin", "Warfarin"])
        .output()
        .expect("running the real mensung binary should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The primary (most authoritative tier, FDA Label) claim is the
    // headline result, unchanged from before this feature existed.
    assert!(stdout.contains("CONTRAINDICATED"), "stdout was:\n{stdout}");
    assert!(
        stdout.contains(
            "Warfarin sodium can cause major or fatal bleeding when combined with aspirin."
        ),
        "stdout was:\n{stdout}"
    );

    // The other, less authoritative claim (DDInter) must still be
    // visible, not silently dropped now that a primary claim is chosen.
    assert!(
        stdout.contains("Also reported by:"),
        "stdout was:\n{stdout}"
    );
    assert!(stdout.contains("DDInter"), "stdout was:\n{stdout}");
    assert!(
        stdout.contains("DDInter classifies this as a minor interaction."),
        "stdout was:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cli_output_omits_the_also_reported_by_section_for_a_single_claim_interaction() {
    let dir = std::env::temp_dir().join("mensung-single-claim-display-test");
    std::fs::create_dir_all(&dir).unwrap();
    let bytes = build_men_file(
        vec![
            TestDrug::plain(0, "Aspirin"),
            TestDrug::plain(1, "Warfarin"),
        ],
        &[TestInteraction::simple(
            0,
            0,
            1,
            Severity::Contraindicated,
            EvidenceLevel::Established,
            "DDInter",
            "Increased bleeding risk.",
        )],
        &[],
    );
    std::fs::write(dir.join("medical_database.men"), bytes).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["Aspirin", "Warfarin"])
        .output()
        .expect("running the real mensung binary should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Also reported by:"),
        "a single-claim interaction should not show the multi-source section, stdout was:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

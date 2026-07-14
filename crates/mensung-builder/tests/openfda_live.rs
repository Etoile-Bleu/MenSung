//! Proves `openfda.rs` and `openfda_download.rs` work together against the
//! real, live openFDA API, not just the captured fixture the unit tests
//! use. Ignored by default: this makes real HTTPS requests, so it must
//! not run as part of `cargo test --workspace` in CI. Run explicitly with
//! `cargo test -p mensung-builder --test openfda_live -- --ignored`.

use mensung_domain::{Drug, DrugFactKind, DrugId, InnName, Severity};

#[test]
#[ignore = "hits the live openFDA API"]
fn a_real_batch_of_ddinter_drugs_produces_real_drug_facts() {
    // A handful of real drug names from DDInter's dataset (see
    // MEDICAL_DATA_POLICY.md's Data Sources section), well known enough to
    // reliably have a current FDA label.
    let names = [
        "Warfarin",
        "Aspirin",
        "Metformin",
        "Ibuprofen",
        "Amoxicillin",
    ];
    let drugs: Vec<Drug> = names
        .iter()
        .enumerate()
        .map(|(index, name)| Drug::new(DrugId::new(index as u32), InnName::parse(name).unwrap()))
        .collect();

    let request_names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    let bodies = mensung_builder::fetch_all_openfda_labels(&request_names)
        .expect("live openFDA requests should succeed");
    assert!(
        !bodies.is_empty(),
        "at least one of these well-known drugs should have an openFDA label"
    );

    let facts = mensung_builder::import_openfda_labels(&bodies, &drugs)
        .expect("real openFDA responses should parse and import cleanly");
    assert!(
        !facts.is_empty(),
        "importing real label responses for well-known drugs should produce at least one fact"
    );

    let warfarin_id = DrugId::new(0);
    let warfarin_facts: Vec<_> = facts
        .iter()
        .filter(|fact| fact.drug() == warfarin_id)
        .collect();
    assert!(
        !warfarin_facts.is_empty(),
        "warfarin has a well-known FDA label and should produce at least one fact"
    );
    assert!(
        warfarin_facts
            .iter()
            .any(|fact| fact.kind() == DrugFactKind::BoxedWarning
                || fact.kind() == DrugFactKind::Contraindication),
        "warfarin's real FDA label carries a boxed warning and a pregnancy contraindication; \
         neither showing up means the field mapping or matching logic regressed"
    );

    for fact in &facts {
        let primary = fact.primary_claim();
        assert!(
            !primary.rationale().trim().is_empty(),
            "{:?} fact for {:?} has an empty rationale",
            fact.kind(),
            fact.drug()
        );
        assert_eq!(
            primary.source().id().as_str(),
            mensung_builder::OPENFDA_SOURCE_ID
        );
        assert_ne!(
            primary.severity(),
            Severity::Unknown,
            "openfda.rs assigns a fixed severity per DrugFactKind; Unknown should never appear"
        );
    }
}

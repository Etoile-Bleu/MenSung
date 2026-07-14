//! Proves `rxnorm.rs` and `rxnorm_download.rs` work together against the
//! real, live RxNorm API, not just captured fixtures. Ignored by default:
//! this makes real HTTPS requests, so it must not run as part of
//! `cargo test --workspace` in CI. Run explicitly with
//! `cargo test -p mensung-builder --test rxnorm_live -- --ignored`.

use mensung_domain::{Drug, DrugId, InnName};

#[test]
#[ignore = "hits the live RxNorm API"]
fn a_real_batch_of_ddinter_drugs_gets_real_rxcuis() {
    // A mix of a plain INN name and one of DDInter's real names with a
    // parenthetical qualifier (see MEDICAL_DATA_POLICY.md's Data Sources
    // section), to prove RxNorm's normalized search handles both without
    // this project needing its own fuzzy matching on top.
    let names = ["Warfarin", "Aspirin", "Dexamethasone (topical)"];
    let drugs: Vec<Drug> = names
        .iter()
        .enumerate()
        .map(|(index, name)| Drug::new(DrugId::new(index as u32), InnName::parse(name).unwrap()))
        .collect();

    let requests: Vec<(DrugId, String)> = drugs
        .iter()
        .map(|drug| (drug.id(), drug.inn_name().as_str().to_string()))
        .collect();

    let responses =
        mensung_builder::fetch_all_rxcuis(&requests).expect("live RxNorm requests should succeed");
    assert_eq!(responses.len(), names.len());

    let enriched = mensung_builder::attach_rxcuis(drugs, &responses)
        .expect("real responses should parse cleanly");

    for drug in &enriched {
        assert!(
            drug.rxcui().is_some(),
            "{:?} should have resolved to a real RxCUI",
            drug.inn_name().as_str()
        );
    }

    let warfarin = enriched
        .iter()
        .find(|drug| drug.inn_name().as_str() == "Warfarin")
        .expect("warfarin should be in the enriched list");
    assert_eq!(warfarin.rxcui().map(|rxcui| rxcui.as_str()), Some("11289"));
}

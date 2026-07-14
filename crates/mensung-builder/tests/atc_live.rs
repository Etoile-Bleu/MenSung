//! Proves `atc.rs` and `atc_download.rs` work together against the real,
//! live RxClass API, not just captured fixtures, chained through a real
//! RxNorm lookup first (ATC classification needs an RxCUI, see
//! `atc.rs`'s header). Ignored by default: this makes real HTTPS
//! requests, so it must not run as part of `cargo test --workspace` in
//! CI. Run explicitly with
//! `cargo test -p mensung-builder --test atc_live -- --ignored`.

use mensung_domain::{Drug, DrugId, InnName};

#[test]
#[ignore = "hits the live RxNorm and RxClass APIs"]
fn a_real_batch_of_ddinter_drugs_gets_real_atc_codes() {
    let names = ["Warfarin", "Aspirin"];
    let drugs: Vec<Drug> = names
        .iter()
        .enumerate()
        .map(|(index, name)| Drug::new(DrugId::new(index as u32), InnName::parse(name).unwrap()))
        .collect();

    let rxcui_requests: Vec<(DrugId, String)> = drugs
        .iter()
        .map(|drug| (drug.id(), drug.inn_name().as_str().to_string()))
        .collect();
    let rxcui_responses = mensung_builder::fetch_all_rxcuis(&rxcui_requests)
        .expect("live RxNorm requests should succeed");
    let drugs_with_rxcui = mensung_builder::attach_rxcuis(drugs, &rxcui_responses)
        .expect("real RxNorm responses should parse cleanly");

    let atc_requests: Vec<(DrugId, String)> = drugs_with_rxcui
        .iter()
        .filter_map(|drug| Some((drug.id(), drug.rxcui()?.as_str().to_string())))
        .collect();
    assert_eq!(
        atc_requests.len(),
        names.len(),
        "both warfarin and aspirin should have resolved an RxCUI already"
    );

    let atc_responses = mensung_builder::fetch_all_atc_codes(&atc_requests)
        .expect("live RxClass requests should succeed");
    let enriched = mensung_builder::attach_atc_codes(drugs_with_rxcui, &atc_responses)
        .expect("real RxClass responses should parse cleanly");

    let warfarin = enriched
        .iter()
        .find(|drug| drug.inn_name().as_str() == "Warfarin")
        .expect("warfarin should be in the enriched list");
    assert!(
        warfarin.atc_codes().iter().any(|atc| atc.code() == "B01AA"),
        "warfarin's real, stable ATC code is B01AA (Vitamin K antagonists)"
    );

    let aspirin = enriched
        .iter()
        .find(|drug| drug.inn_name().as_str() == "Aspirin")
        .expect("aspirin should be in the enriched list");
    assert!(
        aspirin.atc_codes().len() > 1,
        "aspirin is classified under more than one ATC code (analgesic and antiplatelet uses)"
    );
    assert!(
        aspirin
            .atc_codes()
            .iter()
            .all(|atc| atc.code() != "N02AJ"),
        "the aspirin/codeine combination product's ATC code must not leak into plain aspirin's list"
    );
}

//! Proves `pubchem.rs` and `pubchem_download.rs` work together against
//! the real, live PubChem API, not just captured fixtures. Ignored by
//! default: this makes real HTTPS requests, so it must not run as part
//! of `cargo test --workspace` in CI. Run explicitly with
//! `cargo test -p mensung-builder --test pubchem_live -- --ignored`.

use mensung_domain::{Drug, DrugId, InnName};

#[test]
#[ignore = "hits the live PubChem API"]
fn a_real_batch_of_ddinter_drugs_gets_real_chemical_properties() {
    let names = ["Warfarin", "Aspirin", "Metformin"];
    let drugs: Vec<Drug> = names
        .iter()
        .enumerate()
        .map(|(index, name)| Drug::new(DrugId::new(index as u32), InnName::parse(name).unwrap()))
        .collect();

    let requests: Vec<(DrugId, String)> = drugs
        .iter()
        .map(|drug| (drug.id(), drug.inn_name().as_str().to_string()))
        .collect();

    let responses = mensung_builder::fetch_all_pubchem_properties(&requests)
        .expect("live PubChem requests should succeed");
    assert_eq!(
        responses.len(),
        names.len(),
        "all three are common enough drugs to have a PubChem entry"
    );

    let enriched = mensung_builder::attach_chemical_properties(drugs, &responses)
        .expect("real responses should parse cleanly");

    for drug in &enriched {
        let props = drug
            .chemical_properties()
            .unwrap_or_else(|| panic!("{:?} should have chemical properties", drug.inn_name()));
        assert!(!props.molecular_formula().is_empty());
        assert!(
            props.molecular_weight().parse::<f64>().is_ok(),
            "molecular weight '{}' should parse as a number even though it is stored as a string",
            props.molecular_weight()
        );
    }

    let warfarin = enriched
        .iter()
        .find(|drug| drug.inn_name().as_str() == "Warfarin")
        .expect("warfarin should be in the enriched list");
    assert_eq!(
        warfarin.chemical_properties().unwrap().molecular_formula(),
        "C19H16O4"
    );
}

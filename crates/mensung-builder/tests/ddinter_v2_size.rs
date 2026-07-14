//! Proves the .men format v2's shared-string-table deduplication actually
//! fixes the size problem documented in ROADMAP.md's Phase 5 tradeoff
//! note: format v1 inlined DDInter's mostly-repeated synthesized
//! description and source text per record, producing a ~28MB file from
//! the real, full DDInter export. This downloads and compiles that same
//! real dataset through the v2 writer and checks the result is
//! dramatically smaller. Ignored by default: this makes real HTTPS
//! requests. Run explicitly with
//! `cargo test -p mensung-builder --test ddinter_v2_size -- --ignored --nocapture`.

#[test]
#[ignore = "downloads the real DDInter dataset over HTTPS"]
fn the_real_ddinter_dataset_compiles_to_a_much_smaller_v2_file() {
    let download_dir = std::env::temp_dir().join("mensung-ddinter-v2-size-test");
    let (drugs, interactions) = mensung_builder::download_and_import_ddinter(&download_dir)
        .expect("downloading and importing the real DDInter dataset should succeed");
    let drug_count = drugs.len();
    let interaction_count = interactions.len();

    let interactions = mensung_builder::wrap_as_claims(interactions);
    let (bytes, report) = mensung_builder::build_database(drugs, interactions, Vec::new())
        .expect("the real DDInter dataset should compile cleanly");

    assert_eq!(report.errors, 0);
    println!(
        "drugs: {drug_count}, interactions: {interaction_count}, .men v2 size: {} bytes ({:.1} MB)",
        bytes.len(),
        bytes.len() as f64 / (1024.0 * 1024.0)
    );

    // Format v1 produced ~28MB from this same dataset (see ROADMAP.md's
    // Phase 5 tradeoff note); v2's string-table deduplication should cut
    // that dramatically, since the synthesized description and source
    // text repeat across most of the 160,235+ records.
    assert!(
        bytes.len() < 10 * 1024 * 1024,
        "expected the deduplicated v2 file to be well under the old ~28MB, got {} bytes",
        bytes.len()
    );

    let db = mensung_db::Database::open(&bytes).expect("the compiled file should open cleanly");
    assert_eq!(db.drug_count() as usize, drug_count);
    assert_eq!(db.interaction_count() as usize, interaction_count);
}

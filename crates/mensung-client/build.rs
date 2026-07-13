//! Compiles the bootstrap seed dataset into a .men file at build time, so
//! main.rs can embed it with include_bytes!. This is what makes the
//! shipped binary genuinely self-contained: no external database file to
//! distribute or configure, matching README.md's single-embedded-binary
//! requirement. The seed dataset is a placeholder until the real
//! ROADMAP.md Phase 5 importers exist; see mensung_builder::seed_dataset.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (drugs, interactions) = mensung_builder::seed_dataset()?;
    let (bytes, report) = mensung_builder::build_database(drugs, interactions)?;

    let out_dir = std::env::var("OUT_DIR")?;
    let out_path = std::path::Path::new(&out_dir).join("medical_database.men");
    std::fs::write(&out_path, &bytes)?;

    println!(
        "cargo:warning=embedded bootstrap .men: {} interactions, {} errors, {} warnings",
        report.interactions, report.errors, report.warnings
    );
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

//! Builder CLI: runs `mensung-builder`'s full data pipeline (DDInter,
//! OpenFDA, RxNorm, PubChem, WHO ATC) and compiles the result into a
//! `.men` file. Meant to be run once, offline, by a maintainer or CI job,
//! with the output published as a release asset, not run per end-user
//! install: the enrichment sources' rate limits make a full run take
//! roughly an hour at DDInter's real ~1900-drug scale (OpenFDA alone,
//! paced at 40 requests/minute, is about 50 minutes by itself), a poor
//! fit for the low-connectivity field deployments MenSung targets. See
//! ROADMAP.md's Phase 8b for the full reasoning behind this being a
//! build-time tool, not a runtime one.
//!
//! Usage:
//!   mensung-builder build --out <path> [--skip-openfda] [--skip-rxnorm] [--skip-pubchem] [--skip-atc]
//!
//! `--skip-rxnorm` implies `--skip-atc`: WHO ATC classification is looked
//! up by RxCUI (see `atc.rs`'s header for why), so it has nothing to look
//! up once RxNorm is skipped.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(&args)
}

fn run(args: &[String]) -> ExitCode {
    if args.first().map(String::as_str) != Some("build") {
        print_usage();
        return ExitCode::from(2);
    }

    let mut out_path: Option<PathBuf> = None;
    let mut skip_openfda = false;
    let mut skip_rxnorm = false;
    let mut skip_pubchem = false;
    let mut skip_atc = false;

    let mut iter = args[1..].iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--out" => out_path = iter.next().map(PathBuf::from),
            "--skip-openfda" => skip_openfda = true,
            "--skip-rxnorm" => skip_rxnorm = true,
            "--skip-pubchem" => skip_pubchem = true,
            "--skip-atc" => skip_atc = true,
            other => {
                eprintln!("Unrecognized argument: {other}");
                print_usage();
                return ExitCode::from(2);
            }
        }
    }

    let Some(out_path) = out_path else {
        eprintln!("Missing required --out <path>");
        print_usage();
        return ExitCode::from(2);
    };

    if skip_rxnorm {
        skip_atc = true;
    }

    match build(&out_path, skip_openfda, skip_rxnorm, skip_pubchem, skip_atc) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Fatal: {message}");
            ExitCode::from(70)
        }
    }
}

fn print_usage() {
    eprintln!(
        "Usage: mensung-builder build --out <path> [--skip-openfda] [--skip-rxnorm] [--skip-pubchem] [--skip-atc]"
    );
    eprintln!();
    eprintln!("Runs the full data pipeline (DDInter, OpenFDA, RxNorm, PubChem, WHO ATC)");
    eprintln!("and compiles the result into a .men file at <path>. Meant to be run once,");
    eprintln!("offline, not per end-user install; see ROADMAP.md's Phase 8b.");
    eprintln!();
    eprintln!("--skip-rxnorm implies --skip-atc (ATC lookup needs RxNorm's RxCUI output).");
}

fn build(
    out_path: &Path,
    skip_openfda: bool,
    skip_rxnorm: bool,
    skip_pubchem: bool,
    skip_atc: bool,
) -> Result<(), String> {
    eprintln!("Downloading and importing DDInter...");
    let download_dir = std::env::temp_dir().join("mensung-builder-ddinter");
    let (mut drugs, interactions) = mensung_builder::download_and_import_ddinter(&download_dir)
        .map_err(|err| format!("DDInter import failed: {err}"))?;
    eprintln!(
        "  {} drugs, {} interactions",
        drugs.len(),
        interactions.len()
    );
    let interactions = mensung_builder::wrap_as_claims(interactions);

    let mut drug_facts = Vec::new();

    if skip_rxnorm {
        eprintln!("Skipping RxNorm (--skip-rxnorm).");
    } else {
        eprintln!(
            "Fetching RxCUIs from RxNorm for {} drugs (paced at 10 requests/second)...",
            drugs.len()
        );
        let requests: Vec<_> = drugs
            .iter()
            .map(|d| (d.id(), d.inn_name().as_str().to_string()))
            .collect();
        let responses = mensung_builder::fetch_all_rxcuis(&requests)
            .map_err(|err| format!("RxNorm fetch failed: {err}"))?;
        drugs = mensung_builder::attach_rxcuis(drugs, &responses)
            .map_err(|err| format!("RxNorm import failed: {err}"))?;
        let resolved = drugs.iter().filter(|d| d.rxcui().is_some()).count();
        eprintln!("  {resolved}/{} drugs resolved an RxCUI", drugs.len());

        if skip_atc {
            eprintln!("Skipping WHO ATC (--skip-atc).");
        } else {
            eprintln!("Fetching WHO ATC classification from RxClass...");
            let atc_requests: Vec<_> = drugs
                .iter()
                .filter_map(|d| Some((d.id(), d.rxcui()?.as_str().to_string())))
                .collect();
            let atc_responses = mensung_builder::fetch_all_atc_codes(&atc_requests)
                .map_err(|err| format!("ATC fetch failed: {err}"))?;
            drugs = mensung_builder::attach_atc_codes(drugs, &atc_responses)
                .map_err(|err| format!("ATC import failed: {err}"))?;
            let classified = drugs.iter().filter(|d| !d.atc_codes().is_empty()).count();
            eprintln!(
                "  {classified}/{} drugs got at least one ATC code",
                drugs.len()
            );
        }
    }

    if skip_pubchem {
        eprintln!("Skipping PubChem (--skip-pubchem).");
    } else {
        eprintln!(
            "Fetching chemical properties from PubChem for {} drugs (paced at 2 requests/second)...",
            drugs.len()
        );
        let requests: Vec<_> = drugs
            .iter()
            .map(|d| (d.id(), d.inn_name().as_str().to_string()))
            .collect();
        let responses = mensung_builder::fetch_all_pubchem_properties(&requests)
            .map_err(|err| format!("PubChem fetch failed: {err}"))?;
        drugs = mensung_builder::attach_chemical_properties(drugs, &responses)
            .map_err(|err| format!("PubChem import failed: {err}"))?;
        let resolved = drugs
            .iter()
            .filter(|d| d.chemical_properties().is_some())
            .count();
        eprintln!("  {resolved}/{} drugs got chemical properties", drugs.len());
    }

    if skip_openfda {
        eprintln!("Skipping OpenFDA (--skip-openfda).");
    } else {
        eprintln!(
            "Fetching drug labels from OpenFDA for {} drugs (paced at 40 requests/minute, the slowest step)...",
            drugs.len()
        );
        let names: Vec<String> = drugs
            .iter()
            .map(|d| d.inn_name().as_str().to_string())
            .collect();
        let responses = mensung_builder::fetch_all_openfda_labels(&names)
            .map_err(|err| format!("OpenFDA fetch failed: {err}"))?;
        drug_facts = mensung_builder::import_openfda_labels(&responses, &drugs)
            .map_err(|err| format!("OpenFDA import failed: {err}"))?;
        eprintln!("  {} drug facts produced", drug_facts.len());
    }

    eprintln!("Compiling .men file...");
    let (bytes, report) = mensung_builder::build_database(drugs, interactions, drug_facts)
        .map_err(|err| format!("build failed: {err}"))?;

    std::fs::write(out_path, &bytes)
        .map_err(|err| format!("could not write {}: {err}", out_path.display()))?;

    let report_path = out_path.with_file_name("validation-report.json");
    std::fs::write(&report_path, report.to_json())
        .map_err(|err| format!("could not write {}: {err}", report_path.display()))?;

    eprintln!(
        "Done: {} ({:.1} MB), {} interactions, {} errors, {} warnings",
        out_path.display(),
        bytes.len() as f64 / (1024.0 * 1024.0),
        report.interactions,
        report.errors,
        report.warnings,
    );

    Ok(())
}

//! Scriptable, non-interactive mode: two or more INN drug names as
//! arguments in, every known pairwise interaction out, most severe first.
//! Used whenever mensung is invoked with at least one argument; zero
//! arguments launches the TUI instead. `mensung info <drug-name>` is a
//! separate mode for a single drug's own facts (contraindications,
//! warnings, and so on) and cross-reference data (RxCUI, ATC
//! classification, chemical properties), not an interaction between two
//! drugs. This is an offline informational assistant; it does not
//! replace professional clinical judgement.
//!
//! Exit codes: 0 no known interaction found (or, for `info`, the drug
//! resolved successfully); 1 at least one interaction found, or a typed
//! name could not be resolved (both mean "look again before proceeding");
//! 2 bad command-line usage; 70 an internal or database error, the same
//! convention as the Unix EX_SOFTWARE sysexit.

use std::io::IsTerminal as _;
use std::process::ExitCode;

use crossterm::style::Stylize;
use mensung_core::{check_interactions, lookup_drug, CoreError, LookupOutcome};
use mensung_db::{Database, DbError, DrugRecord};
use mensung_domain::{DrugId, Severity};

use crate::DISCLAIMER;

#[derive(Clone, Copy)]
enum Tone {
    Danger,
    Warning,
    Ok,
    Dim,
    Bold,
}

fn styled(text: &str, tone: Tone) -> String {
    if !std::io::stdout().is_terminal() {
        return text.to_string();
    }
    match tone {
        Tone::Danger => text.red().bold().to_string(),
        Tone::Warning => text.yellow().to_string(),
        Tone::Ok => text.green().to_string(),
        Tone::Dim => text.dark_grey().to_string(),
        Tone::Bold => text.bold().to_string(),
    }
}

fn severity_tone(severity: Severity) -> Tone {
    match severity {
        Severity::Contraindicated | Severity::HighRisk => Tone::Danger,
        Severity::Moderate | Severity::Minor | Severity::Unknown => Tone::Warning,
    }
}

pub(crate) fn run(db: &Database, args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("Usage: mensung <drug-1> <drug-2> [<drug-3> ...]");
        eprintln!("Looks up known interactions among two or more drugs, by INN name.");
        eprintln!("Run mensung with no arguments for the interactive interface.");
        return ExitCode::from(2);
    }

    let resolved = match resolve_all(db, args) {
        Ok(resolved) => resolved,
        Err(outcome) => return outcome,
    };

    let ids: Vec<DrugId> = resolved.iter().map(DrugRecord::id).collect();
    let interactions = match check_interactions(db, &ids) {
        Ok(interactions) => interactions,
        Err(CoreError::Database(err)) => return fatal_database_error(&err),
        Err(err) => {
            eprintln!("Fatal: {err}");
            return ExitCode::from(70);
        }
    };

    if interactions.is_empty() {
        let names: Vec<&str> = resolved.iter().map(DrugRecord::name).collect();
        println!(
            "{}",
            styled(
                &format!("No known interaction among: {}", names.join(", ")),
                Tone::Ok
            )
        );
        println!("\n{DISCLAIMER}");
        return ExitCode::SUCCESS;
    }

    for interaction in &interactions {
        let (lower, higher) = interaction.pair().drugs();
        let name_of = |id: DrugId| -> &str {
            resolved
                .iter()
                .find(|drug| drug.id() == id)
                .map(DrugRecord::name)
                .expect("every id in this interaction came from resolved")
        };
        let severity = interaction.severity();
        let tone = severity_tone(severity);

        println!(
            "{}\n",
            styled(&format!("!!! {severity} INTERACTION !!!"), tone)
        );
        println!(
            "{}\n",
            styled(
                &format!("{} + {}", name_of(lower), name_of(higher)),
                Tone::Bold
            )
        );
        println!("Severity:\n{}\n", styled(&severity.to_string(), tone));
        println!("Risk:\n{}\n", interaction.description());
        println!(
            "{}\n",
            styled(
                &format!(
                    "Evidence: {} ({})",
                    interaction.evidence(),
                    interaction.source()
                ),
                Tone::Dim
            )
        );

        let primary = interaction.primary_claim();
        let other_claims: Vec<_> = interaction
            .claims()
            .iter()
            .filter(|claim| **claim != primary)
            .collect();
        if !other_claims.is_empty() {
            println!("{}", styled("Also reported by:", Tone::Dim));
            for claim in other_claims {
                println!(
                    "{}",
                    styled(
                        &format!(
                            "  {} -- {}: {}",
                            claim.source_name(),
                            claim.severity(),
                            claim.rationale()
                        ),
                        Tone::Dim
                    )
                );
            }
            println!();
        }
    }

    println!("{DISCLAIMER}");
    ExitCode::from(1)
}

/// A single drug's own facts and cross-reference data: RxCUI, WHO ATC
/// classification, chemical properties, and any contraindications,
/// warnings, or similar facts known about it, as opposed to `run`'s
/// interaction between two or more drugs.
pub(crate) fn info(db: &Database, args: &[String]) -> ExitCode {
    let Some(query) = args.first() else {
        eprintln!("Usage: mensung info <drug-name>");
        eprintln!("Shows RxCUI, ATC classification, chemical properties, and known");
        eprintln!("drug-specific facts (contraindications, warnings, etc.) for one drug.");
        return ExitCode::from(2);
    };

    let resolved = match resolve_all(db, std::slice::from_ref(query)) {
        Ok(resolved) => resolved,
        Err(outcome) => return outcome,
    };
    let drug = &resolved[0];

    println!("{}\n", styled(drug.name(), Tone::Bold));

    let atc_codes: Result<Vec<_>, DbError> = drug.atc_codes().collect();
    let atc_codes = match atc_codes {
        Ok(codes) => codes,
        Err(err) => return fatal_database_error(&err),
    };

    let mut has_reference_data = false;
    if let Some(rxcui) = drug.rxcui() {
        println!("{}", styled(&format!("RxCUI: {rxcui}"), Tone::Dim));
        has_reference_data = true;
    }
    for atc in &atc_codes {
        println!(
            "{}",
            styled(
                &format!("ATC: {} ({})", atc.code(), atc.class_name()),
                Tone::Dim
            )
        );
        has_reference_data = true;
    }
    if let Some(formula) = drug.molecular_formula() {
        let line = match drug.molecular_weight() {
            Some(weight) => format!("Chemical: {formula}, {weight} g/mol"),
            None => format!("Chemical: {formula}"),
        };
        println!("{}", styled(&line, Tone::Dim));
        has_reference_data = true;
    }
    if let Some(iupac) = drug.iupac_name() {
        println!("{}", styled(&format!("IUPAC name: {iupac}"), Tone::Dim));
        has_reference_data = true;
    }

    let facts = match db.drug_facts(drug.id()) {
        Ok(facts) => facts,
        Err(err) => return fatal_database_error(&err),
    };

    if has_reference_data && !facts.is_empty() {
        println!();
    }

    if facts.is_empty() {
        if !has_reference_data {
            println!(
                "No additional reference information for {} beyond interaction checks.",
                drug.name()
            );
        }
    } else {
        for (index, fact) in facts.iter().enumerate() {
            if index > 0 {
                println!();
            }
            let severity = fact.severity();
            println!(
                "{}",
                styled(
                    &format!("{} ({})", fact.kind(), severity),
                    severity_tone(severity)
                )
            );
            println!("{}", fact.rationale());
            println!(
                "{}",
                styled(
                    &format!("Evidence: {} ({})", fact.evidence(), fact.source()),
                    Tone::Dim
                )
            );

            let primary = fact.primary_claim();
            let other_claims: Vec<_> = fact
                .claims()
                .iter()
                .filter(|claim| **claim != primary)
                .collect();
            if !other_claims.is_empty() {
                println!("{}", styled("Also reported by:", Tone::Dim));
                for claim in other_claims {
                    println!(
                        "{}",
                        styled(
                            &format!(
                                "  {} -- {}: {}",
                                claim.source_name(),
                                claim.severity(),
                                claim.rationale()
                            ),
                            Tone::Dim
                        )
                    );
                }
            }
        }
    }

    println!("\n{DISCLAIMER}");
    ExitCode::SUCCESS
}

fn resolve_all<'a>(db: &Database<'a>, queries: &[String]) -> Result<Vec<DrugRecord<'a>>, ExitCode> {
    let mut resolved = Vec::with_capacity(queries.len());

    for query in queries {
        match lookup_drug(db, query) {
            Ok(LookupOutcome::ExactMatch(drug)) => resolved.push(drug),
            Ok(LookupOutcome::Candidates(candidates)) => {
                println!("Unknown drug:\n{query}\n");
                println!("Did you mean:\n");
                for candidate in &candidates {
                    println!(
                        "{} ({:.1}%)",
                        candidate.drug().name(),
                        candidate.similarity() * 100.0
                    );
                }
                println!("\nConfirm your selection and try again with the exact name.");
                return Err(ExitCode::from(1));
            }
            Ok(LookupOutcome::NoMatch) => {
                println!("Unknown drug:\n{query}\n");
                println!("No similar name was found in the database.");
                return Err(ExitCode::from(1));
            }
            Err(CoreError::Database(err)) => return Err(fatal_database_error(&err)),
            Err(err) => {
                eprintln!("Fatal: {err}");
                return Err(ExitCode::from(70));
            }
        }
    }

    Ok(resolved)
}

fn fatal_database_error(err: &DbError) -> ExitCode {
    eprintln!("Fatal: installed database is corrupt: {err}");
    ExitCode::from(70)
}

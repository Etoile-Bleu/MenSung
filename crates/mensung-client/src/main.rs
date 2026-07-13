//! MenSung CLI: looks up drug-drug interactions from the embedded .men
//! database. Two or more INN drug names in, every known pairwise
//! interaction out, most severe first. This is an offline informational
//! assistant; it does not replace professional clinical judgement.
//!
//! Exit codes: 0 no known interaction found; 1 at least one interaction
//! found, or a typed name could not be resolved (both mean "look again
//! before proceeding"); 2 bad command-line usage; 70 an internal or
//! database error, the same convention as the Unix EX_SOFTWARE sysexit.

use std::process::ExitCode;

use mensung_core::{check_interactions, lookup_drug, CoreError, LookupOutcome};
use mensung_db::{Database, DbError, DrugRecord};
use mensung_domain::DrugId;

static DATABASE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/medical_database.men"));

const DISCLAIMER: &str = "This software is an offline informational assistant.\nAlways use professional clinical judgement.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("Usage: mensung <drug-1> <drug-2> [<drug-3> ...]");
        eprintln!("Looks up known interactions among two or more drugs, by INN name.");
        return ExitCode::from(2);
    }

    let db = match Database::open(DATABASE_BYTES) {
        Ok(db) => db,
        Err(err) => return fatal_database_error(&err),
    };

    let resolved = match resolve_all(&db, &args) {
        Ok(resolved) => resolved,
        Err(outcome) => return outcome,
    };

    let ids: Vec<DrugId> = resolved.iter().map(DrugRecord::id).collect();
    let interactions = match check_interactions(&db, &ids) {
        Ok(interactions) => interactions,
        Err(CoreError::Database(err)) => return fatal_database_error(&err),
        Err(err) => {
            eprintln!("Fatal: {err}");
            return ExitCode::from(70);
        }
    };

    if interactions.is_empty() {
        let names: Vec<&str> = resolved.iter().map(DrugRecord::name).collect();
        println!("No known interaction among: {}", names.join(", "));
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

        println!("!!! {} INTERACTION !!!\n", interaction.severity());
        println!("{} + {}\n", name_of(lower), name_of(higher));
        println!("Severity:\n{}\n", interaction.severity());
        println!("Risk:\n{}\n", interaction.description());
        println!(
            "Evidence: {} ({})\n",
            interaction.evidence(),
            interaction.source()
        );
    }

    println!("{DISCLAIMER}");
    ExitCode::from(1)
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
    eprintln!("Fatal: embedded database is corrupt: {err}");
    ExitCode::from(70)
}

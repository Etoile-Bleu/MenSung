//! MenSung: an offline medication interaction checker. Run with no
//! arguments for the interactive terminal interface, with two or more
//! drug names for the scriptable interaction-check mode, or with
//! `info <drug-name>` for a single drug's own facts and cross-reference
//! data. This is an offline informational assistant; it does not replace
//! professional clinical judgement.
//!
//! The medication database is not embedded in this binary: it is read from
//! `medical_database.men` next to the executable (see data.rs), installed
//! on first run if missing. `version` and `check-update` are handled before
//! any of that: neither needs a database, and neither should be blocked by
//! (or trigger) the dataset install prompt. Installing the dataset and
//! `check-update` (see update.rs) are the only two things that touch the
//! network, and both only with explicit confirmation or an explicit
//! command; nothing here ever runs a network request the user did not ask
//! for.

mod cli;
mod data;
mod tui;
mod update;

use std::process::ExitCode;

use mensung_db::Database;

const DISCLAIMER: &str = "This software is an offline informational assistant.\nAlways use professional clinical judgement.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("version") => return print_version(),
        Some("check-update") => return update::run(),
        _ => {}
    }

    let path = data::database_path();
    let bytes = match data::load_or_install(&path) {
        Ok(bytes) => bytes,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(70);
        }
    };

    let db = match Database::open(&bytes) {
        Ok(db) => db,
        Err(err) => {
            eprintln!("Fatal: database at {} is corrupt: {err}", path.display());
            return ExitCode::from(70);
        }
    };

    match args.first().map(String::as_str) {
        None => tui::run(&db),
        Some("info") => cli::info(&db, &args[1..]),
        Some(_) => cli::run(&db, &args),
    }
}

fn print_version() -> ExitCode {
    println!("mensung {}", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}

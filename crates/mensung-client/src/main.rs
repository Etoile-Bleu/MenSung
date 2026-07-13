//! MenSung: an offline medication interaction checker. Run with no
//! arguments for the interactive terminal interface, or with two or more
//! drug names for the scriptable command-line mode. This is an offline
//! informational assistant; it does not replace professional clinical
//! judgement.
//!
//! The medication database is not embedded in this binary: it is read from
//! `medical_database.men` next to the executable (see data.rs), installed
//! on first run if missing. That install step is the only time this binary
//! touches the network, and only with explicit confirmation.

mod cli;
mod data;
mod tui;

use std::process::ExitCode;

use mensung_db::Database;

const DISCLAIMER: &str = "This software is an offline informational assistant.\nAlways use professional clinical judgement.";

fn main() -> ExitCode {
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

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        tui::run(&db)
    } else {
        cli::run(&db, &args)
    }
}

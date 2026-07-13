//! MenSung: an offline medication interaction checker. Run with no
//! arguments for the interactive terminal interface, or with two or more
//! drug names for the scriptable command-line mode. This is an offline
//! informational assistant; it does not replace professional clinical
//! judgement.

mod cli;
mod tui;

use std::process::ExitCode;

use mensung_db::Database;

static DATABASE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/medical_database.men"));

const DISCLAIMER: &str = "This software is an offline informational assistant.\nAlways use professional clinical judgement.";

fn main() -> ExitCode {
    let db = match Database::open(DATABASE_BYTES) {
        Ok(db) => db,
        Err(err) => {
            eprintln!("Fatal: embedded database is corrupt: {err}");
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

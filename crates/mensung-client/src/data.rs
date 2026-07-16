//! Locates, loads, and if needed installs the .men database at runtime.
//! There is nothing embedded in the binary: `mensung` looks for
//! `medical_database.men` next to its own executable (or under
//! `MENSUNG_DATA_DIR` if set), and offers to download a dataset if it is
//! missing. This means the shipped binary does carry network code, unlike
//! the fully offline design described in earlier project notes; see
//! README.md's Security model section for the current, accurate statement
//! of what this binary does and does not do over the network, and why.
//!
//! Two sources are tried, in order: `dataset_download.rs`'s pre-built,
//! enriched database (DDInter + RxNorm + WHO ATC + PubChem + openFDA,
//! downloaded as a single file, no per-drug API calls) first, since it is
//! both richer and faster to install; then, only if that fails, a bare
//! DDInter-only database built live from CSVs, the original install path.
//! A field deployment with only intermittent connectivity to GitHub still
//! ends up with a usable database either way.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::style::{styled_err as styled, Tone};

pub(crate) fn database_path() -> PathBuf {
    let dir = std::env::var("MENSUNG_DATA_DIR")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| PathBuf::from("."));

    dir.join("medical_database.men")
}

/// Returns the compiled database bytes, installing them first if no
/// database is present at `path` yet. Never touches the network unless the
/// user explicitly agrees, either interactively or via
/// `MENSUNG_DOWNLOAD_DATASET=1`.
pub(crate) fn load_or_install(path: &Path) -> Result<Vec<u8>, String> {
    if path.is_file() {
        return std::fs::read(path)
            .map_err(|err| format!("Fatal: could not read {}: {err}", path.display()));
    }

    eprintln!(
        "{}",
        styled(
            &format!("No medication database found at {}.", path.display()),
            Tone::Warning
        )
    );
    eprintln!(
        "You can place a compiled medical_database.men there yourself, or let mensung install a dataset now."
    );

    if !should_install() {
        return Err(format!(
            "No database installed at {}. Nothing to look up.\n\
             Run again and answer \"y\", set MENSUNG_DOWNLOAD_DATASET=1, or place the file manually.",
            path.display()
        ));
    }

    install(path)
}

fn should_install() -> bool {
    match std::env::var("MENSUNG_DOWNLOAD_DATASET").as_deref() {
        Ok("1") | Ok("true") | Ok("yes") => return true,
        Ok("0") | Ok("false") | Ok("no") => return false,
        _ => {}
    }

    if !std::io::stdin().is_terminal() {
        eprintln!(
            "Not an interactive terminal; set MENSUNG_DOWNLOAD_DATASET=1 to install without prompting."
        );
        return false;
    }

    confirm_interactively()
}

fn confirm_interactively() -> bool {
    eprint!(
        "{} ",
        styled("Would you like to install the dataset now? [y/N]", Tone::Bold)
    );
    let _ = std::io::stderr().flush();

    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn install(path: &Path) -> Result<Vec<u8>, String> {
    eprintln!(
        "{}",
        styled(
            "Downloading MenSung's enriched dataset (DDInter + RxNorm + WHO ATC + PubChem + openFDA)...",
            Tone::Bold
        )
    );

    match crate::dataset_download::fetch() {
        Ok(bytes) => {
            write_database(path, &bytes)?;
            eprintln!(
                "{}",
                styled(
                    &format!("Installed the enriched dataset to {}.", path.display()),
                    Tone::Ok
                )
            );
            return Ok(bytes);
        }
        Err(err) => {
            eprintln!(
                "{}",
                styled(
                    &format!(
                        "Could not download the enriched dataset ({err}); falling back to a \
                         DDInter-only build..."
                    ),
                    Tone::Warning
                )
            );
        }
    }

    install_ddinter_only(path)
}

fn install_ddinter_only(path: &Path) -> Result<Vec<u8>, String> {
    eprintln!(
        "{}",
        styled(
            "Downloading DDInter's dataset (CC BY-NC-SA 4.0, see MEDICAL_DATA_POLICY.md)...",
            Tone::Bold
        )
    );

    let download_dir = std::env::temp_dir().join("mensung-ddinter-download");
    let (drugs, interactions) = mensung_builder::download_and_import_ddinter(&download_dir)
        .map_err(|err| format!("Fatal: could not download the dataset: {err}"))?;
    let interactions = mensung_builder::wrap_as_claims(interactions);

    let (bytes, report) = mensung_builder::build_database(drugs, interactions, Vec::new())
        .map_err(|err| format!("Fatal: could not compile the downloaded dataset: {err}"))?;

    write_database(path, &bytes)?;

    eprintln!(
        "{}",
        styled(
            &format!(
                "Installed {} interactions to {}.",
                report.interactions,
                path.display()
            ),
            Tone::Ok
        )
    );

    Ok(bytes)
}

fn write_database(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|err| {
        format!(
            "Fatal: could not save the database to {}: {err}",
            path.display()
        )
    })
}

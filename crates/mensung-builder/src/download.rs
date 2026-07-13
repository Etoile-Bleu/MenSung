//! Downloads DDInter's eight public CSV files over HTTPS and imports them.
//! This is the only network-touching code in the workspace; it exists so
//! `mensung-client` can offer to install the real dataset at runtime
//! instead of requiring a human to fetch and place the files by hand. TLS
//! certificate validation is never disabled: a failed or invalid-certificate
//! connection is a hard error here, never silently downgraded, from either
//! source below.
//!
//! Two sources are tried in order, per file: DDInter's own site first, then
//! a mirror hosted as assets on this project's own GitHub Release
//! `ddinter-mirror-2025-08-30`. The mirror exists because DDInter's TLS
//! certificate was found expired (verified directly and repeatedly, not
//! assumed) while building this importer; it holds an unmodified,
//! byte-for-byte copy of the same eight files, fetched via a Wayback
//! Machine snapshot since the live site could not be reached over a
//! validated connection at the time. If DDInter's certificate is ever
//! renewed, the primary source starts succeeding again automatically and
//! the mirror goes back to being an unused fallback.
//!
//! DDInter's data, and this mirror of it, is licensed CC BY-NC-SA 4.0,
//! separate from this project's own MIT/Apache-2.0 code license; see
//! MEDICAL_DATA_POLICY.md before calling this from anything that
//! redistributes the result.

use std::path::{Path, PathBuf};

use mensung_domain::{Drug, Interaction};

use crate::ddinter::{import_ddinter, ImportError};

const ATC_CODES: [&str; 8] = ["A", "B", "D", "H", "L", "P", "R", "V"];
const MIRROR_BASE: &str =
    "https://github.com/Etoile-Bleu/MenSung/releases/download/ddinter-mirror-2025-08-30";

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error(
        "could not fetch {file} from either DDInter or the project mirror: \
         primary error: {primary}; mirror error: {mirror}"
    )]
    BothSourcesFailed {
        file: String,
        primary: Box<ureq::Error>,
        mirror: Box<ureq::Error>,
    },

    #[error("filesystem error while caching DDInter data: {0}")]
    Io(#[from] std::io::Error),

    #[error("downloaded DDInter files did not import cleanly: {0}")]
    Import(#[from] ImportError),
}

/// Downloads DDInter's eight CSV files into `dest_dir`, then imports them.
/// Files are written under a temporary name first and only renamed into
/// their final names once all eight have downloaded successfully, so a
/// failure partway through never leaves a partial, silently-incomplete
/// dataset behind for a later run to pick up by accident.
pub fn download_and_import_ddinter(
    dest_dir: &Path,
) -> Result<(Vec<Drug>, Vec<Interaction>), DownloadError> {
    fetch_all(dest_dir)?;
    let files = open_all(dest_dir)?;
    Ok(import_ddinter(files)?)
}

/// True if all eight of DDInter's CSV files are already present in `dir`,
/// so a caller can skip the network entirely on a second run.
pub fn is_cached(dir: &Path) -> bool {
    ATC_CODES
        .iter()
        .all(|code| dir.join(csv_filename(code)).is_file())
}

fn csv_filename(code: &str) -> String {
    format!("ddinter_downloads_code_{code}.csv")
}

fn fetch_all(dest_dir: &Path) -> Result<(), DownloadError> {
    std::fs::create_dir_all(dest_dir)?;

    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(ATC_CODES.len());
    for code in ATC_CODES {
        let name = csv_filename(code);
        let body = fetch_one(&name)?;

        let final_path = dest_dir.join(&name);
        let staged_path = dest_dir.join(format!("{name}.part"));
        std::fs::write(&staged_path, body)?;
        staged.push((staged_path, final_path));
    }

    for (staged_path, final_path) in staged {
        std::fs::rename(staged_path, final_path)?;
    }

    Ok(())
}

fn fetch_one(name: &str) -> Result<String, DownloadError> {
    let primary_url = format!("https://ddinter.scbdd.com/static/media/download/{name}");
    match fetch_url(&primary_url) {
        Ok(body) => Ok(body),
        Err(primary_err) => {
            let mirror_url = format!("{MIRROR_BASE}/{name}");
            match fetch_url(&mirror_url) {
                Ok(body) => Ok(body),
                Err(mirror_err) => Err(DownloadError::BothSourcesFailed {
                    file: name.to_string(),
                    primary: primary_err,
                    mirror: mirror_err,
                }),
            }
        }
    }
}

fn fetch_url(url: &str) -> Result<String, Box<ureq::Error>> {
    ureq::get(url)
        .call()
        .map_err(Box::new)?
        .body_mut()
        .read_to_string()
        .map_err(Box::new)
}

fn open_all(dir: &Path) -> Result<Vec<std::fs::File>, DownloadError> {
    ATC_CODES
        .iter()
        .map(|code| std::fs::File::open(dir.join(csv_filename(code))).map_err(DownloadError::from))
        .collect()
}

//! Downloads DDInter's eight public CSV files over HTTPS and imports them.
//! This is the only network-touching code in the workspace; it exists so
//! `mensung-client` can offer to install the real dataset at runtime
//! instead of requiring a human to fetch and place the files by hand. TLS
//! certificate validation is never disabled: a failed or invalid-certificate
//! connection is a hard error here, never silently downgraded.
//!
//! DDInter's data is licensed CC BY-NC-SA 4.0, separate from this project's
//! own MIT/Apache-2.0 code license; see MEDICAL_DATA_POLICY.md before
//! calling this from anything that redistributes the result.

use std::path::{Path, PathBuf};

use mensung_domain::{Drug, Interaction};

use crate::ddinter::{import_ddinter, ImportError};

const ATC_CODES: [&str; 8] = ["A", "B", "D", "H", "L", "P", "R", "V"];

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("network error fetching DDInter data: {0}")]
    Network(#[from] Box<ureq::Error>),

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
        let url = format!(
            "https://ddinter.scbdd.com/static/media/download/{}",
            csv_filename(code)
        );
        let body = ureq::get(&url)
            .call()
            .map_err(Box::new)?
            .body_mut()
            .read_to_string()
            .map_err(Box::new)?;

        let final_path = dest_dir.join(csv_filename(code));
        let staged_path = dest_dir.join(format!("{}.part", csv_filename(code)));
        std::fs::write(&staged_path, body)?;
        staged.push((staged_path, final_path));
    }

    for (staged_path, final_path) in staged {
        std::fs::rename(staged_path, final_path)?;
    }

    Ok(())
}

fn open_all(dir: &Path) -> Result<Vec<std::fs::File>, DownloadError> {
    ATC_CODES
        .iter()
        .map(|code| std::fs::File::open(dir.join(csv_filename(code))).map_err(DownloadError::from))
        .collect()
}

//! Downloads the pre-built, enriched `.men` database from MenSung's
//! dedicated `medical-database` GitHub Release, instead of building a bare
//! DDInter-only database live from CSVs the way `data.rs`'s DDInter-only
//! `install()` path does. See MEDICAL_DATA_POLICY.md for what "enriched"
//! means (DDInter + RxNorm + WHO ATC + PubChem + openFDA) and the license
//! that governs the composite file.
//!
//! `medical-database` is a stable, non-versioned release tag, deliberately
//! separate from the app's own `vX.Y.Z` release tags (the same pattern
//! `download.rs`'s `ddinter-mirror-2025-08-30` release already uses for a
//! different data asset): its `medical_database.men` asset is replaced in
//! place whenever a maintainer rebuilds the dataset, so the client always
//! asks for "whatever `medical-database` currently points to" rather than
//! pinning a specific build. Verified directly against the real API and a
//! real release before writing this, not assumed.
//!
//! A downloaded file that does not open as a valid database (corrupt
//! transfer, a release with no asset yet) is rejected without being
//! written to disk; `.men`'s own header carries a SHA-256 of its payload
//! that `mensung_db::Database::open` already checks on every open (see
//! `mensung-db/src/database.rs`), so a second, separate checksum step here
//! would only duplicate a check the format already makes. Falling back to
//! `data.rs`'s DDInter-only install on any failure here is the caller's
//! job, not this module's: this module either returns a verified database
//! or an error, nothing in between.

use std::io::Read as _;

const RELEASE_TAG_URL: &str =
    "https://api.github.com/repos/Etoile-Bleu/MenSung/releases/tags/medical-database";
const ASSET_NAME: &str = "medical_database.men";

#[derive(Debug, thiserror::Error)]
pub(crate) enum DatasetDownloadError {
    #[error("could not reach the medical-database release: {0}")]
    ReleaseRequest(Box<ureq::Error>),

    #[error("medical-database release response could not be read: {0}")]
    ReleaseBody(std::io::Error),

    #[error("medical-database release response was not valid JSON: {0}")]
    ReleaseJson(serde_json::Error),

    #[error("the medical-database release has no '{ASSET_NAME}' asset")]
    AssetMissing,

    #[error("could not download {ASSET_NAME}: {0}")]
    AssetRequest(Box<ureq::Error>),

    #[error("downloaded {ASSET_NAME} could not be read: {0}")]
    AssetBody(std::io::Error),

    #[error("downloaded {ASSET_NAME} is not a valid database: {0}")]
    Corrupt(mensung_db::DbError),
}

/// Fetches `medical_database.men` from the `medical-database` release and
/// returns its bytes, only once they have been verified to open as a real
/// database. Never writes anything to disk; the caller decides where the
/// bytes go.
pub(crate) fn fetch() -> Result<Vec<u8>, DatasetDownloadError> {
    let download_url = find_asset_url()?;
    let bytes = download_asset(&download_url)?;

    mensung_db::Database::open(&bytes).map_err(DatasetDownloadError::Corrupt)?;

    Ok(bytes)
}

fn find_asset_url() -> Result<String, DatasetDownloadError> {
    let mut response = ureq::get(RELEASE_TAG_URL)
        .call()
        .map_err(|err| DatasetDownloadError::ReleaseRequest(Box::new(err)))?;

    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(DatasetDownloadError::ReleaseBody)?;

    let release: serde_json::Value =
        serde_json::from_str(&body).map_err(DatasetDownloadError::ReleaseJson)?;

    release
        .get("assets")
        .and_then(|assets| assets.as_array())
        .into_iter()
        .flatten()
        .find(|asset| asset.get("name").and_then(|n| n.as_str()) == Some(ASSET_NAME))
        .and_then(|asset| asset.get("browser_download_url"))
        .and_then(|url| url.as_str())
        .map(str::to_string)
        .ok_or(DatasetDownloadError::AssetMissing)
}

fn download_asset(url: &str) -> Result<Vec<u8>, DatasetDownloadError> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|err| DatasetDownloadError::AssetRequest(Box::new(err)))?;

    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut bytes)
        .map_err(DatasetDownloadError::AssetBody)?;

    Ok(bytes)
}

// Hits the real GitHub API and downloads the real, currently-published
// dataset, so this is `#[ignore]`d by default and excluded from
// `cargo test --workspace`, the same convention every other live-network
// test in this workspace follows. Run explicitly with
// `cargo test -p mensung-client --bin mensung dataset_download -- --ignored`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "hits the live GitHub API and downloads the real dataset"]
    fn fetches_and_verifies_the_published_dataset() {
        let bytes = fetch().expect("the medical-database release should exist and be valid");
        assert!(!bytes.is_empty());
    }

    #[test]
    #[ignore = "hits the live GitHub API"]
    fn finds_the_asset_url_on_the_real_release() {
        let url = find_asset_url().expect("the medical-database release should have the asset");
        assert!(url.contains(ASSET_NAME));
    }
}

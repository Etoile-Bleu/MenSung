//! `mensung check-update`: a manual, explicitly user-triggered check
//! against GitHub's release API for a version newer than the one
//! currently running. Never runs automatically, at startup or otherwise:
//! MenSung's network access is opt-in and explicit everywhere, the same
//! rule `data.rs`'s runtime dataset install already follows. This never
//! downloads or installs anything itself, only reports what it found and
//! where to get it; nothing here can silently replace the running
//! binary the way a self-updating executable could be abused to.
//!
//! Verified against the real, live API before writing this (not
//! assumed): `GET api.github.com/repos/Etoile-Bleu/MenSung/releases/latest`
//! needs no authentication and returns `tag_name` (e.g. `"v0.1.0"`),
//! `name`, `html_url`, `published_at`, and `body` (the release notes),
//! among other fields this does not use.

use std::cmp::Ordering;
use std::io::Read as _;
use std::process::ExitCode;

const RELEASES_LATEST_URL: &str =
    "https://api.github.com/repos/Etoile-Bleu/MenSung/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn run() -> ExitCode {
    println!("Checking for updates...");

    let body = match fetch_latest_release_json() {
        Ok(body) => body,
        Err(message) => {
            eprintln!("Could not check for updates: {message}");
            return ExitCode::from(70);
        }
    };

    let release: serde_json::Value = match serde_json::from_str(&body) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Could not check for updates: unexpected response from GitHub: {err}");
            return ExitCode::from(70);
        }
    };

    let Some(tag_name) = release.get("tag_name").and_then(|v| v.as_str()) else {
        eprintln!("Could not check for updates: GitHub's response had no release tag.");
        return ExitCode::from(70);
    };
    let latest_version = tag_name.strip_prefix('v').unwrap_or(tag_name);

    match compare_versions(CURRENT_VERSION, latest_version) {
        Some(Ordering::Less) => report_update_available(tag_name, &release),
        Some(Ordering::Equal | Ordering::Greater) => {
            println!("You are running the latest version ({CURRENT_VERSION}).");
        }
        None => {
            let url = release
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or(RELEASES_LATEST_URL);
            println!(
                "Could not compare versions ({CURRENT_VERSION} installed, {latest_version} \
                 published); check yourself at {url}"
            );
        }
    }

    ExitCode::SUCCESS
}

fn report_update_available(tag_name: &str, release: &serde_json::Value) {
    println!("\nA new version is available: {tag_name} (you have {CURRENT_VERSION})\n");
    if let Some(name) = release.get("name").and_then(|v| v.as_str()) {
        println!("{name}");
    }
    if let Some(published) = release.get("published_at").and_then(|v| v.as_str()) {
        println!("Published: {published}\n");
    }
    if let Some(notes) = release.get("body").and_then(|v| v.as_str()) {
        println!("{notes}\n");
    }
    if let Some(url) = release.get("html_url").and_then(|v| v.as_str()) {
        println!("Download it yourself from: {url}");
    }
    println!("\nMenSung never downloads or installs updates automatically.");
}

fn fetch_latest_release_json() -> Result<String, String> {
    let mut response = ureq::get(RELEASES_LATEST_URL)
        .call()
        .map_err(|err| err.to_string())?;
    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|err| err.to_string())?;
    Ok(body)
}

/// Parses a plain `major.minor.patch` version string; MenSung does not
/// use pre-release or build-metadata suffixes. Returns `None` if the
/// string does not have exactly that shape, rather than guessing at a
/// partial comparison.
fn parse_version(raw: &str) -> Option<(u32, u32, u32)> {
    let mut parts = raw.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn compare_versions(current: &str, latest: &str) -> Option<Ordering> {
    let current = parse_version(current)?;
    let latest = parse_version(latest)?;
    Some(current.cmp(&latest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_semver_triple() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("12.34.56"), Some((12, 34, 56)));
    }

    #[test]
    fn rejects_a_pre_release_or_build_suffix() {
        assert_eq!(parse_version("0.1.0-alpha"), None);
        assert_eq!(parse_version("0.1.0+build.5"), None);
    }

    #[test]
    fn rejects_a_malformed_version() {
        assert_eq!(parse_version("v0.1.0"), None);
        assert_eq!(parse_version("0.1"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn strips_a_leading_v_before_parsing() {
        let tag = "v0.1.0";
        let stripped = tag.strip_prefix('v').unwrap_or(tag);
        assert_eq!(parse_version(stripped), Some((0, 1, 0)));
    }

    #[test]
    fn compares_versions_numerically_not_lexicographically() {
        // Lexicographic comparison would get "0.9.0" < "0.10.0" wrong.
        assert_eq!(compare_versions("0.9.0", "0.10.0"), Some(Ordering::Less));
    }

    #[test]
    fn detects_a_newer_version() {
        assert_eq!(compare_versions("0.1.0", "0.2.0"), Some(Ordering::Less));
    }

    #[test]
    fn detects_the_current_version_as_up_to_date() {
        assert_eq!(compare_versions("0.1.0", "0.1.0"), Some(Ordering::Equal));
    }

    #[test]
    fn detects_a_locally_ahead_version_as_not_needing_an_update() {
        // A dev build with a bumped Cargo.toml version, not yet
        // released, should not be told to "update" to an older tag.
        assert_eq!(compare_versions("0.2.0", "0.1.0"), Some(Ordering::Greater));
    }

    #[test]
    fn refuses_to_compare_an_unparseable_version_rather_than_guess() {
        assert_eq!(compare_versions("0.1.0", "not-a-version"), None);
    }
}

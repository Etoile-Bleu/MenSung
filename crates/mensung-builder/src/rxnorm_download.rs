//! Fetches each drug's RxCUI from RxNorm's REST API
//! (`rxnav.nlm.nih.gov/REST/rxcui.json`), one request per drug name.
//! RxNorm's own Terms of Service
//! (`lhncbc.nlm.nih.gov/RxNav/TermsofService.html`, checked directly, not
//! assumed) states a limit of 20 requests per second per IP address;
//! requests here are paced at 10 per second, half that limit, leaving
//! margin rather than running at the boundary. TLS certificate validation
//! is never disabled, the same rule every other downloader in this crate
//! follows.
//!
//! Every function in this file makes a real HTTPS request, so its tests
//! are `#[ignore]`d by default and excluded from `cargo test --workspace`,
//! the same convention every other live-network test in this workspace
//! follows. Run them explicitly with `cargo test -p mensung-builder --lib
//! rxnorm_download -- --ignored`. `rxnorm.rs`'s own tests cover the
//! parsing logic that consumes the response bodies these functions
//! return, using real captured responses as fixtures, and do not touch
//! the network.

use std::io::Read as _;
use std::thread;
use std::time::Duration;

use mensung_domain::DrugId;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::retry::with_retry;

const REQUEST_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, thiserror::Error)]
pub enum RxNormFetchError {
    #[error("request to RxNorm for '{drug_name}' failed: {source}")]
    Request {
        drug_name: String,
        source: Box<ureq::Error>,
    },

    #[error("RxNorm response body for '{drug_name}' could not be read: {source}")]
    Body {
        drug_name: String,
        source: std::io::Error,
    },
}

/// Fetches the raw `rxcui.json` body for `drug_name`, using RxNorm's
/// normalized search mode (`search=1`). Unlike openFDA, RxNorm returns
/// HTTP 200 with an empty `idGroup` for "no match" rather than a 404
/// (verified directly), so there is no `Option` here: any non-success
/// status or transport error is a hard error, and parsing "no match" out
/// of a successful body is `rxnorm.rs`'s job.
pub fn fetch_one(drug_name: &str) -> Result<String, RxNormFetchError> {
    let encoded = utf8_percent_encode(drug_name, NON_ALPHANUMERIC);
    let url = format!("https://rxnav.nlm.nih.gov/REST/rxcui.json?name={encoded}&search=1");

    let mut response = ureq::get(&url)
        .call()
        .map_err(|err| RxNormFetchError::Request {
            drug_name: drug_name.to_string(),
            source: Box::new(err),
        })?;

    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|source| RxNormFetchError::Body {
            drug_name: drug_name.to_string(),
            source,
        })?;
    Ok(body)
}

/// Fetches RxCUI lookup responses for every `(DrugId, name)` pair, in
/// order, pacing requests to stay well under RxNorm's rate limit. Each
/// request is retried a few times with backoff on a transient failure
/// (see `retry.rs`) before giving up; a single request still failing
/// after retries stops the whole batch, the same fail-fast choice
/// `openfda_download.rs::fetch_all` makes, rather than silently returning
/// a partial result the caller has no way to know is incomplete.
pub fn fetch_all(drugs: &[(DrugId, String)]) -> Result<Vec<(DrugId, String)>, RxNormFetchError> {
    let mut results = Vec::with_capacity(drugs.len());
    let total = drugs.len();

    for (index, (drug_id, name)) in drugs.iter().enumerate() {
        if index > 0 {
            thread::sleep(REQUEST_INTERVAL);
        }
        eprintln!("  [{}/{total}] RxNorm: {name}", index + 1);
        let body = with_retry(&format!("RxNorm lookup of '{name}'"), || fetch_one(name))?;
        results.push((*drug_id, body));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "hits the live RxNorm API"]
    fn a_real_drug_name_returns_a_matching_rxcui() {
        let body = fetch_one("Warfarin").expect("network request should succeed");
        assert!(body.contains("\"rxnormId\""));
        assert!(body.contains("11289"));
    }

    #[test]
    #[ignore = "hits the live RxNorm API"]
    fn a_nonexistent_drug_name_returns_an_empty_id_group() {
        let body =
            fetch_one("thisisnotarealdrugnamexyz123").expect("network request should succeed");
        assert!(body.contains("\"idGroup\":{}"));
    }

    #[test]
    #[ignore = "hits the live RxNorm API"]
    fn a_name_with_a_parenthetical_qualifier_still_resolves() {
        let body = fetch_one("Dexamethasone (topical)").expect("network request should succeed");
        assert!(body.contains("\"rxnormId\""));
    }
}

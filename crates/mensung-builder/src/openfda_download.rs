//! Fetches OpenFDA drug label records for a given list of INN drug names,
//! one drug at a time via the live search API
//! (`api.fda.gov/drug/label.json`), rather than the 260,530 record /
//! ~1.8GB bulk export (checked directly via `api.fda.gov/download.json`;
//! see `openfda.rs`'s header): MenSung only needs labels for drugs it
//! already knows about from DDInter, a small fraction of the full export.
//!
//! Requests are spaced out to stay under openFDA's unauthenticated rate
//! limit of 40 requests per minute
//! (open.fda.gov/apis/authentication/#rate-limiting), so this needs no API
//! key to run. TLS certificate validation is never disabled, the same rule
//! `download.rs` follows for DDInter. A 404 response means "no label found
//! for this search" (verified directly: openFDA returns
//! `{"error":{"code":"NOT_FOUND", ...}}` with HTTP 404 for a query with no
//! matches), not a failure; any other non-success status or transport
//! error is a hard error.

use std::io::Read as _;
use std::thread;
use std::time::Duration;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::retry::with_retry;

/// openFDA's unauthenticated limit is 40 requests/minute; 1.6s between
/// requests gives 37.5/minute, leaving margin rather than running exactly
/// at the boundary.
const REQUEST_INTERVAL: Duration = Duration::from_millis(1600);

#[derive(Debug, thiserror::Error)]
pub enum OpenFdaFetchError {
    #[error("request to openFDA for '{drug_name}' failed: {source}")]
    Request {
        drug_name: String,
        source: Box<ureq::Error>,
    },

    #[error("openFDA response body for '{drug_name}' could not be read: {source}")]
    Body {
        drug_name: String,
        source: std::io::Error,
    },
}

/// Fetches the first label record openFDA has for `drug_name`, or `None`
/// if openFDA has no match. Does not sleep before the request; callers
/// fetching more than one drug should use `fetch_all` instead, which
/// paces requests to stay under the rate limit.
pub fn fetch_one(drug_name: &str) -> Result<Option<String>, OpenFdaFetchError> {
    let encoded = utf8_percent_encode(drug_name, NON_ALPHANUMERIC);
    let url = format!(
        "https://api.fda.gov/drug/label.json?search=openfda.generic_name:%22{encoded}%22&limit=1"
    );

    match ureq::get(&url).call() {
        Ok(mut response) => {
            let mut body = String::new();
            response
                .body_mut()
                .as_reader()
                .read_to_string(&mut body)
                .map_err(|source| OpenFdaFetchError::Body {
                    drug_name: drug_name.to_string(),
                    source,
                })?;
            Ok(Some(body))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(err) => Err(OpenFdaFetchError::Request {
            drug_name: drug_name.to_string(),
            source: Box::new(err),
        }),
    }
}

/// Fetches label records for every name in `drug_names`, in order, pacing
/// requests to stay under openFDA's unauthenticated rate limit. Names with
/// no matching label are simply absent from the result, not an error.
/// Each request is retried a few times with backoff on a transient
/// failure (see `retry.rs`) before giving up; a single drug's request
/// still failing after retries (a real transport or server error, not a
/// 404) stops the whole batch rather than silently continuing with a
/// partial result the caller has no way to know is incomplete.
pub fn fetch_all(drug_names: &[String]) -> Result<Vec<String>, OpenFdaFetchError> {
    let mut bodies = Vec::new();
    let total = drug_names.len();

    for (index, drug_name) in drug_names.iter().enumerate() {
        if index > 0 {
            thread::sleep(REQUEST_INTERVAL);
        }
        eprintln!("  [{}/{total}] OpenFDA: {drug_name}", index + 1);
        if let Some(body) = with_retry(&format!("OpenFDA lookup of '{drug_name}'"), || {
            fetch_one(drug_name)
        })? {
            bodies.push(body);
        }
    }

    Ok(bodies)
}

// Every function in this file makes a real HTTPS request, and a test
// suite that depends on an external service being up on every run is a
// flaky test suite, so these are `#[ignore]`d by default and excluded
// from `cargo test --workspace`. Run them explicitly with
// `cargo test -p mensung-builder --lib openfda_download -- --ignored`
// when verifying this module against the live API by hand.
// `openfda.rs`'s tests cover the parsing and matching logic that
// consumes the response bodies these functions return, using a real
// captured response as a fixture, and do not touch the network.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "hits the live openFDA API"]
    fn a_real_drug_name_returns_a_matching_label() {
        let body = fetch_one("Warfarin")
            .expect("network request should succeed")
            .expect("openFDA should have at least one warfarin label");
        assert!(body.contains("\"results\""));
        assert!(body.to_uppercase().contains("WARFARIN"));
    }

    #[test]
    #[ignore = "hits the live openFDA API"]
    fn a_nonexistent_drug_name_returns_none() {
        let result = fetch_one("thisisnotarealdrugnamexyz123")
            .expect("network request should succeed even with no match");
        assert!(result.is_none());
    }

    #[test]
    #[ignore = "hits the live openFDA API"]
    fn fetch_all_paces_requests_and_skips_unmatched_names() {
        let names = vec![
            "Warfarin".to_string(),
            "thisisnotarealdrugnamexyz123".to_string(),
        ];
        let bodies = fetch_all(&names).expect("network requests should succeed");
        assert_eq!(bodies.len(), 1, "only Warfarin should have matched");
    }
}

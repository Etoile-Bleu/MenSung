//! Fetches WHO ATC classification codes for each drug from NLM's RxClass
//! API (`rxnav.nlm.nih.gov/REST/rxclass/class/byRxcui.json`), one request
//! per drug's RxCUI. This is the same `rxnav.nlm.nih.gov` service RxNorm
//! itself is served from (see `rxnorm_download.rs`), so requests here are
//! paced under the same Terms of Service limit (20 requests/second per
//! IP, checked directly), at 10 per second. TLS certificate validation is
//! never disabled.
//!
//! Because ATC lookup goes through RxClass rather than WHO's own site
//! (see `atc.rs`'s header for why), it needs a drug's `Rxcui` as input,
//! not just its name: this only runs on drugs the RxNorm lookup already
//! resolved.
//!
//! Every function in this file makes a real HTTPS request, so its tests
//! are `#[ignore]`d by default and excluded from `cargo test --workspace`,
//! the same convention every other live-network test in this workspace
//! follows. Run them explicitly with `cargo test -p mensung-builder --lib
//! atc_download -- --ignored`. `atc.rs`'s own tests cover the parsing and
//! filtering logic that consumes the response bodies these functions
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
pub enum AtcFetchError {
    #[error("request to RxClass for RxCUI '{rxcui}' failed: {source}")]
    Request {
        rxcui: String,
        source: Box<ureq::Error>,
    },

    #[error("RxClass response body for RxCUI '{rxcui}' could not be read: {source}")]
    Body {
        rxcui: String,
        source: std::io::Error,
    },
}

/// Fetches the raw RxClass ATC classification response for `rxcui`.
/// RxClass returns HTTP 200 with `{}` for an RxCUI with no ATC
/// classification (verified directly), so there is no `Option` here, the
/// same shape RxNorm's own `rxcui.json` "no match" case takes.
pub fn fetch_one(rxcui: &str) -> Result<String, AtcFetchError> {
    let encoded = utf8_percent_encode(rxcui, NON_ALPHANUMERIC);
    let url = format!(
        "https://rxnav.nlm.nih.gov/REST/rxclass/class/byRxcui.json?rxcui={encoded}&relaSource=ATC"
    );

    let mut response = ureq::get(&url)
        .call()
        .map_err(|err| AtcFetchError::Request {
            rxcui: rxcui.to_string(),
            source: Box::new(err),
        })?;

    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|source| AtcFetchError::Body {
            rxcui: rxcui.to_string(),
            source,
        })?;
    Ok(body)
}

/// Fetches ATC classification responses for every `(DrugId, rxcui)` pair,
/// in order, pacing requests to stay well under RxClass's rate limit.
/// Returns each input's `DrugId` and `rxcui` alongside its response body,
/// since `atc.rs::attach_atc_codes` needs the queried RxCUI to filter out
/// related combination-product entries RxClass can include in the same
/// response. Each request is retried a few times with backoff on a
/// transient failure (see `retry.rs`) before giving up; a single request
/// still failing after retries stops the whole batch, the same fail-fast
/// choice every other fetcher in this crate makes.
pub fn fetch_all(
    drugs: &[(DrugId, String)],
) -> Result<Vec<(DrugId, String, String)>, AtcFetchError> {
    let mut results = Vec::with_capacity(drugs.len());
    let total = drugs.len();

    for (index, (drug_id, rxcui)) in drugs.iter().enumerate() {
        if index > 0 {
            thread::sleep(REQUEST_INTERVAL);
        }
        eprintln!("  [{}/{total}] WHO ATC: rxcui {rxcui}", index + 1);
        let body = with_retry(&format!("ATC lookup of rxcui '{rxcui}'"), || {
            fetch_one(rxcui)
        })?;
        results.push((*drug_id, rxcui.clone(), body));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "hits the live RxClass API"]
    fn a_real_rxcui_returns_its_atc_classification() {
        let body = fetch_one("11289").expect("network request should succeed");
        assert!(body.contains("\"classId\":\"B01AA\""));
    }

    #[test]
    #[ignore = "hits the live RxClass API"]
    fn a_nonexistent_rxcui_returns_an_empty_response() {
        let body = fetch_one("999999999").expect("network request should succeed");
        assert_eq!(body.trim(), "{}");
    }
}

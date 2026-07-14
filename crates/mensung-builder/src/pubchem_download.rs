//! Fetches molecular properties for each drug from PubChem's PUG REST API
//! (`pubchem.ncbi.nlm.nih.gov/rest/pug`), one request per drug name.
//! PubChem's own usage policy asks for no more than 5 requests per second
//! per user; requests here are paced at 2 per second, well under that,
//! since PubChem also applies dynamic throttling under load and will
//! temporarily block abusive callers. TLS certificate validation is never
//! disabled, the same rule every other downloader in this crate follows.
//!
//! A name with no PubChem match returns HTTP 404 with a
//! `{"Fault":{"Code":"PUGREST.NotFound", ...}}` body (verified directly),
//! treated as "no properties for this drug," not a failure.

use std::io::Read as _;
use std::thread;
use std::time::Duration;

use mensung_domain::DrugId;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

const REQUEST_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub enum PubchemFetchError {
    #[error("request to PubChem for '{drug_name}' failed: {source}")]
    Request {
        drug_name: String,
        source: Box<ureq::Error>,
    },

    #[error("PubChem response body for '{drug_name}' could not be read: {source}")]
    Body {
        drug_name: String,
        source: std::io::Error,
    },
}

/// Fetches the raw PUG REST property response for `drug_name`, or `None`
/// if PubChem has no matching compound.
pub fn fetch_one(drug_name: &str) -> Result<Option<String>, PubchemFetchError> {
    let encoded = utf8_percent_encode(drug_name, NON_ALPHANUMERIC);
    let url = format!(
        "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{encoded}/property/MolecularFormula,MolecularWeight,IUPACName/JSON"
    );

    match ureq::get(&url).call() {
        Ok(mut response) => {
            let mut body = String::new();
            response
                .body_mut()
                .as_reader()
                .read_to_string(&mut body)
                .map_err(|source| PubchemFetchError::Body {
                    drug_name: drug_name.to_string(),
                    source,
                })?;
            Ok(Some(body))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(err) => Err(PubchemFetchError::Request {
            drug_name: drug_name.to_string(),
            source: Box::new(err),
        }),
    }
}

/// Fetches property responses for every `(DrugId, name)` pair, in order,
/// pacing requests to stay well under PubChem's rate limit. Names with no
/// match are simply absent from the result, not an error. A single
/// request failing with a real transport or server error (not a 404)
/// stops the whole batch, the same fail-fast choice every other fetcher
/// in this crate makes.
pub fn fetch_all(drugs: &[(DrugId, String)]) -> Result<Vec<(DrugId, String)>, PubchemFetchError> {
    let mut results = Vec::new();

    for (index, (drug_id, name)) in drugs.iter().enumerate() {
        if index > 0 {
            thread::sleep(REQUEST_INTERVAL);
        }
        if let Some(body) = fetch_one(name)? {
            results.push((*drug_id, body));
        }
    }

    Ok(results)
}

// Every function in this file makes a real HTTPS request, and a test
// suite that depends on an external service being up on every run is a
// flaky test suite, so these are `#[ignore]`d by default and excluded
// from `cargo test --workspace`. Run them explicitly with
// `cargo test -p mensung-builder --lib pubchem_download -- --ignored`
// when verifying this module against the live API by hand.
// `pubchem.rs`'s tests cover the parsing logic that consumes the
// response bodies these functions return, using real captured responses
// as fixtures, and do not touch the network.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "hits the live PubChem API"]
    fn a_real_drug_name_returns_matching_properties() {
        let body = fetch_one("Warfarin")
            .expect("network request should succeed")
            .expect("PubChem should have a compound for warfarin");
        assert!(body.contains("\"MolecularFormula\""));
        assert!(body.contains("C19H16O4"));
    }

    #[test]
    #[ignore = "hits the live PubChem API"]
    fn a_nonexistent_drug_name_returns_none() {
        let result = fetch_one("thisisnotarealdrugnamexyz123")
            .expect("network request should succeed even with no match");
        assert!(result.is_none());
    }
}

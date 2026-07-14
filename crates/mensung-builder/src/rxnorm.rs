//! Parses RxNorm `rxcui.json` lookup responses
//! (`rxnav.nlm.nih.gov/REST/rxcui.json`) and attaches the resulting RxCUI
//! to each matched `Drug`. Verified against real live responses (not
//! assumed): a match looks like `{"idGroup":{"rxnormId":["11289"]}}`, no
//! match looks like `{"idGroup":{}}`, both with HTTP 200 (unlike
//! OpenFDA's 404-for-no-match, checked separately; see
//! `rxnorm_download.rs`).
//!
//! Lookups use RxNorm's own "normalized" search mode (`search=1`), which
//! already accounts for salt forms, word order, and common abbreviations
//! server-side ("morphine sulfate" matches "morphine", per RxNorm's own
//! documentation for `findRxcuiByString`). This project does not layer
//! its own fuzzy matching on top of that: RxNorm's normalized match is
//! already conservative enough in practice (checked directly against
//! "Dexamethasone (topical)", one of the more unusual INN names in
//! DDInter's dataset, which still resolved correctly) that adding a
//! second, weaker matching pass would only add false-match risk, not
//! reduce it. A drug with no match is left without an RxCUI, not guessed
//! at; RxNorm can list more than one candidate RxCUI for an ambiguous
//! name, and the first is used as-is, since it is already RxNorm's own
//! best-ranked match, not re-ranked here.

use std::collections::HashMap;

use mensung_domain::{Drug, DrugId, Rxcui};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum RxNormImportError {
    #[error("failed to parse an RxNorm rxcui.json response for drug {0:?}: {1}")]
    Json(DrugId, serde_json::Error),
}

#[derive(Debug, Default, Deserialize)]
struct RxcuiResponse {
    #[serde(default, rename = "idGroup")]
    id_group: IdGroup,
}

#[derive(Debug, Default, Deserialize)]
struct IdGroup {
    #[serde(default, rename = "rxnormId")]
    rxnorm_id: Vec<String>,
}

fn parse_rxcui_response(drug_id: DrugId, body: &str) -> Result<Option<Rxcui>, RxNormImportError> {
    let parsed: RxcuiResponse =
        serde_json::from_str(body).map_err(|err| RxNormImportError::Json(drug_id, err))?;
    Ok(parsed
        .id_group
        .rxnorm_id
        .first()
        .and_then(|raw| Rxcui::parse(raw).ok()))
}

/// Attaches an RxCUI to each drug in `drugs` whose id appears in
/// `responses`, leaving every other drug unchanged. `responses` pairs a
/// `DrugId` with the raw `rxcui.json` body returned for that drug's name;
/// a drug not present in `responses`, or whose response has no match,
/// keeps `rxcui() == None`.
pub fn attach_rxcuis(
    drugs: Vec<Drug>,
    responses: &[(DrugId, String)],
) -> Result<Vec<Drug>, RxNormImportError> {
    let mut rxcuis: HashMap<DrugId, Rxcui> = HashMap::with_capacity(responses.len());
    for (drug_id, body) in responses {
        if let Some(rxcui) = parse_rxcui_response(*drug_id, body)? {
            rxcuis.insert(*drug_id, rxcui);
        }
    }

    Ok(drugs
        .into_iter()
        .map(|drug| match rxcuis.remove(&drug.id()) {
            Some(rxcui) => drug.with_rxcui(rxcui),
            None => drug,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::InnName;

    fn drug(id: u32, name: &str) -> Drug {
        Drug::new(DrugId::new(id), InnName::parse(name).unwrap())
    }

    // Real response bodies captured from rxnav.nlm.nih.gov/REST/rxcui.json
    // (verified 2026-07, see this module's header).
    const WARFARIN_MATCH: &str = r#"{"idGroup":{"rxnormId":["11289"]}}"#;
    const NO_MATCH: &str = r#"{"idGroup":{}}"#;

    #[test]
    fn attaches_a_matched_rxcui() {
        let drugs = vec![drug(0, "Warfarin")];
        let responses = vec![(DrugId::new(0), WARFARIN_MATCH.to_string())];
        let result = attach_rxcuis(drugs, &responses).unwrap();
        assert_eq!(result[0].rxcui().map(Rxcui::as_str), Some("11289"));
    }

    #[test]
    fn leaves_an_unmatched_drug_without_an_rxcui() {
        let drugs = vec![drug(0, "Warfarin")];
        let responses = vec![(DrugId::new(0), NO_MATCH.to_string())];
        let result = attach_rxcuis(drugs, &responses).unwrap();
        assert_eq!(result[0].rxcui(), None);
    }

    #[test]
    fn leaves_a_drug_with_no_response_at_all_without_an_rxcui() {
        let drugs = vec![drug(0, "Warfarin"), drug(1, "Aspirin")];
        let responses = vec![(DrugId::new(0), WARFARIN_MATCH.to_string())];
        let result = attach_rxcuis(drugs, &responses).unwrap();
        assert_eq!(result[0].rxcui().map(Rxcui::as_str), Some("11289"));
        assert_eq!(result[1].rxcui(), None);
    }

    #[test]
    fn does_not_mix_up_rxcuis_between_drugs() {
        let drugs = vec![drug(0, "Warfarin"), drug(1, "Aspirin")];
        let responses = vec![
            (DrugId::new(0), WARFARIN_MATCH.to_string()),
            (
                DrugId::new(1),
                r#"{"idGroup":{"rxnormId":["1191"]}}"#.to_string(),
            ),
        ];
        let result = attach_rxcuis(drugs, &responses).unwrap();
        assert_eq!(result[0].rxcui().map(Rxcui::as_str), Some("11289"));
        assert_eq!(result[1].rxcui().map(Rxcui::as_str), Some("1191"));
    }
}

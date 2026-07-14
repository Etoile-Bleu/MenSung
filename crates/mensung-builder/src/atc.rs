//! Parses NLM RxClass API responses
//! (`rxnav.nlm.nih.gov/REST/rxclass/class/byRxcui.json?...&relaSource=ATC`)
//! and attaches the resulting WHO ATC classification codes to each
//! matched `Drug`. WHO's own ATC/DDD Index has no bulk download or API
//! (checked directly: `whocc.no/atc_ddd_index/` is a search-only web
//! page); RxClass cross-references RxNorm concepts to ATC codes and is
//! reachable programmatically, so this project gets WHO's classification
//! through it rather than through WHO's own site. This means ATC lookup
//! needs a drug's `Rxcui` first (see `mensung-builder::rxnorm`), a real
//! pipeline dependency, not a workaround.
//!
//! Verified against real live responses (not assumed): a match looks
//! like `{"rxclassDrugInfoList":{"rxclassDrugInfo":[{"minConcept":
//! {"rxcui":"11289",...},"rxclassMinConceptItem":{"classId":"B01AA",
//! "className":"Vitamin K antagonists","classType":"ATC1-4"},...}]}}`,
//! no match is `{}`, both HTTP 200. A single RxCUI can have more than one
//! ATC code (aspirin is both a platelet aggregation inhibitor and a
//! salicylate analgesic); RxClass can also return entries for a related
//! combination-product RxCUI (e.g. "aspirin / codeine") when only the
//! plain ingredient was asked about, so this only keeps entries whose
//! `minConcept.rxcui` matches the RxCUI actually queried for.

use std::collections::HashMap;

use mensung_domain::{AtcCode, Drug, DrugId};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum AtcImportError {
    #[error("failed to parse an RxClass ATC response for drug {0:?}: {1}")]
    Json(DrugId, serde_json::Error),
}

#[derive(Debug, Default, Deserialize)]
struct RxClassResponse {
    #[serde(default, rename = "rxclassDrugInfoList")]
    drug_info_list: Option<DrugInfoList>,
}

#[derive(Debug, Default, Deserialize)]
struct DrugInfoList {
    #[serde(default, rename = "rxclassDrugInfo")]
    drug_info: Vec<DrugInfo>,
}

#[derive(Debug, Default, Deserialize)]
struct DrugInfo {
    #[serde(default, rename = "minConcept")]
    min_concept: MinConcept,
    #[serde(default, rename = "rxclassMinConceptItem")]
    class_item: ClassItem,
}

#[derive(Debug, Default, Deserialize)]
struct MinConcept {
    #[serde(default, rename = "rxcui")]
    rxcui: String,
}

#[derive(Debug, Default, Deserialize)]
struct ClassItem {
    #[serde(default, rename = "classId")]
    class_id: String,
    #[serde(default, rename = "className")]
    class_name: String,
}

fn parse_atc_response(
    drug_id: DrugId,
    queried_rxcui: &str,
    body: &str,
) -> Result<Vec<AtcCode>, AtcImportError> {
    let parsed: RxClassResponse =
        serde_json::from_str(body).map_err(|err| AtcImportError::Json(drug_id, err))?;

    let Some(list) = parsed.drug_info_list else {
        return Ok(Vec::new());
    };

    let mut codes: Vec<AtcCode> = list
        .drug_info
        .into_iter()
        .filter(|entry| entry.min_concept.rxcui == queried_rxcui)
        .filter_map(|entry| {
            AtcCode::new(entry.class_item.class_id, entry.class_item.class_name).ok()
        })
        .collect();
    codes.dedup();

    Ok(codes)
}

/// Attaches WHO ATC codes to each drug in `drugs` whose id appears in
/// `responses`, leaving every other drug unchanged. `responses` pairs a
/// `DrugId` with the RxCUI that was queried and the raw RxClass response
/// body for it; a drug not present in `responses`, or whose response has
/// no ATC entries, keeps `atc_codes()` empty rather than a guess.
pub fn attach_atc_codes(
    drugs: Vec<Drug>,
    responses: &[(DrugId, String, String)],
) -> Result<Vec<Drug>, AtcImportError> {
    let mut codes: HashMap<DrugId, Vec<AtcCode>> = HashMap::with_capacity(responses.len());
    for (drug_id, rxcui, body) in responses {
        let parsed = parse_atc_response(*drug_id, rxcui, body)?;
        if !parsed.is_empty() {
            codes.insert(*drug_id, parsed);
        }
    }

    Ok(drugs
        .into_iter()
        .map(|drug| match codes.remove(&drug.id()) {
            Some(entries) => drug.with_atc_codes(entries),
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

    // Real response bodies captured from RxClass's byRxcui.json endpoint
    // (verified 2026-07, see this module's header).
    const WARFARIN_MATCH: &str = r#"{"rxclassDrugInfoList":{"rxclassDrugInfo":[{"minConcept":{"rxcui":"11289","name":"warfarin","tty":"IN"},"rxclassMinConceptItem":{"classId":"B01AA","className":"Vitamin K antagonists","classType":"ATC1-4"},"rela":"","relaSource":"ATC"}]}}"#;
    const ASPIRIN_MATCH_WITH_RELATED_COMBO: &str = r#"{"rxclassDrugInfoList":{"rxclassDrugInfo":[{"minConcept":{"rxcui":"1191","name":"aspirin","tty":"IN"},"rxclassMinConceptItem":{"classId":"A01AD","className":"Other agents for local oral treatment","classType":"ATC1-4"},"rela":"","relaSource":"ATC"},{"minConcept":{"rxcui":"1191","name":"aspirin","tty":"IN"},"rxclassMinConceptItem":{"classId":"B01AC","className":"Platelet aggregation inhibitors excl. heparin","classType":"ATC1-4"},"rela":"","relaSource":"ATC"},{"minConcept":{"rxcui":"135095","name":"aspirin / codeine","tty":"MIN"},"rxclassMinConceptItem":{"classId":"N02AJ","className":"Opioids in combination with non-opioid analgesics","classType":"ATC1-4"},"rela":"","relaSource":"ATC"}]}}"#;
    const NO_MATCH: &str = r#"{}"#;

    #[test]
    fn attaches_a_single_matched_atc_code() {
        let drugs = vec![drug(0, "Warfarin")];
        let responses = vec![(
            DrugId::new(0),
            "11289".to_string(),
            WARFARIN_MATCH.to_string(),
        )];
        let result = attach_atc_codes(drugs, &responses).unwrap();
        assert_eq!(result[0].atc_codes().len(), 1);
        assert_eq!(result[0].atc_codes()[0].code(), "B01AA");
    }

    #[test]
    fn keeps_only_entries_for_the_queried_rxcui_not_a_related_combination_product() {
        let drugs = vec![drug(0, "Aspirin")];
        let responses = vec![(
            DrugId::new(0),
            "1191".to_string(),
            ASPIRIN_MATCH_WITH_RELATED_COMBO.to_string(),
        )];
        let result = attach_atc_codes(drugs, &responses).unwrap();
        let codes: Vec<&str> = result[0].atc_codes().iter().map(AtcCode::code).collect();
        assert_eq!(codes, vec!["A01AD", "B01AC"]);
        assert!(
            !codes.contains(&"N02AJ"),
            "combination-product entry should be filtered out"
        );
    }

    #[test]
    fn leaves_an_unmatched_drug_without_atc_codes() {
        let drugs = vec![drug(0, "Warfarin")];
        let responses = vec![(DrugId::new(0), "11289".to_string(), NO_MATCH.to_string())];
        let result = attach_atc_codes(drugs, &responses).unwrap();
        assert!(result[0].atc_codes().is_empty());
    }

    #[test]
    fn leaves_a_drug_with_no_response_at_all_without_atc_codes() {
        let drugs = vec![drug(0, "Warfarin"), drug(1, "Aspirin")];
        let responses = vec![(
            DrugId::new(0),
            "11289".to_string(),
            WARFARIN_MATCH.to_string(),
        )];
        let result = attach_atc_codes(drugs, &responses).unwrap();
        assert!(!result[0].atc_codes().is_empty());
        assert!(result[1].atc_codes().is_empty());
    }
}

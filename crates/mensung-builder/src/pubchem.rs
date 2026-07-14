//! Parses PubChem PUG REST compound property responses
//! (`pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/<name>/property/...`)
//! and attaches the resulting `ChemicalProperties` to each matched `Drug`.
//! Verified against real live responses (not assumed): CID is a plain
//! integer, `MolecularFormula` and `IUPACName` are strings, and
//! `MolecularWeight` is a decimal string, not a number
//! ("MolecularWeight": "180.16"). A no-match query returns HTTP 404 with
//! `{"Fault":{"Code":"PUGREST.NotFound", ...}}` (checked directly; see
//! `pubchem_download.rs`).
//!
//! This is reference chemistry data, not a clinical fact: it does not go
//! through the `Claim`/`Source` model, since there is no severity or
//! evidence level for a molecular formula, and nothing to resolve a
//! disagreement about (see `mensung_domain::pubchem`'s header).

use mensung_domain::{ChemicalProperties, Drug, DrugId, PubchemCid};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum PubchemImportError {
    #[error("failed to parse a PubChem property response for drug {0:?}: {1}")]
    Json(DrugId, serde_json::Error),
}

#[derive(Debug, Default, Deserialize)]
struct PropertyResponse {
    #[serde(default, rename = "PropertyTable")]
    property_table: Option<PropertyTable>,
}

#[derive(Debug, Default, Deserialize)]
struct PropertyTable {
    #[serde(default, rename = "Properties")]
    properties: Vec<Properties>,
}

#[derive(Debug, Default, Deserialize)]
struct Properties {
    #[serde(rename = "CID")]
    cid: u32,
    #[serde(default, rename = "MolecularFormula")]
    molecular_formula: Option<String>,
    #[serde(default, rename = "MolecularWeight")]
    molecular_weight: Option<String>,
    #[serde(default, rename = "IUPACName")]
    iupac_name: Option<String>,
}

fn parse_property_response(
    drug_id: DrugId,
    body: &str,
) -> Result<Option<ChemicalProperties>, PubchemImportError> {
    let parsed: PropertyResponse =
        serde_json::from_str(body).map_err(|err| PubchemImportError::Json(drug_id, err))?;

    let Some(entry) = parsed
        .property_table
        .and_then(|table| table.properties.into_iter().next())
    else {
        return Ok(None);
    };
    let (Some(formula), Some(weight)) = (entry.molecular_formula, entry.molecular_weight) else {
        return Ok(None);
    };

    Ok(ChemicalProperties::new(
        PubchemCid::new(entry.cid),
        formula,
        weight,
        entry.iupac_name,
    )
    .ok())
}

/// Attaches `ChemicalProperties` to each drug in `drugs` whose id appears
/// in `responses`, leaving every other drug unchanged. `responses` pairs
/// a `DrugId` with the raw PUG REST property response body for that
/// drug's name; a drug not present in `responses`, or whose response has
/// no match or an unparseable value, keeps `chemical_properties() ==
/// None` rather than a guess.
pub fn attach_chemical_properties(
    drugs: Vec<Drug>,
    responses: &[(DrugId, String)],
) -> Result<Vec<Drug>, PubchemImportError> {
    let mut properties: HashMap<DrugId, ChemicalProperties> =
        HashMap::with_capacity(responses.len());
    for (drug_id, body) in responses {
        if let Some(props) = parse_property_response(*drug_id, body)? {
            properties.insert(*drug_id, props);
        }
    }

    Ok(drugs
        .into_iter()
        .map(|drug| match properties.remove(&drug.id()) {
            Some(props) => drug.with_chemical_properties(props),
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

    // Real response bodies captured from PubChem's PUG REST API (verified
    // 2026-07, see this module's header).
    const WARFARIN_MATCH: &str = r#"{"PropertyTable":{"Properties":[{"CID":54678486,"MolecularFormula":"C19H16O4","MolecularWeight":"308.3","IUPACName":"4-hydroxy-3-(3-oxo-1-phenylbutyl)chromen-2-one"}]}}"#;
    const NOT_FOUND: &str = r#"{"Fault":{"Code":"PUGREST.NotFound","Message":"No CID found","Details":["No CID found that matches the given name"]}}"#;

    #[test]
    fn attaches_matched_chemical_properties() {
        let drugs = vec![drug(0, "Warfarin")];
        let responses = vec![(DrugId::new(0), WARFARIN_MATCH.to_string())];
        let result = attach_chemical_properties(drugs, &responses).unwrap();
        let props = result[0].chemical_properties().unwrap();
        assert_eq!(props.cid(), PubchemCid::new(54678486));
        assert_eq!(props.molecular_formula(), "C19H16O4");
        assert_eq!(props.molecular_weight(), "308.3");
    }

    #[test]
    fn leaves_an_unmatched_drug_without_properties() {
        let drugs = vec![drug(0, "Warfarin")];
        let responses = vec![(DrugId::new(0), NOT_FOUND.to_string())];
        let result = attach_chemical_properties(drugs, &responses).unwrap();
        assert_eq!(result[0].chemical_properties(), None);
    }

    #[test]
    fn leaves_a_drug_with_no_response_at_all_without_properties() {
        let drugs = vec![drug(0, "Warfarin"), drug(1, "Aspirin")];
        let responses = vec![(DrugId::new(0), WARFARIN_MATCH.to_string())];
        let result = attach_chemical_properties(drugs, &responses).unwrap();
        assert!(result[0].chemical_properties().is_some());
        assert!(result[1].chemical_properties().is_none());
    }
}

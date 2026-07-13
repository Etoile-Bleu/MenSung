//! Imports DDInter's downloadable CSV export into `mensung_domain` values.
//! See MEDICAL_DATA_POLICY.md's Data Sources section for why DDInter and
//! not the originally planned OpenFDA/RxNorm/WHO.
//!
//! DDInter publishes eight CSV files split by top-level ATC code, each with
//! exactly five columns: `DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level`.
//! Checked against the real files rather than assumed: drug names can
//! contain commas inside quoted fields (`"Thyroid, porcine"`), so this uses
//! the `csv` crate's RFC 4180 parsing instead of splitting on `,`, which
//! would silently corrupt those rows. `Level` takes exactly four values in
//! the real data: `Major`, `Moderate`, `Minor`, `Unknown`.
//!
//! Two limitations of the bulk export, verified by inspecting it directly,
//! not present in DDInter's own published paper's description of the
//! per-drug web interface: there is no per-pair mechanism description or
//! source citation in this file, only a severity level, and there is no
//! interaction-pair identifier, only per-drug `DDInterNNN` ids. This module
//! synthesizes a description from the severity tier and cites "DDInter"
//! itself as the source; it assigns interaction ids sequentially, in a
//! sorted, deterministic order, since none exist upstream.
//!
//! DDInter's own `DDInterNNN` drug ids are reused directly as `DrugId`
//! values (stripping the `DDInter` prefix), rather than assigning new ones,
//! so re-running the importer on an updated DDInter export keeps the same
//! ids for drugs that already existed.

use std::collections::HashMap;
use std::io::Read;

use mensung_domain::{
    DomainError, Drug, DrugId, DrugPair, EvidenceLevel, InnName, Interaction, InteractionId,
    Severity,
};
use serde::Deserialize;

const DDINTER_SOURCE: &str = "DDInter (http://ddinter.scbdd.com/)";

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("failed to read a DDInter CSV file: {0}")]
    Csv(#[from] csv::Error),

    #[error("DDInter id '{0}' does not match the expected DDInter<digits> format")]
    MalformedId(String),

    #[error("DDInter level '{0}' is not one of Major, Moderate, Minor, Unknown")]
    UnrecognizedLevel(String),

    #[error("drug data error: {0}")]
    Domain(#[from] DomainError),

    #[error(
        "DDInter id {id:?} is named '{first}' in one file and '{second}' in another; the export is inconsistent"
    )]
    ConflictingDrugName {
        id: DrugId,
        first: InnName,
        second: InnName,
    },
}

#[derive(Debug, Deserialize)]
struct DdinterRow {
    #[serde(rename = "DDInterID_A")]
    id_a: String,
    #[serde(rename = "Drug_A")]
    name_a: String,
    #[serde(rename = "DDInterID_B")]
    id_b: String,
    #[serde(rename = "Drug_B")]
    name_b: String,
    #[serde(rename = "Level")]
    level: String,
}

fn parse_ddinter_id(raw: &str) -> Result<DrugId, ImportError> {
    raw.strip_prefix("DDInter")
        .and_then(|digits| digits.parse::<u32>().ok())
        .map(DrugId::new)
        .ok_or_else(|| ImportError::MalformedId(raw.to_string()))
}

fn map_level(level: &str) -> Result<Severity, ImportError> {
    match level {
        "Major" => Ok(Severity::Contraindicated),
        "Moderate" => Ok(Severity::Moderate),
        "Minor" => Ok(Severity::Minor),
        "Unknown" => Ok(Severity::Unknown),
        other => Err(ImportError::UnrecognizedLevel(other.to_string())),
    }
}

fn synthesize_description(severity: Severity) -> &'static str {
    match severity {
        Severity::Contraindicated => {
            "DDInter classifies this as a major interaction. Treat with the same urgency as a \
             contraindication: verify management guidance before co-administering."
        }
        Severity::Moderate => {
            "DDInter classifies this as a moderate interaction. Review before co-administering."
        }
        Severity::Minor => "DDInter classifies this as a minor interaction.",
        Severity::Unknown | Severity::HighRisk => {
            "DDInter records an interaction between these drugs without a specified severity. \
             Verify clinically before co-administering."
        }
    }
}

/// Reads every row of every given DDInter CSV file and merges them into one
/// deduplicated drug list and interaction list. A pair appearing in more
/// than one file (a drug can belong to more than one ATC code) is folded
/// into a single record; if two files disagree on its severity, the more
/// severe of the two wins, per the zero false negative policy in
/// MEDICAL_DATA_POLICY.md. Checked against the real DDInter files: this
/// disagreement does not currently happen, but the importer does not rely
/// on that staying true.
pub fn import_ddinter<R: Read>(
    readers: Vec<R>,
) -> Result<(Vec<Drug>, Vec<Interaction>), ImportError> {
    let mut drug_names: HashMap<DrugId, InnName> = HashMap::new();
    let mut pair_severities: HashMap<(DrugId, DrugId), Severity> = HashMap::new();

    for reader in readers {
        let mut csv_reader = csv::Reader::from_reader(reader);
        for result in csv_reader.deserialize::<DdinterRow>() {
            let row = result?;
            let id_a = parse_ddinter_id(&row.id_a)?;
            let id_b = parse_ddinter_id(&row.id_b)?;
            let inn_a = InnName::parse(&row.name_a)?;
            let inn_b = InnName::parse(&row.name_b)?;

            record_drug_name(&mut drug_names, id_a, inn_a)?;
            record_drug_name(&mut drug_names, id_b, inn_b)?;

            let severity = map_level(&row.level)?;
            let pair = DrugPair::new(id_a, id_b)?;
            let (lower, higher) = pair.drugs();

            pair_severities
                .entry((lower, higher))
                .and_modify(|existing| *existing = (*existing).min(severity))
                .or_insert(severity);
        }
    }

    let mut drugs: Vec<Drug> = drug_names
        .into_iter()
        .map(|(id, name)| Drug::new(id, name))
        .collect();
    drugs.sort_by_key(|drug| drug.id().value());

    let mut sorted_pairs: Vec<((DrugId, DrugId), Severity)> = pair_severities.into_iter().collect();
    sorted_pairs.sort_by_key(|((lower, higher), _)| (lower.value(), higher.value()));

    let interactions = sorted_pairs
        .into_iter()
        .enumerate()
        .map(|(index, ((lower, higher), severity))| {
            let pair = DrugPair::new(lower, higher)
                .expect("lower and higher were already a valid distinct pair when first inserted");
            Interaction::new(
                InteractionId::new(index as u32),
                pair,
                severity,
                synthesize_description(severity),
                EvidenceLevel::Established,
                DDINTER_SOURCE,
            )
            .expect("severity, description, and source are all valid by construction")
        })
        .collect();

    Ok((drugs, interactions))
}

fn record_drug_name(
    drug_names: &mut HashMap<DrugId, InnName>,
    id: DrugId,
    name: InnName,
) -> Result<(), ImportError> {
    match drug_names.get(&id) {
        Some(existing) if existing != &name => Err(ImportError::ConflictingDrugName {
            id,
            first: existing.clone(),
            second: name,
        }),
        Some(_) => Ok(()),
        None => {
            drug_names.insert(id, name);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_db::Database;

    // Real rows taken directly from DDInter's downloadable CSV files
    // (verified 2026-07, see the crate-level doc comment), trimmed to a
    // handful for testing rather than redistributing the full export.
    const SAMPLE_FILE_A: &str = "DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level\n\
DDInter513,Dexamethasone,DDInter582,Dolutegravir,Minor\n\
DDInter582,Dolutegravir,DDInter625,Elagolix,Minor\n";

    const SAMPLE_FILE_B: &str = "DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level\n\
DDInter1,Abacavir,DDInter1348,Orlistat,Moderate\n\
DDInter513,Dexamethasone,DDInter582,Dolutegravir,Minor\n";

    const SAMPLE_WITH_QUOTED_NAME: &str = "DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level\n\
DDInter1112,\"Thyroid, porcine\",DDInter513,Dexamethasone,Moderate\n";

    #[test]
    fn imports_drugs_and_interactions_from_a_single_file() {
        let (drugs, interactions) = import_ddinter(vec![SAMPLE_FILE_A.as_bytes()]).unwrap();
        assert_eq!(drugs.len(), 3);
        assert_eq!(interactions.len(), 2);
        assert!(drugs
            .iter()
            .any(|d| d.inn_name().as_str() == "Dexamethasone"));
    }

    #[test]
    fn deduplicates_a_pair_repeated_across_files() {
        let (drugs, interactions) =
            import_ddinter(vec![SAMPLE_FILE_A.as_bytes(), SAMPLE_FILE_B.as_bytes()]).unwrap();
        // Dexamethasone+Dolutegravir appears in both files; Abacavir+Orlistat
        // and Dolutegravir+Elagolix are each unique -- three distinct pairs.
        assert_eq!(interactions.len(), 3);
        assert_eq!(drugs.len(), 5);
    }

    #[test]
    fn parses_a_quoted_drug_name_containing_a_comma() {
        let (drugs, _) = import_ddinter(vec![SAMPLE_WITH_QUOTED_NAME.as_bytes()]).unwrap();
        assert!(drugs
            .iter()
            .any(|d| d.inn_name().as_str() == "Thyroid, porcine"));
    }

    #[test]
    fn maps_major_to_contraindicated_and_synthesizes_a_caution() {
        let csv = "DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level\nDDInter1,Aspirin,DDInter2,Warfarin,Major\n";
        let (_, interactions) = import_ddinter(vec![csv.as_bytes()]).unwrap();
        assert_eq!(interactions[0].severity(), Severity::Contraindicated);
        assert_eq!(interactions[0].source(), DDINTER_SOURCE);
        assert!(!interactions[0].description().is_empty());
    }

    #[test]
    fn rejects_an_unrecognized_level() {
        let csv = "DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level\nDDInter1,Aspirin,DDInter2,Warfarin,Severe\n";
        let err = import_ddinter(vec![csv.as_bytes()]).unwrap_err();
        assert!(matches!(err, ImportError::UnrecognizedLevel(level) if level == "Severe"));
    }

    #[test]
    fn rejects_a_malformed_ddinter_id() {
        let csv = "DDInterID_A,Drug_A,DDInterID_B,Drug_B,Level\nAspirinID,Aspirin,DDInter2,Warfarin,Minor\n";
        let err = import_ddinter(vec![csv.as_bytes()]).unwrap_err();
        assert!(matches!(err, ImportError::MalformedId(id) if id == "AspirinID"));
    }

    #[test]
    fn output_opens_and_round_trips_through_mensung_db() {
        let (drugs, interactions) =
            import_ddinter(vec![SAMPLE_FILE_A.as_bytes(), SAMPLE_FILE_B.as_bytes()]).unwrap();
        let (bytes, report) = crate::build_database(drugs, interactions).unwrap();
        assert_eq!(report.errors, 0);
        let db = Database::open(&bytes).unwrap();
        assert_eq!(db.drug_count(), 5);
    }
}

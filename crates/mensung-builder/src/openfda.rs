//! Imports OpenFDA drug label API records (`api.fda.gov/drug/label.json`)
//! into `mensung_domain::DrugFact` values, matched by name against an
//! existing INN drug list. Field names and shapes below are verified
//! against a real live response and FDA's own published schema
//! (github.com/FDA/openfda/blob/master/schemas/druglabel_schema.json), not
//! assumed; see MEDICAL_DATA_POLICY.md's Trust and Conflict Resolution
//! section for how these facts combine with DDInter's.
//!
//! OpenFDA's full drug/label bulk export is 260,530 records across 14
//! zipped JSON files, about 1.8GB compressed (checked directly via
//! `api.fda.gov/download.json`). Almost none of that is relevant here:
//! this only enriches drugs MenSung already has from DDInter, so records
//! are fetched one drug at a time through the live search API (see
//! `openfda_download.rs`) instead of ingesting the entire bulk export.
//!
//! Name matching: `openfda.generic_name` is the label's official substance
//! name, but it usually includes the salt or ester form DDInter's INN name
//! does not ("WARFARIN SODIUM" vs "Warfarin"). A false match here would
//! silently attach one drug's contraindications to a different drug, so
//! this only accepts a match when the INN name's words are an exact,
//! case-insensitive prefix of `generic_name`'s words, never a substring or
//! fuzzy match. Anything that does not match this way is skipped, not
//! guessed.
//!
//! Field mapping, verified against FDA's schema:
//! - `DrugFactKind::Contraindication` <- `contraindications`
//! - `DrugFactKind::BoxedWarning` <- `boxed_warning`
//! - `DrugFactKind::Warning` <- `warnings_and_cautions`, falling back to
//!   the older `warnings` field when the newer one is absent
//! - `DrugFactKind::Pregnancy` <- `pregnancy`, falling back to the older
//!   combined `pregnancy_or_breast_feeding` field
//! - `DrugFactKind::Breastfeeding` <- `nursing_mothers` only; labels using
//!   FDA's newer format fold this into a subsection of
//!   `use_in_specific_populations` with no dedicated field, and extracting
//!   it from that free text would be a guess, so no `Breastfeeding` fact
//!   is produced for those labels
//! - `DrugFactKind::Dosage` <- `dosage_and_administration`
//! - `DrugFactKind::Indication` <- `indications_and_usage`
//!
//! Severity is not a structured value in label text. Each `DrugFactKind`
//! is given a fixed, conservative severity reflecting what that kind of
//! fact means clinically, not an assessment of the specific text: see
//! `default_severity`. This is a documented default, not a silent
//! inference from free text.

use mensung_domain::{
    Claim, ClaimDate, Confidence, DomainError, Drug, DrugFact, DrugFactId, DrugFactKind, DrugId,
    EvidenceLevel, Severity, Source, SourceId, SourceTier,
};
use serde::Deserialize;

pub const OPENFDA_SOURCE_ID: &str = "openfda-label";
const OPENFDA_SOURCE_NAME: &str = "OpenFDA Drug Labeling";

#[derive(Debug, thiserror::Error)]
pub enum OpenFdaImportError {
    #[error("failed to parse an OpenFDA label response: {0}")]
    Json(#[from] serde_json::Error),

    #[error("drug data error: {0}")]
    Domain(#[from] DomainError),
}

#[derive(Debug, Default, Deserialize)]
struct OpenFdaResponse {
    #[serde(default)]
    results: Vec<OpenFdaRecord>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenFdaRecord {
    #[serde(default)]
    openfda: OpenFdaAnnotation,
    #[serde(default)]
    effective_time: Option<String>,
    #[serde(default)]
    contraindications: Vec<String>,
    #[serde(default)]
    boxed_warning: Vec<String>,
    #[serde(default)]
    warnings_and_cautions: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    pregnancy: Vec<String>,
    #[serde(default)]
    pregnancy_or_breast_feeding: Vec<String>,
    #[serde(default)]
    nursing_mothers: Vec<String>,
    #[serde(default)]
    dosage_and_administration: Vec<String>,
    #[serde(default)]
    indications_and_usage: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenFdaAnnotation {
    #[serde(default)]
    generic_name: Vec<String>,
}

pub fn openfda_source() -> Source {
    Source::new(
        SourceId::parse(OPENFDA_SOURCE_ID).expect("OPENFDA_SOURCE_ID is a valid slug literal"),
        OPENFDA_SOURCE_NAME,
        SourceTier::CuratedDatabase,
    )
    .expect("OPENFDA_SOURCE_NAME is a valid, non-empty literal")
}

fn default_severity(kind: DrugFactKind) -> Severity {
    match kind {
        DrugFactKind::Contraindication => Severity::Contraindicated,
        DrugFactKind::BoxedWarning => Severity::HighRisk,
        DrugFactKind::Warning => Severity::Moderate,
        DrugFactKind::Pregnancy | DrugFactKind::Breastfeeding => Severity::Moderate,
        DrugFactKind::Dosage | DrugFactKind::Indication => Severity::Minor,
    }
}

/// True only when every word of `inn_name` matches, in order and
/// case-insensitively, a prefix of `generic_name`'s words. Never a
/// substring or fuzzy match; see this module's header for why.
fn matches_inn(generic_name: &str, inn_name: &str) -> bool {
    let inn_words: Vec<&str> = inn_name.split_whitespace().collect();
    if inn_words.is_empty() {
        return false;
    }

    generic_name
        .split_whitespace()
        .zip(inn_words.iter())
        .filter(|(generic_word, inn_word)| generic_word.eq_ignore_ascii_case(inn_word))
        .count()
        == inn_words.len()
}

fn find_matching_drug<'a>(generic_names: &[String], drugs: &'a [Drug]) -> Option<&'a Drug> {
    generic_names.iter().find_map(|generic_name| {
        drugs
            .iter()
            .find(|drug| matches_inn(generic_name, drug.inn_name().as_str()))
    })
}

fn parse_effective_time(raw: &str) -> Option<ClaimDate> {
    if raw.len() != 8 || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year: u16 = raw.get(0..4)?.parse().ok()?;
    let month: u8 = raw.get(4..6)?.parse().ok()?;
    let day: u8 = raw.get(6..8)?.parse().ok()?;
    ClaimDate::new(year, month, day).ok()
}

fn join_text(paragraphs: &[String]) -> Option<String> {
    let joined = paragraphs.join("\n\n");
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn build_claim(
    source: &Source,
    kind: DrugFactKind,
    text: Option<String>,
    last_updated: ClaimDate,
) -> Option<Claim> {
    Claim::new(
        source.clone(),
        default_severity(kind),
        EvidenceLevel::Established,
        Confidence::Medium,
        text?,
        last_updated,
    )
    .ok()
}

fn record_to_facts(
    record: &OpenFdaRecord,
    drug_id: DrugId,
    source: &Source,
    next_id: &mut u32,
) -> Vec<DrugFact> {
    let Some(last_updated) = record
        .effective_time
        .as_deref()
        .and_then(parse_effective_time)
    else {
        return Vec::new();
    };

    let warnings = if record.warnings_and_cautions.is_empty() {
        &record.warnings
    } else {
        &record.warnings_and_cautions
    };
    let pregnancy = if record.pregnancy.is_empty() {
        &record.pregnancy_or_breast_feeding
    } else {
        &record.pregnancy
    };

    let fields: [(DrugFactKind, &[String]); 7] = [
        (DrugFactKind::Contraindication, &record.contraindications),
        (DrugFactKind::BoxedWarning, &record.boxed_warning),
        (DrugFactKind::Warning, warnings),
        (DrugFactKind::Pregnancy, pregnancy),
        (DrugFactKind::Breastfeeding, &record.nursing_mothers),
        (DrugFactKind::Dosage, &record.dosage_and_administration),
        (DrugFactKind::Indication, &record.indications_and_usage),
    ];

    fields
        .into_iter()
        .filter_map(|(kind, paragraphs)| {
            let claim = build_claim(source, kind, join_text(paragraphs), last_updated)?;
            let id = DrugFactId::new(*next_id);
            *next_id += 1;
            DrugFact::new(id, drug_id, kind, vec![claim]).ok()
        })
        .collect()
}

/// Parses one or more raw OpenFDA `drug/label.json` response bodies and
/// matches each record against `drugs` by name, producing one `DrugFact`
/// per non-empty, mappable label field. Records that do not match any
/// known drug, or that have no parseable `effective_time`, are skipped,
/// not guessed at.
pub fn import_openfda_labels(
    responses: &[String],
    drugs: &[Drug],
) -> Result<Vec<DrugFact>, OpenFdaImportError> {
    let source = openfda_source();
    let mut facts = Vec::new();
    let mut next_id: u32 = 0;

    for body in responses {
        let parsed: OpenFdaResponse = serde_json::from_str(body)?;
        for record in &parsed.results {
            let Some(drug) = find_matching_drug(&record.openfda.generic_name, drugs) else {
                continue;
            };
            facts.extend(record_to_facts(record, drug.id(), &source, &mut next_id));
        }
    }

    Ok(facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::InnName;

    fn drug(id: u32, name: &str) -> Drug {
        Drug::new(DrugId::new(id), InnName::parse(name).unwrap())
    }

    // A trimmed but otherwise real record shape, captured from a live
    // api.fda.gov/drug/label.json response for warfarin (verified 2026-07,
    // see this module's header), with the long free-text fields shortened.
    const WARFARIN_RESPONSE: &str = r#"{
        "results": [{
            "effective_time": "20250617",
            "openfda": { "generic_name": ["WARFARIN SODIUM"] },
            "contraindications": ["4 CONTRAINDICATIONS Warfarin sodium is contraindicated in pregnancy."],
            "boxed_warning": ["WARNING: BLEEDING RISK Warfarin sodium can cause major or fatal bleeding."],
            "warnings_and_cautions": ["5 WARNINGS AND CAUTIONS Necrosis of skin and other tissues."],
            "pregnancy": ["8.1 Pregnancy Risk Summary Warfarin sodium can cause fetal harm."],
            "dosage_and_administration": ["2 DOSAGE AND ADMINISTRATION Individualize dosing based on INR."],
            "indications_and_usage": ["1 INDICATIONS AND USAGE Warfarin sodium is indicated for prophylaxis and treatment of thromboembolism."]
        }]
    }"#;

    #[test]
    fn matches_a_salt_form_generic_name_to_its_inn() {
        assert!(matches_inn("WARFARIN SODIUM", "Warfarin"));
        assert!(matches_inn("Acetylsalicylic Acid", "Acetylsalicylic acid"));
    }

    #[test]
    fn does_not_match_an_unrelated_drug() {
        assert!(!matches_inn("IBUPROFEN", "Warfarin"));
        assert!(!matches_inn("WARFARINOL", "Warfarin"));
    }

    #[test]
    fn parses_effective_time_in_yyyymmdd_form() {
        assert_eq!(
            parse_effective_time("20250617"),
            Some(ClaimDate::new(2025, 6, 17).unwrap())
        );
        assert_eq!(parse_effective_time("not-a-date"), None);
        assert_eq!(parse_effective_time("202506"), None);
    }

    #[test]
    fn imports_a_real_warfarin_record_into_six_drug_facts() {
        let drugs = vec![drug(0, "Warfarin")];
        let facts = import_openfda_labels(&[WARFARIN_RESPONSE.to_string()], &drugs).unwrap();

        assert_eq!(facts.len(), 6);
        assert!(facts.iter().all(|fact| fact.drug() == DrugId::new(0)));

        let contraindication = facts
            .iter()
            .find(|fact| fact.kind() == DrugFactKind::Contraindication)
            .expect("a contraindication fact should be produced");
        assert_eq!(
            contraindication.primary_claim().severity(),
            Severity::Contraindicated
        );
        assert_eq!(
            contraindication.primary_claim().source().id().as_str(),
            OPENFDA_SOURCE_ID
        );
        assert_eq!(
            contraindication.primary_claim().last_updated(),
            ClaimDate::new(2025, 6, 17).unwrap()
        );

        let boxed = facts
            .iter()
            .find(|fact| fact.kind() == DrugFactKind::BoxedWarning)
            .expect("a boxed warning fact should be produced");
        assert_eq!(boxed.primary_claim().severity(), Severity::HighRisk);

        // No nursing_mothers field in this record, so no Breastfeeding
        // fact is produced -- skipped, not guessed at.
        assert!(!facts
            .iter()
            .any(|fact| fact.kind() == DrugFactKind::Breastfeeding));
    }

    #[test]
    fn skips_a_record_that_does_not_match_any_known_drug() {
        let drugs = vec![drug(0, "Ibuprofen")];
        let facts = import_openfda_labels(&[WARFARIN_RESPONSE.to_string()], &drugs).unwrap();
        assert!(facts.is_empty());
    }

    #[test]
    fn skips_a_record_with_no_parseable_effective_time() {
        let response = r#"{"results": [{
            "openfda": { "generic_name": ["WARFARIN SODIUM"] },
            "contraindications": ["some text"]
        }]}"#;
        let drugs = vec![drug(0, "Warfarin")];
        let facts = import_openfda_labels(&[response.to_string()], &drugs).unwrap();
        assert!(facts.is_empty());
    }

    #[test]
    fn falls_back_to_the_older_warnings_field_when_warnings_and_cautions_is_absent() {
        let response = r#"{"results": [{
            "effective_time": "20200101",
            "openfda": { "generic_name": ["ASPIRIN"] },
            "warnings": ["Reye's syndrome warning."]
        }]}"#;
        let drugs = vec![drug(0, "Aspirin")];
        let facts = import_openfda_labels(&[response.to_string()], &drugs).unwrap();
        let warning = facts
            .iter()
            .find(|fact| fact.kind() == DrugFactKind::Warning)
            .expect("the older warnings field should still produce a Warning fact");
        assert!(warning.primary_claim().rationale().contains("Reye"));
    }
}

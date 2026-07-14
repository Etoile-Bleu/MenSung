//! Compiles a validated set of drugs, interaction facts, and drug facts
//! into a .men byte buffer, following docs/DATABASE_FORMAT.md exactly.
//! Assumes the caller already validated the dataset with
//! `validate::validate` and got back no issues; this does not re-check
//! duplicate names or dangling references, it only encodes what it is
//! given. `build_timestamp` honors `SOURCE_DATE_EPOCH` when set, so
//! builds stay byte-for-byte reproducible in CI and release pipelines
//! that set it; it falls back to wall-clock time for local,
//! non-reproducible development builds.
//!
//! Every string (drug names, RxCUIs, chemical properties, ATC class
//! names, claim source ids/names/rationale) is interned through one
//! shared cache, so a string repeated across many records, DDInter's
//! synthesized descriptions across most of its 160,235 interactions
//! being the motivating case (see ROADMAP.md's Phase 5 tradeoff note),
//! is written to the String Table once and referenced by every record
//! that uses it, not duplicated per record the way format version 1 did.

use std::collections::HashMap;

use mensung_domain::{
    Claim, Confidence, Drug, DrugFact, DrugFactKind, EvidenceLevel, InteractionFact, Severity,
    SourceTier,
};
use sha2::{Digest, Sha256};

const HEADER_LEN: usize = 192;

/// Interns strings into a single growing buffer, returning the same
/// `(offset, length)` for a string already seen rather than writing a
/// second copy. Offsets are stable once issued: this only ever appends,
/// so earlier callers' offsets stay valid no matter what is interned
/// afterward.
#[derive(Default)]
struct StringPool {
    bytes: Vec<u8>,
    seen: HashMap<String, (u32, u32)>,
}

impl StringPool {
    fn intern(&mut self, s: &str) -> (u32, u32) {
        if let Some(&existing) = self.seen.get(s) {
            return existing;
        }
        let offset = self.bytes.len() as u32;
        self.bytes.extend_from_slice(s.as_bytes());
        let len = s.len() as u32;
        self.seen.insert(s.to_string(), (offset, len));
        (offset, len)
    }
}

fn severity_byte(severity: Severity) -> u8 {
    match severity {
        Severity::Contraindicated => 0,
        Severity::HighRisk => 1,
        Severity::Moderate => 2,
        Severity::Minor => 3,
        Severity::Unknown => 4,
    }
}

fn evidence_byte(evidence: EvidenceLevel) -> u8 {
    match evidence {
        EvidenceLevel::Established => 0,
        EvidenceLevel::Probable => 1,
        EvidenceLevel::Theoretical => 2,
    }
}

fn confidence_byte(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Low => 0,
        Confidence::Medium => 1,
        Confidence::High => 2,
    }
}

fn tier_byte(tier: SourceTier) -> u8 {
    match tier {
        SourceTier::Regulatory => 0,
        SourceTier::ClinicalGuideline => 1,
        SourceTier::CuratedDatabase => 2,
        SourceTier::Secondary => 3,
    }
}

fn drug_fact_kind_byte(kind: DrugFactKind) -> u8 {
    match kind {
        DrugFactKind::Contraindication => 0,
        DrugFactKind::Warning => 1,
        DrugFactKind::BoxedWarning => 2,
        DrugFactKind::Pregnancy => 3,
        DrugFactKind::Breastfeeding => 4,
        DrugFactKind::Dosage => 5,
        DrugFactKind::Indication => 6,
    }
}

fn encode_claim(pool: &mut StringPool, claim: &Claim) -> Vec<u8> {
    let (source_id_offset, source_id_len) = pool.intern(claim.source().id().as_str());
    let (source_name_offset, source_name_len) = pool.intern(claim.source().name());
    let (rationale_offset, rationale_len) = pool.intern(claim.rationale());

    let mut bytes = Vec::with_capacity(28);
    bytes.extend_from_slice(&source_id_offset.to_le_bytes());
    bytes.extend_from_slice(&(source_id_len as u16).to_le_bytes());
    bytes.extend_from_slice(&source_name_offset.to_le_bytes());
    bytes.extend_from_slice(&(source_name_len as u16).to_le_bytes());
    bytes.push(tier_byte(claim.source().tier()));
    bytes.push(severity_byte(claim.severity()));
    bytes.push(evidence_byte(claim.evidence()));
    bytes.push(confidence_byte(claim.confidence()));
    bytes.extend_from_slice(&claim.last_updated().year().to_le_bytes());
    bytes.push(claim.last_updated().month());
    bytes.push(claim.last_updated().day());
    bytes.extend_from_slice(&rationale_offset.to_le_bytes());
    bytes.extend_from_slice(&rationale_len.to_le_bytes());
    bytes
}

fn build_timestamp() -> u64 {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        })
}

pub fn compile(
    mut drugs: Vec<Drug>,
    interactions: &[InteractionFact],
    drug_facts: &[DrugFact],
) -> Vec<u8> {
    drugs.sort_by(|a, b| a.inn_name().as_str().cmp(b.inn_name().as_str()));

    let mut pool = StringPool::default();
    let mut drug_table = Vec::new();
    let mut atc_table = Vec::new();

    for drug in &drugs {
        let (name_offset, name_len) = pool.intern(drug.inn_name().as_str());
        let (rxcui_offset, rxcui_len) = drug
            .rxcui()
            .map(|r| pool.intern(r.as_str()))
            .unwrap_or((0, 0));
        let pubchem_cid = drug
            .chemical_properties()
            .map(|p| p.cid().value())
            .unwrap_or(0);
        let (formula_offset, formula_len) = drug
            .chemical_properties()
            .map(|p| pool.intern(p.molecular_formula()))
            .unwrap_or((0, 0));
        let (weight_offset, weight_len) = drug
            .chemical_properties()
            .map(|p| pool.intern(p.molecular_weight()))
            .unwrap_or((0, 0));
        let (iupac_offset, iupac_len) = drug
            .chemical_properties()
            .and_then(|p| p.iupac_name())
            .map(|name| pool.intern(name))
            .unwrap_or((0, 0));

        let atc_start_index = (atc_table.len() / 12) as u32;
        for atc in drug.atc_codes() {
            let (class_offset, class_len) = pool.intern(atc.class_name());
            atc_table.extend_from_slice(atc.code().as_bytes());
            atc_table.push(0);
            atc_table.extend_from_slice(&class_offset.to_le_bytes());
            atc_table.extend_from_slice(&(class_len as u16).to_le_bytes());
        }
        let atc_count = drug.atc_codes().len() as u16;

        drug_table.extend_from_slice(&drug.id().value().to_le_bytes());
        drug_table.extend_from_slice(&name_offset.to_le_bytes());
        drug_table.extend_from_slice(&(name_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&0u16.to_le_bytes());
        drug_table.extend_from_slice(&rxcui_offset.to_le_bytes());
        drug_table.extend_from_slice(&(rxcui_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&pubchem_cid.to_le_bytes());
        drug_table.extend_from_slice(&formula_offset.to_le_bytes());
        drug_table.extend_from_slice(&(formula_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&weight_offset.to_le_bytes());
        drug_table.extend_from_slice(&(weight_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&iupac_offset.to_le_bytes());
        drug_table.extend_from_slice(&(iupac_len as u16).to_le_bytes());
        drug_table.extend_from_slice(&atc_start_index.to_le_bytes());
        drug_table.extend_from_slice(&atc_count.to_le_bytes());
        drug_table.extend_from_slice(&0u16.to_le_bytes());
    }

    let mut sorted_interactions: Vec<&InteractionFact> = interactions.iter().collect();
    sorted_interactions.sort_by_key(|fact| fact.pair().drugs());

    let mut interaction_index = Vec::new();
    let mut interaction_records = Vec::new();
    for fact in &sorted_interactions {
        let (lower, higher) = fact.pair().drugs();
        let record_offset = interaction_records.len() as u32;

        let mut record = Vec::new();
        record.extend_from_slice(&fact.id().value().to_le_bytes());
        record.extend_from_slice(&lower.value().to_le_bytes());
        record.extend_from_slice(&higher.value().to_le_bytes());
        record.extend_from_slice(&(fact.claims().len() as u16).to_le_bytes());
        record.extend_from_slice(&0u16.to_le_bytes());
        for claim in fact.claims() {
            record.extend_from_slice(&encode_claim(&mut pool, claim));
        }

        interaction_index.extend_from_slice(&lower.value().to_le_bytes());
        interaction_index.extend_from_slice(&higher.value().to_le_bytes());
        interaction_index.extend_from_slice(&record_offset.to_le_bytes());
        interaction_index.extend_from_slice(&(record.len() as u32).to_le_bytes());

        interaction_records.extend_from_slice(&record);
    }

    let mut sorted_facts: Vec<&DrugFact> = drug_facts.iter().collect();
    sorted_facts.sort_by_key(|fact| (fact.drug().value(), drug_fact_kind_byte(fact.kind())));

    let mut drug_fact_index = Vec::new();
    let mut drug_fact_records = Vec::new();
    for fact in &sorted_facts {
        let record_offset = drug_fact_records.len() as u32;

        let mut record = Vec::new();
        record.extend_from_slice(&fact.id().value().to_le_bytes());
        record.extend_from_slice(&fact.drug().value().to_le_bytes());
        record.push(drug_fact_kind_byte(fact.kind()));
        record.push(0);
        record.extend_from_slice(&(fact.claims().len() as u16).to_le_bytes());
        for claim in fact.claims() {
            record.extend_from_slice(&encode_claim(&mut pool, claim));
        }

        drug_fact_index.extend_from_slice(&fact.drug().value().to_le_bytes());
        drug_fact_index.push(drug_fact_kind_byte(fact.kind()));
        drug_fact_index.extend_from_slice(&[0u8; 3]);
        drug_fact_index.extend_from_slice(&record_offset.to_le_bytes());
        drug_fact_index.extend_from_slice(&(record.len() as u32).to_le_bytes());

        drug_fact_records.extend_from_slice(&record);
    }

    let string_table = pool.bytes;
    let string_table_offset = HEADER_LEN as u64;
    let string_table_len = string_table.len() as u64;
    let drug_table_offset = string_table_offset + string_table_len;
    let drug_table_len = drug_table.len() as u64;
    let atc_table_offset = drug_table_offset + drug_table_len;
    let atc_table_len = atc_table.len() as u64;
    let interaction_index_offset = atc_table_offset + atc_table_len;
    let interaction_index_len = interaction_index.len() as u64;
    let interaction_records_offset = interaction_index_offset + interaction_index_len;
    let interaction_records_len = interaction_records.len() as u64;
    let drug_fact_index_offset = interaction_records_offset + interaction_records_len;
    let drug_fact_index_len = drug_fact_index.len() as u64;
    let drug_fact_records_offset = drug_fact_index_offset + drug_fact_index_len;
    let drug_fact_records_len = drug_fact_records.len() as u64;

    let mut payload = Vec::new();
    payload.extend_from_slice(&string_table);
    payload.extend_from_slice(&drug_table);
    payload.extend_from_slice(&atc_table);
    payload.extend_from_slice(&interaction_index);
    payload.extend_from_slice(&interaction_records);
    payload.extend_from_slice(&drug_fact_index);
    payload.extend_from_slice(&drug_fact_records);

    let payload_sha256: [u8; 32] = Sha256::digest(&payload).into();

    let mut header = vec![0u8; HEADER_LEN];
    header[0..4].copy_from_slice(b"MEN1");
    header[4..6].copy_from_slice(&2u16.to_le_bytes());
    header[6..8].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
    header[12..20].copy_from_slice(&build_timestamp().to_le_bytes());
    header[20..52].copy_from_slice(&payload_sha256);
    header[52..56].copy_from_slice(&(drugs.len() as u32).to_le_bytes());
    header[56..60].copy_from_slice(&(sorted_interactions.len() as u32).to_le_bytes());
    header[60..68].copy_from_slice(&string_table_offset.to_le_bytes());
    header[68..76].copy_from_slice(&string_table_len.to_le_bytes());
    header[76..84].copy_from_slice(&drug_table_offset.to_le_bytes());
    header[84..92].copy_from_slice(&drug_table_len.to_le_bytes());
    header[92..100].copy_from_slice(&interaction_index_offset.to_le_bytes());
    header[100..108].copy_from_slice(&interaction_index_len.to_le_bytes());
    header[108..116].copy_from_slice(&interaction_records_offset.to_le_bytes());
    header[116..124].copy_from_slice(&interaction_records_len.to_le_bytes());
    header[124..128].copy_from_slice(&[0u8; 4]);
    header[128..136].copy_from_slice(&atc_table_offset.to_le_bytes());
    header[136..144].copy_from_slice(&atc_table_len.to_le_bytes());
    header[144..148].copy_from_slice(&((atc_table.len() / 12) as u32).to_le_bytes());
    header[148..156].copy_from_slice(&drug_fact_index_offset.to_le_bytes());
    header[156..164].copy_from_slice(&drug_fact_index_len.to_le_bytes());
    header[164..168].copy_from_slice(&(sorted_facts.len() as u32).to_le_bytes());
    header[168..176].copy_from_slice(&drug_fact_records_offset.to_le_bytes());
    header[176..184].copy_from_slice(&drug_fact_records_len.to_le_bytes());
    header[184..192].copy_from_slice(&[0u8; 8]);

    let crc = {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&header[0..8]);
        hasher.update(&header[12..]);
        hasher.finalize()
    };
    header[8..12].copy_from_slice(&crc.to_le_bytes());

    let mut full = header;
    full.extend_from_slice(&payload);
    full
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_domain::{
        AtcCode, ChemicalProperties, ClaimDate, Drug, DrugFactId, DrugId, DrugPair, EvidenceLevel,
        InnName, InteractionId, PubchemCid, Rxcui, Source, SourceId,
    };

    fn ddinter_source() -> Source {
        Source::new(
            SourceId::parse("ddinter").unwrap(),
            "DDInter",
            SourceTier::CuratedDatabase,
        )
        .unwrap()
    }

    fn claim(severity: Severity, rationale: &str) -> Claim {
        Claim::new(
            ddinter_source(),
            severity,
            EvidenceLevel::Established,
            Confidence::Medium,
            rationale,
            ClaimDate::new(2026, 7, 14).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn compiled_output_opens_and_round_trips_through_mensung_db() {
        let aspirin = DrugId::new(0);
        let warfarin = DrugId::new(1);
        let drugs = vec![
            Drug::new(aspirin, InnName::parse("Aspirin").unwrap()),
            Drug::new(warfarin, InnName::parse("Warfarin").unwrap())
                .with_rxcui(Rxcui::parse("11289").unwrap())
                .with_chemical_properties(
                    ChemicalProperties::new(PubchemCid::new(54678486), "C19H16O4", "308.3", None)
                        .unwrap(),
                )
                .with_atc_codes(vec![AtcCode::new("B01AA", "Vitamin K antagonists").unwrap()]),
        ];
        let interactions = vec![InteractionFact::new(
            InteractionId::new(0),
            DrugPair::new(aspirin, warfarin).unwrap(),
            vec![claim(
                Severity::Contraindicated,
                "Increased bleeding and hemorrhage probability.",
            )],
        )
        .unwrap()];
        let drug_facts = vec![DrugFact::new(
            DrugFactId::new(0),
            warfarin,
            DrugFactKind::BoxedWarning,
            vec![claim(
                Severity::Contraindicated,
                "Warfarin sodium can cause major or fatal bleeding.",
            )],
        )
        .unwrap()];

        let bytes = compile(drugs, &interactions, &drug_facts);
        let db = mensung_db::Database::open(&bytes).unwrap();

        assert_eq!(db.drug_count(), 2);
        let drug = db.find_drug_by_name("Warfarin").unwrap().unwrap();
        assert_eq!(drug.rxcui(), Some("11289"));
        assert_eq!(drug.molecular_formula(), Some("C19H16O4"));
        assert_eq!(drug.atc_codes().count(), 1);

        let pair = DrugPair::new(drug.id(), aspirin).unwrap();
        let interaction = db.find_interaction(pair).unwrap().unwrap();
        assert_eq!(interaction.severity(), Severity::Contraindicated);

        let facts = db.drug_facts(warfarin).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].kind(), DrugFactKind::BoxedWarning);
    }

    #[test]
    fn identical_rationale_text_is_written_to_the_string_table_once() {
        let a = DrugId::new(0);
        let b = DrugId::new(1);
        let c = DrugId::new(2);
        let drugs = vec![
            Drug::new(a, InnName::parse("Aspirin").unwrap()),
            Drug::new(b, InnName::parse("Warfarin").unwrap()),
            Drug::new(c, InnName::parse("Ibuprofen").unwrap()),
        ];
        let repeated_text = "DDInter classifies this as a minor interaction.";
        let interactions = vec![
            InteractionFact::new(
                InteractionId::new(0),
                DrugPair::new(a, b).unwrap(),
                vec![claim(Severity::Minor, repeated_text)],
            )
            .unwrap(),
            InteractionFact::new(
                InteractionId::new(1),
                DrugPair::new(a, c).unwrap(),
                vec![claim(Severity::Minor, repeated_text)],
            )
            .unwrap(),
        ];

        let bytes = compile(drugs, &interactions, &[]);
        let db = mensung_db::Database::open(&bytes).unwrap();

        let pair_ab = DrugPair::new(a, b).unwrap();
        let pair_ac = DrugPair::new(a, c).unwrap();
        let interaction_ab = db.find_interaction(pair_ab).unwrap().unwrap();
        let interaction_ac = db.find_interaction(pair_ac).unwrap().unwrap();
        assert_eq!(interaction_ab.description(), repeated_text);
        assert_eq!(interaction_ac.description(), repeated_text);

        // Both records' claims should point at the exact same String
        // Table bytes rather than two separate copies of the same text.
        let combined_len = bytes.len();
        let single_copy_extra = repeated_text.len();
        // A crude but effective duplication check: search the whole file
        // for a second, non-overlapping occurrence of the repeated text.
        let haystack = &bytes[..];
        let needle = repeated_text.as_bytes();
        let mut occurrences = 0;
        let mut search_from = 0;
        while let Some(pos) = haystack[search_from..]
            .windows(needle.len())
            .position(|window| window == needle)
        {
            occurrences += 1;
            search_from += pos + needle.len();
        }
        assert_eq!(
            occurrences, 1,
            "the repeated rationale text should appear exactly once in the compiled file"
        );
        let _ = (combined_len, single_copy_extra);
    }
}

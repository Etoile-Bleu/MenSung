//! Reads Drug Table records: fixed 48-byte entries sorted by INN name bytes
//! for binary search, as defined in docs/DATABASE_FORMAT.md. Carries the
//! optional RxCUI, PubChem chemical properties, and WHO ATC codes a build
//! may have attached to a drug, alongside its id and name.

use mensung_domain::DrugId;

use crate::atc_table::AtcCodeIter;
use crate::bytes::{read_u16, read_u32};
use crate::DbError;

pub(crate) const RECORD_LEN: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrugRecord<'a> {
    id: DrugId,
    name: &'a str,
    rxcui: Option<&'a str>,
    pubchem_cid: Option<u32>,
    molecular_formula: Option<&'a str>,
    molecular_weight: Option<&'a str>,
    iupac_name: Option<&'a str>,
    atc_start_index: u32,
    atc_count: u16,
    atc_table: &'a [u8],
    strings: &'a [u8],
}

impl<'a> DrugRecord<'a> {
    pub fn id(&self) -> DrugId {
        self.id
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn rxcui(&self) -> Option<&'a str> {
        self.rxcui
    }

    pub fn pubchem_cid(&self) -> Option<u32> {
        self.pubchem_cid
    }

    pub fn molecular_formula(&self) -> Option<&'a str> {
        self.molecular_formula
    }

    pub fn molecular_weight(&self) -> Option<&'a str> {
        self.molecular_weight
    }

    pub fn iupac_name(&self) -> Option<&'a str> {
        self.iupac_name
    }

    pub fn atc_codes(&self) -> AtcCodeIter<'a> {
        AtcCodeIter {
            table: self.atc_table,
            strings: self.strings,
            next_index: self.atc_start_index,
            remaining: self.atc_count,
        }
    }
}

fn optional_string(strings: &[u8], offset: u32, len: u16) -> Result<Option<&str>, DbError> {
    if len == 0 {
        return Ok(None);
    }
    let start = offset as usize;
    let end = start + len as usize;
    let bytes = strings.get(start..end).ok_or(DbError::Truncated)?;
    let text = std::str::from_utf8(bytes).map_err(|_| DbError::InvalidStringTableEntry)?;
    Ok(Some(text))
}

pub(crate) fn parse_record<'a>(
    table: &[u8],
    strings: &'a [u8],
    atc_table: &'a [u8],
    index: u32,
) -> Result<DrugRecord<'a>, DbError> {
    let start = index as usize * RECORD_LEN;
    let record = table
        .get(start..start + RECORD_LEN)
        .ok_or(DbError::Truncated)?;

    let drug_id = read_u32(record, 0)?;
    let name_offset = read_u32(record, 4)?;
    let name_len = read_u16(record, 8)?;
    let name_bytes = strings
        .get(name_offset as usize..name_offset as usize + name_len as usize)
        .ok_or(DbError::Truncated)?;
    let name = std::str::from_utf8(name_bytes).map_err(|_| DbError::InvalidDrugName { index })?;

    let rxcui_offset = read_u32(record, 12)?;
    let rxcui_len = read_u16(record, 16)?;
    let rxcui = optional_string(strings, rxcui_offset, rxcui_len)?;

    let pubchem_cid_raw = read_u32(record, 18)?;
    let pubchem_cid = (pubchem_cid_raw != 0).then_some(pubchem_cid_raw);

    let molecular_formula_offset = read_u32(record, 22)?;
    let molecular_formula_len = read_u16(record, 26)?;
    let molecular_formula =
        optional_string(strings, molecular_formula_offset, molecular_formula_len)?;

    let molecular_weight_offset = read_u32(record, 28)?;
    let molecular_weight_len = read_u16(record, 32)?;
    let molecular_weight = optional_string(strings, molecular_weight_offset, molecular_weight_len)?;

    let iupac_name_offset = read_u32(record, 34)?;
    let iupac_name_len = read_u16(record, 38)?;
    let iupac_name = optional_string(strings, iupac_name_offset, iupac_name_len)?;

    let atc_start_index = read_u32(record, 40)?;
    let atc_count = read_u16(record, 44)?;

    Ok(DrugRecord {
        id: DrugId::new(drug_id),
        name,
        rxcui,
        pubchem_cid,
        molecular_formula,
        molecular_weight,
        iupac_name,
        atc_start_index,
        atc_count,
        atc_table,
        strings,
    })
}

pub(crate) fn find_by_name<'a>(
    table: &[u8],
    strings: &'a [u8],
    atc_table: &'a [u8],
    count: u32,
    target: &str,
) -> Result<Option<DrugRecord<'a>>, DbError> {
    let mut low = 0u32;
    let mut high = count;

    while low < high {
        let mid = low + (high - low) / 2;
        let record = parse_record(table, strings, atc_table, mid)?;
        match record.name().cmp(target) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Ok(Some(record)),
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn record_bytes(
        id: u32,
        name_offset: u32,
        name_len: u16,
        rxcui_offset: u32,
        rxcui_len: u16,
        pubchem_cid: u32,
        formula_offset: u32,
        formula_len: u16,
        weight_offset: u32,
        weight_len: u16,
        iupac_offset: u32,
        iupac_len: u16,
        atc_start_index: u32,
        atc_count: u16,
    ) -> Vec<u8> {
        let mut record = Vec::with_capacity(RECORD_LEN);
        record.extend_from_slice(&id.to_le_bytes());
        record.extend_from_slice(&name_offset.to_le_bytes());
        record.extend_from_slice(&name_len.to_le_bytes());
        record.extend_from_slice(&0u16.to_le_bytes());
        record.extend_from_slice(&rxcui_offset.to_le_bytes());
        record.extend_from_slice(&rxcui_len.to_le_bytes());
        record.extend_from_slice(&pubchem_cid.to_le_bytes());
        record.extend_from_slice(&formula_offset.to_le_bytes());
        record.extend_from_slice(&formula_len.to_le_bytes());
        record.extend_from_slice(&weight_offset.to_le_bytes());
        record.extend_from_slice(&weight_len.to_le_bytes());
        record.extend_from_slice(&iupac_offset.to_le_bytes());
        record.extend_from_slice(&iupac_len.to_le_bytes());
        record.extend_from_slice(&atc_start_index.to_le_bytes());
        record.extend_from_slice(&atc_count.to_le_bytes());
        record.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(record.len(), RECORD_LEN);
        record
    }

    #[test]
    fn parses_a_minimal_record_with_no_optional_data() {
        let mut strings = Vec::new();
        let offset = strings.len() as u32;
        strings.extend_from_slice(b"Aspirin");
        let record = record_bytes(0, offset, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);

        let parsed = parse_record(&record, &strings, &[], 0).unwrap();
        assert_eq!(parsed.id(), DrugId::new(0));
        assert_eq!(parsed.name(), "Aspirin");
        assert_eq!(parsed.rxcui(), None);
        assert_eq!(parsed.pubchem_cid(), None);
        assert_eq!(parsed.molecular_formula(), None);
        assert_eq!(parsed.atc_codes().count(), 0);
    }

    #[test]
    fn parses_a_fully_populated_record() {
        let mut strings = Vec::new();
        let name_offset = strings.len() as u32;
        strings.extend_from_slice(b"Warfarin");
        let rxcui_offset = strings.len() as u32;
        strings.extend_from_slice(b"11289");
        let formula_offset = strings.len() as u32;
        strings.extend_from_slice(b"C19H16O4");
        let weight_offset = strings.len() as u32;
        strings.extend_from_slice(b"308.3");
        let iupac_offset = strings.len() as u32;
        strings.extend_from_slice(b"some iupac name");

        let record = record_bytes(
            1,
            name_offset,
            8,
            rxcui_offset,
            5,
            54678486,
            formula_offset,
            8,
            weight_offset,
            5,
            iupac_offset,
            15,
            0,
            0,
        );

        let parsed = parse_record(&record, &strings, &[], 0).unwrap();
        assert_eq!(parsed.name(), "Warfarin");
        assert_eq!(parsed.rxcui(), Some("11289"));
        assert_eq!(parsed.pubchem_cid(), Some(54678486));
        assert_eq!(parsed.molecular_formula(), Some("C19H16O4"));
        assert_eq!(parsed.molecular_weight(), Some("308.3"));
        assert_eq!(parsed.iupac_name(), Some("some iupac name"));
    }

    #[test]
    fn binary_search_finds_every_entry_in_a_sorted_table() {
        let names = ["Amoxicillin", "Aspirin", "Paracetamol", "Warfarin"];
        let mut strings = Vec::new();
        let mut table = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let offset = strings.len() as u32;
            strings.extend_from_slice(name.as_bytes());
            table.extend_from_slice(&record_bytes(
                i as u32,
                offset,
                name.len() as u16,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ));
        }

        for (i, name) in names.iter().enumerate() {
            let found = find_by_name(&table, &strings, &[], names.len() as u32, name)
                .unwrap()
                .unwrap();
            assert_eq!(found.id(), DrugId::new(i as u32));
        }
    }

    #[test]
    fn binary_search_returns_none_for_a_name_not_present() {
        let names = ["Amoxicillin", "Aspirin", "Warfarin"];
        let mut strings = Vec::new();
        let mut table = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let offset = strings.len() as u32;
            strings.extend_from_slice(name.as_bytes());
            table.extend_from_slice(&record_bytes(
                i as u32,
                offset,
                name.len() as u16,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ));
        }
        assert_eq!(
            find_by_name(&table, &strings, &[], names.len() as u32, "Ibuprofen").unwrap(),
            None
        );
    }

    #[test]
    fn rejects_a_record_pointing_outside_the_string_table() {
        let record = record_bytes(0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(
            parse_record(&record, &[], &[], 0).unwrap_err(),
            DbError::Truncated
        );
    }
}

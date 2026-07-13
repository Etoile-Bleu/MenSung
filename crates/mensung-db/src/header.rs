//! Parses and validates the fixed 128-byte .men header defined in
//! docs/DATABASE_FORMAT.md: magic, version, header CRC32, and the section
//! offset table. Every other section is only read after this succeeds.

use crate::bytes::{read_u16, read_u32, read_u64};
use crate::layout;
use crate::DbError;

pub(crate) const HEADER_LEN: usize = 128;
const MAGIC: [u8; 4] = *b"MEN1";
const SUPPORTED_FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Header {
    pub(crate) header_len: usize,
    pub(crate) payload_sha256: [u8; 32],
    pub(crate) drug_count: u32,
    pub(crate) interaction_count: u32,
    pub(crate) string_table_offset: u64,
    pub(crate) string_table_len: u64,
    pub(crate) drug_table_offset: u64,
    pub(crate) drug_table_len: u64,
    pub(crate) interaction_index_offset: u64,
    pub(crate) interaction_index_len: u64,
    pub(crate) interaction_records_offset: u64,
    pub(crate) interaction_records_len: u64,
}

impl Header {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, DbError> {
        if bytes.len() < HEADER_LEN {
            return Err(DbError::Truncated);
        }

        if bytes[layout::MAGIC] != MAGIC {
            return Err(DbError::BadMagic);
        }

        let format_version = read_u16(bytes, layout::FORMAT_VERSION.start)?;
        if format_version != SUPPORTED_FORMAT_VERSION {
            return Err(DbError::UnsupportedVersion(format_version));
        }

        let header_len = read_u16(bytes, layout::HEADER_LEN_FIELD.start)? as usize;
        if header_len < HEADER_LEN {
            return Err(DbError::Truncated);
        }

        let header_bytes = bytes.get(..header_len).ok_or(DbError::Truncated)?;
        let stored_crc = read_u32(header_bytes, layout::HEADER_CRC32.start)?;
        let computed_crc = compute_header_crc32(header_bytes);
        if stored_crc != computed_crc {
            return Err(DbError::HeaderChecksumMismatch {
                expected: stored_crc,
                found: computed_crc,
            });
        }

        let payload_sha256: [u8; 32] = header_bytes
            .get(layout::PAYLOAD_SHA256)
            .ok_or(DbError::Truncated)?
            .try_into()
            .expect("slice of exactly 32 bytes");

        let header = Self {
            header_len,
            payload_sha256,
            drug_count: read_u32(bytes, layout::DRUG_COUNT.start)?,
            interaction_count: read_u32(bytes, layout::INTERACTION_COUNT.start)?,
            string_table_offset: read_u64(bytes, layout::STRING_TABLE_OFFSET.start)?,
            string_table_len: read_u64(bytes, layout::STRING_TABLE_LEN.start)?,
            drug_table_offset: read_u64(bytes, layout::DRUG_TABLE_OFFSET.start)?,
            drug_table_len: read_u64(bytes, layout::DRUG_TABLE_LEN.start)?,
            interaction_index_offset: read_u64(bytes, layout::INTERACTION_INDEX_OFFSET.start)?,
            interaction_index_len: read_u64(bytes, layout::INTERACTION_INDEX_LEN.start)?,
            interaction_records_offset: read_u64(bytes, layout::INTERACTION_RECORDS_OFFSET.start)?,
            interaction_records_len: read_u64(bytes, layout::INTERACTION_RECORDS_LEN.start)?,
        };

        header.validate_sections_within(bytes.len())?;
        Ok(header)
    }

    fn validate_sections_within(&self, total_len: usize) -> Result<(), DbError> {
        let total_len = total_len as u64;
        for (offset, len) in [
            (self.string_table_offset, self.string_table_len),
            (self.drug_table_offset, self.drug_table_len),
            (self.interaction_index_offset, self.interaction_index_len),
            (
                self.interaction_records_offset,
                self.interaction_records_len,
            ),
        ] {
            let end = offset.checked_add(len).ok_or(DbError::Truncated)?;
            if end > total_len {
                return Err(DbError::Truncated);
            }
        }
        Ok(())
    }
}

fn compute_header_crc32(header_bytes: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&header_bytes[..layout::HEADER_CRC32.start]);
    hasher.update(&header_bytes[layout::HEADER_CRC32.end..]);
    hasher.finalize()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn valid_header_bytes(sections_len: u64) -> Vec<u8> {
        let mut header = vec![0u8; HEADER_LEN];
        header[layout::MAGIC].copy_from_slice(&MAGIC);
        header[layout::FORMAT_VERSION].copy_from_slice(&SUPPORTED_FORMAT_VERSION.to_le_bytes());
        header[layout::HEADER_LEN_FIELD].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
        header[layout::BUILD_TIMESTAMP].copy_from_slice(&0u64.to_le_bytes());
        header[layout::PAYLOAD_SHA256].copy_from_slice(&[0u8; 32]);
        header[layout::DRUG_COUNT].copy_from_slice(&0u32.to_le_bytes());
        header[layout::INTERACTION_COUNT].copy_from_slice(&0u32.to_le_bytes());
        header[layout::STRING_TABLE_OFFSET].copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
        header[layout::STRING_TABLE_LEN].copy_from_slice(&sections_len.to_le_bytes());
        header[layout::DRUG_TABLE_OFFSET].copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
        header[layout::DRUG_TABLE_LEN].copy_from_slice(&0u64.to_le_bytes());
        header[layout::INTERACTION_INDEX_OFFSET]
            .copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
        header[layout::INTERACTION_INDEX_LEN].copy_from_slice(&0u64.to_le_bytes());
        header[layout::INTERACTION_RECORDS_OFFSET]
            .copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
        header[layout::INTERACTION_RECORDS_LEN].copy_from_slice(&0u64.to_le_bytes());
        header[layout::RESERVED].copy_from_slice(&[0u8; 4]);

        let crc = compute_header_crc32(&header);
        header[layout::HEADER_CRC32].copy_from_slice(&crc.to_le_bytes());
        header
    }

    #[test]
    fn parses_a_well_formed_header() {
        let bytes = valid_header_bytes(0);
        let header = Header::parse(&bytes).unwrap();
        assert_eq!(header.header_len, HEADER_LEN);
        assert_eq!(header.drug_count, 0);
    }

    #[test]
    fn rejects_wrong_magic_bytes() {
        let mut bytes = valid_header_bytes(0);
        bytes[0] = b'X';
        assert_eq!(Header::parse(&bytes).unwrap_err(), DbError::BadMagic);
    }

    #[test]
    fn rejects_an_unsupported_format_version() {
        let mut bytes = valid_header_bytes(0);
        bytes[layout::FORMAT_VERSION].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            Header::parse(&bytes).unwrap_err(),
            DbError::UnsupportedVersion(2)
        );
    }

    #[test]
    fn rejects_a_tampered_header() {
        let mut bytes = valid_header_bytes(0);
        bytes[layout::DRUG_COUNT].copy_from_slice(&999u32.to_le_bytes());
        assert!(matches!(
            Header::parse(&bytes).unwrap_err(),
            DbError::HeaderChecksumMismatch { .. }
        ));
    }

    #[test]
    fn rejects_a_file_shorter_than_the_header() {
        let bytes = vec![0u8; HEADER_LEN - 1];
        assert_eq!(Header::parse(&bytes).unwrap_err(), DbError::Truncated);
    }

    #[test]
    fn rejects_a_section_that_would_read_past_the_end_of_the_file() {
        let bytes = valid_header_bytes(1_000_000);
        assert_eq!(Header::parse(&bytes).unwrap_err(), DbError::Truncated);
    }
}

//! Byte ranges for every fixed field in the .men header, named to match
//! docs/DATABASE_FORMAT.md's field table exactly, so field access reads as
//! `header[BUILD_TIMESTAMP]` rather than a bare `header[12..20]`.

use std::ops::Range;

pub(crate) const MAGIC: Range<usize> = 0..4;
pub(crate) const FORMAT_VERSION: Range<usize> = 4..6;
pub(crate) const HEADER_LEN_FIELD: Range<usize> = 6..8;
pub(crate) const HEADER_CRC32: Range<usize> = 8..12;
pub(crate) const BUILD_TIMESTAMP: Range<usize> = 12..20;
pub(crate) const PAYLOAD_SHA256: Range<usize> = 20..52;
pub(crate) const DRUG_COUNT: Range<usize> = 52..56;
pub(crate) const INTERACTION_COUNT: Range<usize> = 56..60;
pub(crate) const STRING_TABLE_OFFSET: Range<usize> = 60..68;
pub(crate) const STRING_TABLE_LEN: Range<usize> = 68..76;
pub(crate) const DRUG_TABLE_OFFSET: Range<usize> = 76..84;
pub(crate) const DRUG_TABLE_LEN: Range<usize> = 84..92;
pub(crate) const INTERACTION_INDEX_OFFSET: Range<usize> = 92..100;
pub(crate) const INTERACTION_INDEX_LEN: Range<usize> = 100..108;
pub(crate) const INTERACTION_RECORDS_OFFSET: Range<usize> = 108..116;
pub(crate) const INTERACTION_RECORDS_LEN: Range<usize> = 116..124;
pub(crate) const RESERVED: Range<usize> = 124..128;

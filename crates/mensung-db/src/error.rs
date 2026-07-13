//! Errors returned when opening or reading a .men database. Every variant
//! corresponds to a specific way the file in docs/DATABASE_FORMAT.md can be
//! wrong; there is no catch-all variant, so a failure always says what
//! actually went wrong.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DbError {
    #[error("file is too short to contain a valid .men header or section")]
    Truncated,

    #[error("not a .men file: missing or incorrect magic bytes")]
    BadMagic,

    #[error("unsupported .men format version {0}; this reader supports version 1")]
    UnsupportedVersion(u16),

    #[error("header checksum mismatch: expected {expected:08x}, computed {found:08x}")]
    HeaderChecksumMismatch { expected: u32, found: u32 },

    #[error("payload checksum mismatch: the database file is corrupted")]
    PayloadChecksumMismatch,

    #[error("interaction record at offset {offset} declares length {declared}, but its actual length is {actual}")]
    InteractionRecordLengthMismatch {
        offset: u64,
        declared: u32,
        actual: u32,
    },

    #[error("drug table entry {index} is not valid UTF-8")]
    InvalidDrugName { index: u32 },

    #[error("interaction record {index} is not valid UTF-8")]
    InvalidInteractionText { index: u32 },

    #[error("interaction record {index} has an unrecognized severity byte {value}")]
    InvalidSeverity { index: u32, value: u8 },

    #[error("interaction record {index} has an unrecognized evidence byte {value}")]
    InvalidEvidence { index: u32, value: u8 },

    #[error("interaction index entry {index} pairs a drug with itself")]
    CorruptPair { index: u32 },
}

//! Errors returned when opening or reading a .men database. Every variant
//! corresponds to a specific way the file in docs/DATABASE_FORMAT.md can be
//! wrong; there is no catch-all variant, so a failure always says what
//! actually went wrong.

use mensung_domain::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DbError {
    #[error("file is too short to contain a valid .men header or section")]
    Truncated,

    #[error("not a .men file: missing or incorrect magic bytes")]
    BadMagic,

    #[error("unsupported .men format version {0}; this reader supports version 2")]
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

    #[error("drug fact record at offset {offset} declares length {declared}, but its actual length is {actual}")]
    DrugFactRecordLengthMismatch {
        offset: u64,
        declared: u32,
        actual: u32,
    },

    #[error("drug table entry {index} is not valid UTF-8")]
    InvalidDrugName { index: u32 },

    #[error("a string table reference points outside the string table or is not valid UTF-8")]
    InvalidStringTableEntry,

    #[error("a claim's fields do not satisfy mensung-domain's invariants: {0}")]
    InvalidClaim(DomainError),

    #[error("an ATC code table entry does not satisfy mensung-domain's invariants: {0}")]
    InvalidAtcCode(DomainError),

    #[error("unrecognized source tier byte {0}")]
    InvalidSourceTier(u8),

    #[error("unrecognized severity byte {0}")]
    InvalidSeverity(u8),

    #[error("unrecognized evidence byte {0}")]
    InvalidEvidence(u8),

    #[error("unrecognized confidence byte {0}")]
    InvalidConfidence(u8),

    #[error("unrecognized drug fact kind byte {0}")]
    InvalidDrugFactKind(u8),

    #[error("interaction index entry {index} pairs a drug with itself")]
    CorruptPair { index: u32 },
}

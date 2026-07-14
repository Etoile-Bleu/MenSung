//! Errors returned by domain-level construction and validation. Every one of
//! these represents a rejected input, not a bug; callers are expected to
//! handle them, never to unwrap past them.

use crate::{DrugFactId, DrugId, InteractionId};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("INN name cannot be empty")]
    EmptyInnName,

    #[error(
        "INN name '{name}' contains an invalid character '{invalid_char}'; only letters, spaces, and hyphens are allowed"
    )]
    InvalidInnNameCharacter { name: String, invalid_char: char },

    #[error("INN name '{name}' is {actual} characters long, exceeding the {max} character limit")]
    InnNameTooLong {
        name: String,
        max: usize,
        actual: usize,
    },

    #[error("drug {0:?} cannot interact with itself")]
    SelfInteraction(DrugId),

    #[error("interaction {0:?} has an empty description")]
    EmptyDescription(InteractionId),

    #[error("interaction {0:?} has an empty source citation")]
    EmptySource(InteractionId),

    #[error("source id '{0}' must be a non-empty, lowercase slug (letters, digits, hyphens)")]
    InvalidSourceId(String),

    #[error("source id '{0}' has no display name")]
    EmptySourceName(String),

    #[error("claim date {year:04}-{month:02}-{day:02} is not a valid calendar date")]
    InvalidClaimDate { year: u16, month: u8, day: u8 },

    #[error("a claim from source '{0}' has an empty rationale")]
    EmptyRationale(String),

    #[error("interaction {0:?} was constructed with zero claims; every fact needs at least one")]
    NoClaimsForInteraction(InteractionId),

    #[error("drug fact {0:?} was constructed with zero claims; every fact needs at least one")]
    NoClaimsForDrugFact(DrugFactId),

    #[error("'{0}' is not a valid RxCUI (must be a non-empty string of digits)")]
    InvalidRxcui(String),

    #[error("PubChem CID {0} has an empty molecular formula")]
    EmptyMolecularFormula(u32),

    #[error("PubChem CID {cid} has a molecular weight '{raw}' that does not parse as a number")]
    InvalidMolecularWeight { cid: u32, raw: String },

    #[error(
        "'{0}' is not a valid ATC code (must be one uppercase letter, two digits, two uppercase letters)"
    )]
    InvalidAtcCode(String),

    #[error("ATC code '{0}' has no class name")]
    EmptyAtcClassName(String),
}

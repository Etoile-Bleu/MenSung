//! Errors returned by domain-level construction and validation. Every one of
//! these represents a rejected input, not a bug; callers are expected to
//! handle them, never to unwrap past them.

use crate::{DrugId, InteractionId};

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
}

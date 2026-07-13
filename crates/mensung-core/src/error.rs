//! Errors returned by the lookup engine and fuzzy matcher.

use mensung_db::DbError;
use mensung_domain::DrugId;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Database(#[from] DbError),

    #[error("cannot check interactions among fewer than two drugs")]
    NotEnoughDrugs,

    #[error("the same drug ({0:?}) was listed twice")]
    DuplicateDrug(DrugId),
}

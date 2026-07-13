//! Errors that stop a database build outright.

use mensung_db::DbError;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("{count} validation issue(s) found, refusing to produce a .men file")]
    ValidationFailed { count: usize },

    #[error("the compiled database failed to re-open and self-verify: {0}")]
    SelfVerificationFailed(#[from] DbError),
}

//! Compiles a validated drug/interaction dataset into a .men file. This is
//! the one crate in the workspace allowed to write the binary format;
//! mensung-db only ever reads it. See docs/DATABASE_FORMAT.md. Every build
//! re-opens its own output through mensung-db before returning it, so a bug
//! in the writer surfaces as a build failure here, not as a corrupt file
//! shipped to the field. `ddinter` imports the real dataset (see
//! MEDICAL_DATA_POLICY.md for why DDInter and not the originally planned
//! OpenFDA/RxNorm/WHO); `seed` is a small hand-authored placeholder used
//! until a human runs the DDInter import for real.

mod ddinter;
mod error;
mod report;
mod seed;
mod validate;
mod writer;

pub use ddinter::{import_ddinter, ImportError};
pub use error::BuildError;
pub use report::ValidationReport;
pub use seed::seed_dataset;
pub use validate::{validate, ValidationIssue};

use mensung_domain::{Drug, Interaction};

pub fn build_database(
    drugs: Vec<Drug>,
    interactions: Vec<Interaction>,
) -> Result<(Vec<u8>, ValidationReport), BuildError> {
    let issues = validate::validate(&drugs, &interactions);
    let report = ValidationReport {
        errors: issues.len(),
        warnings: 0,
        interactions: interactions.len(),
    };

    if !issues.is_empty() {
        return Err(BuildError::ValidationFailed {
            count: issues.len(),
        });
    }

    let bytes = writer::compile(drugs, &interactions);
    self_verify_by_reopening(&bytes)?;

    Ok((bytes, report))
}

fn self_verify_by_reopening(bytes: &[u8]) -> Result<(), BuildError> {
    mensung_db::Database::open(bytes)?;
    Ok(())
}

//! Compiles a validated drug/interaction dataset into a .men file. This is
//! the one crate in the workspace allowed to write the binary format;
//! mensung-db only ever reads it. See docs/DATABASE_FORMAT.md. Every build
//! re-opens its own output through mensung-db before returning it, so a bug
//! in the writer surfaces as a build failure here, not as a corrupt file
//! shipped to the field. A real DDInter importer is ROADMAP.md Phase 5 work
//! still open (see MEDICAL_DATA_POLICY.md for why DDInter and not the
//! originally planned OpenFDA/RxNorm/WHO); today this crate ships the
//! compiler, the validation pipeline, and a small bootstrap seed dataset.

mod error;
mod report;
mod seed;
mod validate;
mod writer;

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

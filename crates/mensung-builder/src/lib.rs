//! Compiles a validated drug/interaction dataset into a .men file. This is
//! the one crate in the workspace allowed to write the binary format;
//! mensung-db only ever reads it. See docs/DATABASE_FORMAT.md. Every build
//! re-opens its own output through mensung-db before returning it, so a bug
//! in the writer surfaces as a build failure here, not as a corrupt file
//! shipped to the field. `ddinter` imports the real dataset (see
//! MEDICAL_DATA_POLICY.md for why DDInter and not the originally planned
//! OpenFDA/RxNorm/WHO); `download` fetches it over HTTPS, the only
//! network-touching code in the workspace, used by `mensung-client` to
//! install the dataset at runtime.

mod atc;
mod atc_download;
mod claims;
mod ddinter;
mod download;
mod error;
mod openfda;
mod openfda_download;
mod pubchem;
mod pubchem_download;
mod report;
mod rxnorm;
mod rxnorm_download;
mod validate;
mod writer;

pub use atc::{attach_atc_codes, AtcImportError};
pub use atc_download::{fetch_all as fetch_all_atc_codes, AtcFetchError};
pub use claims::{ddinter_source, wrap_as_claims, DDINTER_SOURCE_ID};
pub use ddinter::{import_ddinter, ImportError};
pub use download::{download_and_import_ddinter, is_cached, DownloadError};
pub use error::BuildError;
pub use openfda::{import_openfda_labels, openfda_source, OpenFdaImportError, OPENFDA_SOURCE_ID};
pub use openfda_download::{fetch_all as fetch_all_openfda_labels, OpenFdaFetchError};
pub use pubchem::{attach_chemical_properties, PubchemImportError};
pub use pubchem_download::{fetch_all as fetch_all_pubchem_properties, PubchemFetchError};
pub use report::ValidationReport;
pub use rxnorm::{attach_rxcuis, RxNormImportError};
pub use rxnorm_download::{fetch_all as fetch_all_rxcuis, RxNormFetchError};
pub use validate::{validate, ValidationIssue};

use mensung_domain::{Drug, DrugFact, InteractionFact};

pub fn build_database(
    drugs: Vec<Drug>,
    interactions: Vec<InteractionFact>,
    drug_facts: Vec<DrugFact>,
) -> Result<(Vec<u8>, ValidationReport), BuildError> {
    let issues = validate::validate(&drugs, &interactions, &drug_facts);
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

    let bytes = writer::compile(drugs, &interactions, &drug_facts);
    self_verify_by_reopening(&bytes)?;

    Ok((bytes, report))
}

fn self_verify_by_reopening(bytes: &[u8]) -> Result<(), BuildError> {
    mensung_db::Database::open(bytes)?;
    Ok(())
}

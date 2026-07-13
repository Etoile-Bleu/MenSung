//! Domain layer: drug entities, interaction records, severity, evidence,
//! and the validation rules that construct them. No I/O, no UI, and no
//! dependency on any other crate in the workspace, per GOOD_PRACTICE.md.

mod drug;
mod error;
mod evidence;
mod ids;
mod inn_name;
mod interaction;
mod severity;

pub use drug::Drug;
pub use error::DomainError;
pub use evidence::EvidenceLevel;
pub use ids::{DrugId, InteractionId};
pub use inn_name::InnName;
pub use interaction::{DrugPair, Interaction};
pub use severity::Severity;

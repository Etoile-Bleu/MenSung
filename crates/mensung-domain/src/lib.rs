//! Domain layer: drug entities, interaction records, severity, evidence,
//! and the validation rules that construct them. No I/O, no UI, and no
//! dependency on any other crate in the workspace, per GOOD_PRACTICE.md.
//!
//! `Interaction` is the single-claim, single-severity fact the current
//! `.men` format compiles and the CLI/TUI display. `InteractionFact` and
//! `DrugFact` are the richer, multi-source model behind it: every claim
//! from every contributing source (`Source`, ranked by `SourceTier`), none
//! ever discarded when sources disagree. See MEDICAL_DATA_POLICY.md's
//! Trust and Conflict Resolution section.

mod claim;
mod claim_date;
mod confidence;
mod drug;
mod drug_fact;
mod error;
mod evidence;
mod ids;
mod inn_name;
mod interaction;
mod interaction_fact;
mod rxcui;
mod severity;
mod source;

pub use claim::Claim;
pub use claim_date::ClaimDate;
pub use confidence::Confidence;
pub use drug::Drug;
pub use drug_fact::{DrugFact, DrugFactKind};
pub use error::DomainError;
pub use evidence::EvidenceLevel;
pub use ids::{DrugFactId, DrugId, InteractionId};
pub use inn_name::InnName;
pub use interaction::{DrugPair, Interaction};
pub use interaction_fact::InteractionFact;
pub use rxcui::Rxcui;
pub use severity::Severity;
pub use source::{Source, SourceId, SourceTier};

//! Application layer: the lookup engine and fuzzy matcher built on top of
//! mensung-domain and mensung-db. No filesystem, no network, no UI; every
//! function here takes a `Database` reference and returns borrowed data.

mod candidate;
mod error;
mod fuzzy;
mod interaction;
mod lookup;

pub use candidate::Candidate;
pub use error::CoreError;
pub use interaction::check_interactions;
pub use lookup::{lookup_drug, LookupOutcome};

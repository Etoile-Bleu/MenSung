//! Binary .men database reader: zero-copy access on the binary-search
//! lookup path and checksum validation, implementing the format specified
//! in docs/DATABASE_FORMAT.md. This crate never touches the filesystem or
//! the network; callers hand it a byte buffer, however it was loaded.

mod atc_table;
mod bytes;
mod claim_record;
mod database;
mod drug_fact;
mod drug_fact_index;
mod drug_table;
mod error;
mod header;
mod interaction_index;
mod interaction_record;
mod layout;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use atc_table::AtcCodeIter;
pub use atc_table::AtcCodeRecord;
pub use claim_record::ClaimRecord;
pub use database::{Database, DrugIter};
pub use drug_fact::DrugFactRecord;
pub use drug_table::DrugRecord;
pub use error::DbError;
pub use interaction_record::InteractionRecord;

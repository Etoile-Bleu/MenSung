//! Binary .men database reader: zero-copy access and checksum validation,
//! implementing the format specified in docs/DATABASE_FORMAT.md. This crate
//! never allocates on the lookup path and never touches the filesystem or
//! the network; callers hand it a byte buffer, however it was loaded.

mod bytes;
mod database;
mod drug_table;
mod error;
mod header;
mod interaction_index;
mod interaction_record;

pub use database::{Database, DrugIter};
pub use drug_table::DrugRecord;
pub use error::DbError;
pub use interaction_record::InteractionRecord;

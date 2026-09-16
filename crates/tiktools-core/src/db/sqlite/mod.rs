//! SQLite schema and conservative persistence helpers.
//!
//! The SQL intentionally mirrors the current `src/db/points-db.ts` and
//! `src/db/automation-db.ts` tables. Domain payloads that have not yet gained
//! a Rust type stay as JSON so an existing database can be opened without a
//! destructive migration.

mod behavior;
mod meta;
mod points;
mod schema;
#[cfg(test)]
mod tests;
mod util;
mod workflows;

use thiserror::Error;

pub(super) use super::DatabaseManager;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored record is invalid: {0}")]
    Invalid(String),
}

//! Durable plugin source revisions and account-partitioned local sync intents.
//!
//! These methods provide only the storage transaction. Their caller must hold
//! the protocol and identity/opt-in lease, drain before any network sync, and
//! refuse stale source generations. Stored identity is not authorization.

mod admission;
mod history;
mod outbox;
mod receipts;
mod source;
mod untracked;

use history::{apply_history_allows_source, package_source_available};
// Keep sibling SQLite journals available to the extracted history guard.
use super::{plugin_apply_quarantine, plugin_sync_apply};

use rusqlite::{params, OptionalExtension, TransactionBehavior};

use super::{
    behavior::advance_plugin_source,
    util::{bool_int, now},
    DatabaseError, DatabaseManager,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalPluginSyncIntent<'a> {
    pub origin: &'a str,
    pub account_id: &'a str,
    pub config_generation: u128,
    /// None is a tombstone; a selection contains the installed version.
    pub version: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginSyncOutboxEntry {
    pub origin: String,
    pub account_id: String,
    pub plugin_id: String,
    pub generation: i64,
    pub source_generation: i64,
    pub config_generation: u128,
    /// Confirmation is valid only for this particular native host lifetime.
    pub runtime_epoch: Option<String>,
    pub reviewed_base: Option<String>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

/// Exact local-only source observation. It carries no Cloud authorization;
/// callers must bind it to explicit consent, package identity and runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntrackedPluginReview {
    pub plugin_id: String,
    pub generation: i64,
    pub installed: bool,
    pub enabled: bool,
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_runtime_epoch(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests;

//! Durable source-fenced local application decisions for v1 sync.
//!
//! Storage is not authorization. The caller must hold the shared protocol
//! admission and local plugin mutation gates, revalidate the exact live
//! session and opt-in epoch, and check the coordinator's durable state before
//! advancing any phase. No runtime operation or network request occurs here.
mod admission;
mod commit;
mod finalization;
mod readiness;
mod recovery;
mod validation;

use super::plugin_apply_quarantine;
#[cfg(test)]
use super::LocalPluginSyncIntent;
use validation::{payload, valid_intent};

use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::{
    PluginSelection, SyncState, V1Edit, V1LocalEditContext, V1Pending, V1Record,
};

use super::{
    util::{bool_int, now},
    DatabaseError, DatabaseManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PluginApplyReason {
    Pulled,
    ReviewedConflict,
    ReviewedUncertain,
}

/// Native-only snapshot. A UI must never construct or edit this evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PluginApplyIntent {
    pub intent_id: String,
    pub origin: String,
    pub account_id: String,
    #[serde(with = "tiktools_cloud_sync::consent_generation")]
    pub config_generation: u128,
    pub plugin_id: String,
    pub source_generation: i64,
    pub installed_version: String,
    pub expected_enabled: bool,
    pub reviewed_pending: Option<V1Pending>,
    /// The frozen earlier base when known. None does not prove it was absent.
    pub base_record: Option<V1Record>,
    /// None means absent in Cloud and therefore disables, never uninstalls.
    pub remote: Option<V1Record>,
    pub reason: PluginApplyReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginApplyPhase {
    Prepared,
    Ready,
    RuntimePending,
    Applied,
    Finalized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredPluginApplyIntent {
    pub intent: PluginApplyIntent,
    pub phase: PluginApplyPhase,
    pub committed_generation: Option<i64>,
    pub runtime_epoch: Option<String>,
}

/// Native-only discovery result. A page bounds the number of *all* active
/// journal rows scanned, even if none belong to this identity. The cursor is
/// opaque and must never be exposed to the webview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginApplyRecoveryPage {
    pub intents: Vec<StoredPluginApplyIntent>,
    pub stale_consent: bool,
    pub next_cursor: Option<String>,
}

fn valid_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn matches_context(intent: &PluginApplyIntent, context: &V1LocalEditContext) -> bool {
    intent.origin == context.origin
        && intent.account_id == context.account_id
        && intent.config_generation == context.config_generation
}

pub(super) type StoredRow = (String, String, String, Option<i64>, Option<String>);

pub(super) fn decode_stored(
    id: &str,
    row: StoredRow,
) -> Result<StoredPluginApplyIntent, DatabaseError> {
    let (plugin_id, phase, encoded, generation, epoch) = row;
    if !valid_token(id) || encoded.len() > 16 * 1024 {
        return Err(DatabaseError::Invalid("invalid plugin apply intent".into()));
    }
    let intent: PluginApplyIntent = serde_json::from_str(&encoded)?;
    if intent.intent_id != id || intent.plugin_id != plugin_id || !valid_intent(&intent) {
        return Err(DatabaseError::Invalid("invalid plugin apply intent".into()));
    }
    let phase = match (phase.as_str(), generation, epoch.as_deref()) {
        ("prepared", None, None) => PluginApplyPhase::Prepared,
        ("ready", None, None) => PluginApplyPhase::Ready,
        ("runtime_pending", Some(g), None)
            if g == intent.source_generation.checked_add(1).unwrap_or(0) =>
        {
            PluginApplyPhase::RuntimePending
        }
        ("applied", Some(g), Some(e))
            if g == intent.source_generation.checked_add(1).unwrap_or(0) && valid_token(e) =>
        {
            PluginApplyPhase::Applied
        }
        ("finalized", Some(g), Some(e))
            if g == intent.source_generation.checked_add(1).unwrap_or(0) && valid_token(e) =>
        {
            PluginApplyPhase::Finalized
        }
        _ => return Err(DatabaseError::Invalid("invalid plugin apply phase".into())),
    };
    Ok(StoredPluginApplyIntent {
        intent,
        phase,
        committed_generation: generation,
        runtime_epoch: epoch,
    })
}

#[cfg(test)]
mod tests;

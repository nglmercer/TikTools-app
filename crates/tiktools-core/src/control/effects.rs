//! Owner-readable, metadata-only receipts for nontransactional automation effects.
use super::OperationError;
use crate::AppCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EffectKind {
    Http,
    Audio,
    Plugin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EffectState {
    Accepted,
    Dispatched,
    Succeeded,
    Failed,
    Unknown,
}

/// Deliberately excludes definitions, requests, destinations, settings, errors
/// and response payloads. An expired receipt retains its deduplication identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectReceipt {
    pub effect_id: String,
    pub kind: EffectKind,
    pub state: EffectState,
    pub accepted_at: i64,
    pub updated_at: i64,
    pub outcome_expired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectHistory {
    pub items: Vec<EffectReceipt>,
    pub next_before: Option<String>,
}

#[cfg(feature = "persistence")]
fn history_error(error: crate::db::DatabaseError) -> OperationError {
    match error {
        crate::db::DatabaseError::Invalid(message) => OperationError::invalid(message),
        _ => OperationError::internal("external effect history is unavailable"),
    }
}

impl AppCore {
    /// Trusted local Owner operation. Transport adapters must enforce Owner
    /// access before exposing this history; it conveys no execution authority.
    pub fn automation_effect_receipt(
        &self,
        effect_id: &str,
    ) -> Result<Option<EffectReceipt>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            self.effects.as_ref().map_err(|_| {
                OperationError::unavailable("external effect journal recovery is unavailable")
            })?;
            self.db
                .load_effect_receipt(effect_id)
                .map_err(history_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = effect_id;
            Err(OperationError::unavailable(
                "effect history requires persistence",
            ))
        }
    }

    /// A bounded page ordered by opaque identity. The cursor never includes
    /// request data and looking up a receipt cannot retry its effect.
    pub fn automation_effect_history(
        &self,
        before: Option<&str>,
        limit: u32,
    ) -> Result<EffectHistory, OperationError> {
        #[cfg(feature = "persistence")]
        {
            self.effects.as_ref().map_err(|_| {
                OperationError::unavailable("external effect journal recovery is unavailable")
            })?;
            self.db
                .load_effect_history(before, limit)
                .map_err(history_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (before, limit);
            Err(OperationError::unavailable(
                "effect history requires persistence",
            ))
        }
    }
}

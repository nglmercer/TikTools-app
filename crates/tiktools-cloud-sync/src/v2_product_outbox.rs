//! Private v2 source-outbox staging. This never sends or applies remote data.
use crate::{
    PluginSelection, ProtocolFenceError, SyncCoordError, SyncCoordinator, WorkspaceCoordinator,
};
use cloud_client::{WorkspaceEdit, WorkspaceKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// A durable product decision. Native source adapters must not construct it
/// from UI claims; its revision belongs to the locally committed source.
#[derive(Clone, Serialize)]
pub struct V2ProductDecision {
    pub origin: String,
    pub account_id: String,
    pub config_generation: u128,
    pub source_generation: u64,
    pub plugin_id: String,
    pub selection: Option<PluginSelection>,
}

/// Trusted native adapter: hold the source mutation fence while loading the
/// durable confirmed candidate, running `stage`, and acknowledging the same
/// source revision. Never acknowledge an error or perform network work here.
/// An ack failure after staging is safe to retry: the workspace receipt is
/// persisted atomically with the decision and survives delivery/rebuild.
pub trait V2ProductSource {
    fn with_fenced_decision<T>(
        &self,
        stage: impl FnOnce(&V2ProductDecision) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError>;
}

impl SyncCoordinator {
    /// Stage one durable source decision, with the source fence outermost and
    /// protocol, opt-in and account checks held until the workspace save.
    /// This is native plumbing only: it neither dispatches nor authorizes
    /// remotely derived edits, and product wiring must prevent sync loops.
    pub fn stage_v2_product_decision<S: V2ProductSource>(
        &self,
        source: &S,
    ) -> Result<(u64, bool), SyncCoordError> {
        source.with_fenced_decision(|decision| {
            let identity = self
                .auth
                .try_sync_identity()
                .map_err(|_| ProtocolFenceError::Busy)?
                .ok_or(SyncCoordError::NotSignedIn)?;
            if identity.0 != decision.origin || identity.1 != decision.account_id {
                return Err(SyncCoordError::StaleReview);
            }
            let _lease = self.protocol.try_v2(&identity.0, &identity.1)?;
            self.auth
                .with_sync_identity_permit(|current| {
                    if current.as_ref() != Some(&identity) {
                        return Err(SyncCoordError::StaleReview);
                    }
                    let config = self.read_staging_config()?;
                    if !config.enabled
                        || config.version != 2
                        || config.scope.is_none()
                        || !config.allows(&identity.0, &identity.1)
                        || config.generation != decision.config_generation
                    {
                        return Err(SyncCoordError::StaleReview);
                    }
                    if decision
                        .selection
                        .as_ref()
                        .is_some_and(|v| v.plugin_id != decision.plugin_id)
                    {
                        return Err(SyncCoordError::StaleReview);
                    }
                    let target = WorkspaceKey::new("plugin-selections", &decision.plugin_id)
                        .map_err(|_| SyncCoordError::StaleReview)?;
                    let edit = match &decision.selection {
                        Some(selection) => WorkspaceEdit::Put(
                            serde_json::to_value(selection)
                                .map_err(|_| SyncCoordError::StaleReview)?,
                        ),
                        None => WorkspaceEdit::Delete,
                    };
                    let encoded =
                        serde_json::to_vec(decision).map_err(|_| SyncCoordError::StaleReview)?;
                    let digest: String = Sha256::digest(encoded)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect();
                    let root = self
                        .config_path
                        .parent()
                        .ok_or_else(|| SyncCoordError::Io("invalid config directory".into()))?;
                    let workspace =
                        WorkspaceCoordinator::open(root, identity.0.clone(), identity.1.clone())
                            .map_err(|_| SyncCoordError::StaleReview)?;
                    workspace
                        .stage_product_decision(target, edit, decision.source_generation, &digest)
                        .map_err(|_| SyncCoordError::StaleReview)
                })
                .map_err(|_| SyncCoordError::Protocol(ProtocolFenceError::Busy))?
        })
    }
}

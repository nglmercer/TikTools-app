//! Explicit reviewed construction of the first v2 workspace.
use super::{WorkspaceCoordError, WorkspaceCoordinator};
use cloud_client::{
    PluginSelection, SyncState, WorkspaceEdit, WorkspaceKey, WorkspaceState, WorkspaceStateError,
};

impl WorkspaceCoordinator {
    /// Caller must have reviewed these exact local selections and legacy
    /// pending intents under the current account. This operation uses no
    /// network, imports no v1 bases/cursor, and never rewrites the v1 file.
    pub fn bootstrap(
        &self,
        selections: &[PluginSelection],
        legacy: Option<&SyncState>,
    ) -> Result<(), WorkspaceCoordError> {
        let _lease = self.generic_lease()?;
        self.bootstrap_inner(selections, legacy, None)
    }

    /// Construct exactly the reviewed candidate in one atomic state save.
    pub(crate) fn bootstrap_reviewed(
        &self,
        preview: &crate::CutoverPreview,
    ) -> Result<(), WorkspaceCoordError> {
        self.bootstrap_inner(&preview.selections(), preview.legacy(), Some(preview))
    }

    fn bootstrap_inner(
        &self,
        selections: &[PluginSelection],
        legacy: Option<&SyncState>,
        preview: Option<&crate::CutoverPreview>,
    ) -> Result<(), WorkspaceCoordError> {
        let _operation = self
            .store
            .try_operation_lock()?
            .ok_or(WorkspaceCoordError::Busy)?;
        let mut ids = std::collections::BTreeSet::new();
        for selection in selections {
            if !ids.insert(&selection.plugin_id) {
                return Err(WorkspaceStateError::Invalid.into());
            }
        }
        self.update_inner(
            preview.is_some_and(|preview| preview.is_marked()),
            |state| {
                if state.bootstrap_complete {
                    return Err(WorkspaceCoordError::AlreadyBootstrapped);
                }
                if *state != WorkspaceState::new(self.origin.clone(), self.account_id.clone())? {
                    return Err(WorkspaceCoordError::BootstrapNeedsReview);
                }
                for selection in selections {
                    let key = WorkspaceKey::new("plugin-selections", selection.plugin_id.clone())?;
                    let value = serde_json::to_value(selection)
                        .map_err(|_| WorkspaceStateError::Invalid)?;
                    state.stage(key, WorkspaceEdit::Put(value))?;
                }
                if let Some(legacy) = legacy {
                    state.import_v1_pending(legacy)?;
                }
                if let Some(preview) = preview {
                    preview.apply_review_policy(state)?;
                    if preview.is_marked() {
                        state.mark_product_guarded();
                    }
                }
                state.mark_bootstrap_complete();
                if preview.is_some_and(|preview| !preview.matches_bootstrap(state)) {
                    return Err(WorkspaceStateError::StaleReview.into());
                }
                Ok(())
            },
        )
    }
}

//! Read-only preparation for explicit v1-to-v2 cutover. This does not acquire
//! the product-wide fence or disable v1; callers must do both before accepting.
use std::collections::BTreeMap;

use cloud_client::{
    PluginSelection, StateSnapshot, WorkspaceEdit, WorkspaceEntry, WorkspaceKey, WorkspaceState,
    WorkspaceStateError,
};

/// Captures exactly what will be staged and the v1 snapshot used to review it.
/// The preview is native-only and must not be reconstructed from UI parameters.
pub struct CutoverPreview {
    origin: String,
    account_id: String,
    selections: BTreeMap<String, PluginSelection>,
    legacy: Option<StateSnapshot>,
    candidate: WorkspaceState,
}

impl CutoverPreview {
    pub fn new(
        origin: String,
        account_id: String,
        selections: &[PluginSelection],
        legacy: Option<&StateSnapshot>,
    ) -> Result<Self, WorkspaceStateError> {
        let mut candidate = WorkspaceState::new(origin.clone(), account_id.clone())?;
        let mut selected = BTreeMap::new();
        for selection in selections {
            if selected
                .insert(selection.plugin_id.clone(), selection.clone())
                .is_some()
            {
                return Err(WorkspaceStateError::Invalid);
            }
        }
        // Bootstrap receives the same canonical order, so its local
        // generations match the reviewed candidate.
        for selection in selected.values() {
            let key = WorkspaceKey::new("plugin-selections", &selection.plugin_id)?;
            let value =
                serde_json::to_value(selection).map_err(|_| WorkspaceStateError::Invalid)?;
            candidate.stage(key, WorkspaceEdit::Put(value))?;
        }
        if let Some(legacy) = legacy {
            candidate.import_v1_pending(&legacy.state)?;
        }
        Ok(Self {
            origin,
            account_id,
            selections: selected,
            legacy: legacy.cloned(),
            candidate,
        })
    }

    /// Frozen resulting pending entries, including imported v1 conflicts and
    /// newer, differing installed selections. V1 revisions are not reused.
    pub fn entries(&self) -> &[WorkspaceEntry] {
        &self.candidate.entries
    }

    /// Verify the source of consent while the caller holds the product-wide
    /// cutover fence. This comparison alone cannot stop an in-flight v1 write.
    pub fn verify_inputs(
        &self,
        origin: &str,
        account_id: &str,
        selections: &[PluginSelection],
        legacy: Option<&StateSnapshot>,
    ) -> Result<(), WorkspaceStateError> {
        if self.origin != origin || self.account_id != account_id {
            return Err(WorkspaceStateError::WrongIdentity);
        }
        let mut current = BTreeMap::new();
        for selection in selections {
            if current
                .insert(selection.plugin_id.clone(), selection.clone())
                .is_some()
            {
                return Err(WorkspaceStateError::Invalid);
            }
        }
        let same_legacy = match (&self.legacy, legacy) {
            (None, None) => true,
            (Some(old), Some(now)) => old.generation == now.generation && old.state == now.state,
            _ => false,
        };
        if current != self.selections || !same_legacy {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(())
    }

    /// Exact frozen inputs to pass to `WorkspaceCoordinator::bootstrap` after
    /// `verify_inputs` succeeds under the same still-held product fence.
    pub fn selections(&self) -> Vec<PluginSelection> {
        self.selections.values().cloned().collect()
    }

    pub fn legacy(&self) -> Option<&cloud_client::SyncState> {
        self.legacy.as_ref().map(|snapshot| &snapshot.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_client::{Edit, SyncState, WorkspacePendingStatus};

    fn selection(enabled: bool) -> PluginSelection {
        PluginSelection {
            schema_version: 1,
            plugin_id: "acme.plugin".into(),
            version: "1.0.0".into(),
            enabled,
        }
    }

    #[test]
    fn preview_keeps_v1_conflict_and_newer_local_choice_without_old_revision() {
        let mut state = SyncState::new("https://cloud.example".into(), "account".into());
        state
            .stage("acme.plugin".into(), Edit::Put(selection(false)))
            .unwrap();
        let pending = state
            .entries
            .get_mut("acme.plugin")
            .unwrap()
            .pending
            .as_mut()
            .unwrap();
        pending.conflict = true;
        pending.base_revision = Some(9);
        let legacy = StateSnapshot {
            generation: 3,
            state,
        };
        let preview = CutoverPreview::new(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(true)],
            Some(&legacy),
        )
        .unwrap();
        let entry = &preview.entries()[0];
        let pending = entry.pending.as_ref().unwrap();
        assert_eq!(pending.status, WorkspacePendingStatus::Conflict);
        assert_eq!(pending.base_revision, None);
        assert!(entry.next.is_some());
        assert!(preview
            .verify_inputs(
                "https://cloud.example",
                "account",
                &[selection(true)],
                Some(&legacy)
            )
            .is_ok());
        let mut newer = legacy.clone();
        newer.generation += 1;
        assert!(matches!(
            preview.verify_inputs(
                "https://cloud.example",
                "account",
                &[selection(true)],
                Some(&newer)
            ),
            Err(WorkspaceStateError::StaleReview)
        ));
        assert!(matches!(
            preview.verify_inputs(
                "https://cloud.example",
                "another",
                &[selection(true)],
                Some(&legacy)
            ),
            Err(WorkspaceStateError::WrongIdentity)
        ));
        assert!(matches!(
            preview.verify_inputs(
                "https://cloud.example",
                "account",
                &[selection(false)],
                Some(&legacy)
            ),
            Err(WorkspaceStateError::StaleReview)
        ));
    }

    #[test]
    fn preview_rejects_wrong_account_and_duplicate_selections() {
        let legacy = StateSnapshot {
            generation: 1,
            state: SyncState::new("https://cloud.example".into(), "other".into()),
        };
        assert!(matches!(
            CutoverPreview::new(
                "https://cloud.example".into(),
                "account".into(),
                &[],
                Some(&legacy)
            ),
            Err(WorkspaceStateError::WrongIdentity)
        ));
        assert!(matches!(
            CutoverPreview::new(
                "https://cloud.example".into(),
                "account".into(),
                &[selection(true), selection(false)],
                None
            ),
            Err(WorkspaceStateError::Invalid)
        ));
    }

    #[test]
    fn reverse_order_preview_matches_real_bootstrap_generations() {
        let first = selection(true);
        let mut second = first.clone();
        second.plugin_id = "aaa.plugin".into();
        let preview = CutoverPreview::new(
            "https://cloud.example".into(),
            "account".into(),
            &[first, second],
            None,
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let coordinator = crate::WorkspaceCoordinator::open(
            temp.path().join("private"),
            "https://cloud.example".into(),
            "account".into(),
        )
        .unwrap();
        coordinator
            .bootstrap(&preview.selections(), preview.legacy())
            .unwrap();
        assert_eq!(
            preview.entries(),
            coordinator.snapshot().unwrap().state.entries
        );
    }
}

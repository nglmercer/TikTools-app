//! Read-only preparation for explicit v1-to-v2 cutover. This does not acquire
//! the product-wide fence or disable v1; callers must do both before accepting.
use std::collections::BTreeMap;

use cloud_client::{
    PluginSelection, StateSnapshot, WorkspaceEdit, WorkspaceEntry, WorkspaceKey,
    WorkspacePendingStatus, WorkspaceState, WorkspaceStateError,
};

use crate::V1CutoverReceipt;

/// Captures exactly what will be staged and the v1 snapshot used to review it.
/// The preview is native-only and must not be reconstructed from UI parameters.
pub struct CutoverPreview {
    origin: String,
    account_id: String,
    selections: BTreeMap<String, PluginSelection>,
    legacy: Option<StateSnapshot>,
    candidate: WorkspaceState,
    marked_receipts: Option<Vec<V1CutoverReceipt>>,
    legacy_promotion: bool,
}

impl CutoverPreview {
    pub fn new(
        origin: String,
        account_id: String,
        selections: &[PluginSelection],
        legacy: Option<&StateSnapshot>,
    ) -> Result<Self, WorkspaceStateError> {
        // The portable workspace holds at most 1000 records. Reject an
        // oversized installed set before repeatedly validating candidates.
        if selections.len() > 1000 {
            return Err(WorkspaceStateError::TooLarge);
        }
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
            marked_receipts: None,
            legacy_promotion: false,
        })
    }

    /// Preserve legacy edits without inventing receipts. Every imported or
    /// local selection is a conflict until its source and Cloud base have
    /// been reconciled after migration.
    pub(crate) fn new_legacy_promotion(
        origin: String,
        account_id: String,
        selections: &[PluginSelection],
        legacy: Option<&StateSnapshot>,
    ) -> Result<Self, WorkspaceStateError> {
        let mut preview = Self::new(origin, account_id, selections, legacy)?;
        preview.marked_receipts = Some(Vec::new());
        preview.legacy_promotion = true;
        Self::force_receipt_review(&mut preview.candidate, &[])?;
        Ok(preview)
    }

    /// A marked candidate can never replay a receipt automatically. It must
    /// match all pending v1 intents, with 409 conflict promotion preserved;
    /// receipts for settled entries require an identical retained remote.
    pub(crate) fn new_marked(
        origin: String,
        account_id: String,
        selections: &[PluginSelection],
        legacy: Option<&StateSnapshot>,
        receipts: &[V1CutoverReceipt],
    ) -> Result<Self, WorkspaceStateError> {
        if receipts.len() > 1000 {
            return Err(WorkspaceStateError::TooLarge);
        }
        let mut ordered = BTreeMap::new();
        for receipt in receipts {
            if ordered
                .insert(receipt.key().to_owned(), receipt.clone())
                .is_some()
            {
                return Err(WorkspaceStateError::Invalid);
            }
        }
        if let Some(legacy) = legacy {
            for (key, entry) in &legacy.state.entries {
                if let Some(pending) = &entry.pending {
                    if !ordered
                        .get(key)
                        .is_some_and(|receipt| receipt.covers_pending(pending))
                    {
                        return Err(WorkspaceStateError::StaleReview);
                    }
                }
            }
        }
        for (key, receipt) in &ordered {
            let entry = legacy.and_then(|snapshot| snapshot.state.entries.get(key));
            let covered = entry.is_some_and(|entry| match &entry.pending {
                Some(pending) => receipt.covers_pending(pending),
                None => entry
                    .remote
                    .as_ref()
                    .is_some_and(|remote| receipt.covers_remote(remote)),
            });
            if !covered {
                return Err(WorkspaceStateError::StaleReview);
            }
        }
        let mut preview = Self::new(origin, account_id, selections, legacy)?;
        preview.marked_receipts = Some(ordered.into_values().collect());
        let receipts = preview.marked_receipts.as_ref().unwrap();
        Self::force_receipt_review(&mut preview.candidate, receipts)?;
        Ok(preview)
    }

    fn force_receipt_review(
        state: &mut WorkspaceState,
        receipts: &[V1CutoverReceipt],
    ) -> Result<(), WorkspaceStateError> {
        for receipt in receipts {
            let key = WorkspaceKey::new("plugin-selections", receipt.key())?;
            if state.entry(&key).is_none() {
                let edit = match &receipt.pending().edit {
                    cloud_client::Edit::Put(value) => WorkspaceEdit::Put(
                        serde_json::to_value(value).map_err(|_| WorkspaceStateError::Invalid)?,
                    ),
                    cloud_client::Edit::Delete => WorkspaceEdit::Delete,
                };
                state.stage(key, edit)?;
            }
            if let Some(entry) = state.entries.iter_mut().find(|entry| {
                entry.target.collection == "plugin-selections" && entry.target.key == receipt.key()
            }) {
                let pending = entry
                    .pending
                    .as_mut()
                    .ok_or(WorkspaceStateError::StaleReview)?;
                pending.status = WorkspacePendingStatus::Conflict;
            }
        }
        // Runtime continuity across hosts is not yet fenced. Every marked
        // selection must therefore remain non-dispatchable pending explicit
        // v2 review, even if it did not have a v1 receipt.
        for entry in &mut state.entries {
            if let Some(pending) = &mut entry.pending {
                pending.status = WorkspacePendingStatus::Conflict;
            }
        }
        state.validate()
    }

    pub(crate) fn apply_review_policy(
        &self,
        state: &mut WorkspaceState,
    ) -> Result<(), WorkspaceStateError> {
        if let Some(receipts) = &self.marked_receipts {
            Self::force_receipt_review(state, receipts)?;
        }
        Ok(())
    }

    pub(crate) fn is_marked(&self) -> bool {
        self.marked_receipts.is_some()
    }
    pub(crate) fn is_legacy_promotion(&self) -> bool {
        self.legacy_promotion
    }
    pub(crate) fn receipts(&self) -> Option<&[V1CutoverReceipt]> {
        self.marked_receipts.as_deref()
    }

    /// Frozen resulting pending entries, including imported v1 conflicts and
    /// newer, differing installed selections. V1 revisions are not reused.
    pub fn entries(&self) -> &[WorkspaceEntry] {
        &self.candidate.entries
    }

    /// Check the full state, including its next local generation, against the
    /// exact candidate produced by review.
    pub(crate) fn matches_bootstrap(&self, actual: &WorkspaceState) -> bool {
        let mut expected = self.candidate.clone();
        // Existing marked cutovers may have been bootstrapped before the
        // workspace-level guard existed. They remain resumable, but product
        // operations require a separate guarded-format upgrade before use.
        if self.is_marked() && actual.is_product_guarded() {
            expected.mark_product_guarded();
        }
        expected.mark_bootstrap_complete();
        expected == *actual
    }

    /// Canonical, versioned review material. A prepared migration must bind
    /// the entire source snapshot, including records that do not become v2
    /// entries, not merely the resulting staged edits.
    pub(crate) fn review_material(&self) -> Result<Vec<u8>, serde_json::Error> {
        let legacy = (
            "tiktools-v1-v2-cutover-review-v2",
            &self.origin,
            &self.account_id,
            &self.selections,
            self.legacy
                .as_ref()
                .map(|snapshot| (snapshot.generation, &snapshot.state)),
            &self.candidate.entries,
        );
        match &self.marked_receipts {
            None => serde_json::to_vec(&legacy),
            Some(receipts) if self.legacy_promotion => {
                serde_json::to_vec(&("tiktools-legacy-product-promotion-v1", legacy, receipts))
            }
            Some(receipts) => {
                serde_json::to_vec(&("tiktools-marked-cutover-review-v3", legacy, receipts))
            }
        }
    }

    /// Legacy keys that have neither a current local selection nor a pending
    /// v1 edit in this candidate. The old records remain in v1; callers must
    /// report them, not imply that they were migrated or intentionally deleted.
    pub fn unseeded_legacy_keys(&self) -> Vec<&str> {
        let seeded: std::collections::BTreeSet<_> = self
            .candidate
            .entries
            .iter()
            .map(|entry| entry.target.key.as_str())
            .collect();
        self.legacy
            .as_ref()
            .map(|legacy| {
                legacy
                    .state
                    .entries
                    .keys()
                    .filter(|key| !seeded.contains(key.as_str()))
                    .map(String::as_str)
                    .collect()
            })
            .unwrap_or_default()
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
    fn marked_receipt_requires_exact_pending_and_forces_review_without_mutation() {
        let mut state = SyncState::new("https://cloud.example".into(), "account".into());
        state
            .stage("acme.plugin".into(), Edit::Put(selection(false)))
            .unwrap();
        let original = state.entries["acme.plugin"].pending.clone().unwrap();
        state
            .entries
            .get_mut("acme.plugin")
            .unwrap()
            .pending
            .as_mut()
            .unwrap()
            .conflict = true;
        let snapshot = StateSnapshot {
            generation: 3,
            state,
        };
        let receipt =
            V1CutoverReceipt::new("acme.plugin".into(), 7, 12, "a".repeat(64), original).unwrap();
        let preview = CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(true)],
            Some(&snapshot),
            std::slice::from_ref(&receipt),
        )
        .unwrap();
        let entry = &preview.entries()[0];
        assert_eq!(
            entry.pending.as_ref().unwrap().status,
            WorkspacePendingStatus::Conflict
        );
        assert!(entry.next.is_some());
        assert!(
            snapshot.state.entries["acme.plugin"]
                .pending
                .as_ref()
                .unwrap()
                .conflict
        );
        assert!(CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(true)],
            Some(&snapshot),
            &[]
        )
        .is_err());
        let mut changed = snapshot.clone();
        changed
            .state
            .entries
            .get_mut("acme.plugin")
            .unwrap()
            .pending
            .as_mut()
            .unwrap()
            .base_revision = Some(10);
        assert!(CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(true)],
            Some(&changed),
            &[receipt]
        )
        .is_err());
    }

    #[test]
    fn settled_marked_receipt_requires_matching_remote_newer_than_base() {
        let mut state = SyncState::new("https://cloud.example".into(), "account".into());
        state.cursor = 5;
        state.entries.insert(
            "acme.plugin".into(),
            cloud_client::Entry {
                remote: Some(cloud_client::Record {
                    collection: "plugin-selections".into(),
                    key: "acme.plugin".into(),
                    revision: 5,
                    deleted: false,
                    value: Some(selection(false)),
                }),
                pending: None,
            },
        );
        let pending = cloud_client::Pending {
            base_revision: Some(4),
            edit: Edit::Put(selection(false)),
            conflict: false,
        };
        let receipt =
            V1CutoverReceipt::new("acme.plugin".into(), 1, 7, "a".repeat(64), pending).unwrap();
        let snapshot = StateSnapshot {
            generation: 1,
            state,
        };
        let preview = CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(false)],
            Some(&snapshot),
            std::slice::from_ref(&receipt),
        )
        .unwrap();
        assert_eq!(
            preview.entries()[0].pending.as_ref().unwrap().status,
            WorkspacePendingStatus::Conflict
        );
        let mut not_newer = snapshot.clone();
        not_newer
            .state
            .entries
            .get_mut("acme.plugin")
            .unwrap()
            .remote
            .as_mut()
            .unwrap()
            .revision = 4;
        assert!(CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(false)],
            Some(&not_newer),
            &[receipt]
        )
        .is_err());
    }

    #[test]
    fn settled_marked_delete_becomes_reviewable_v2_tombstone() {
        let mut state = SyncState::new("https://cloud.example".into(), "account".into());
        state.cursor = 5;
        state.entries.insert(
            "acme.plugin".into(),
            cloud_client::Entry {
                remote: Some(cloud_client::Record {
                    collection: "plugin-selections".into(),
                    key: "acme.plugin".into(),
                    revision: 5,
                    deleted: true,
                    value: None,
                }),
                pending: None,
            },
        );
        let pending = cloud_client::Pending {
            base_revision: Some(4),
            edit: Edit::Delete,
            conflict: false,
        };
        let receipt =
            V1CutoverReceipt::new("acme.plugin".into(), 1, 7, "a".repeat(64), pending).unwrap();
        let snapshot = StateSnapshot {
            generation: 1,
            state,
        };
        let preview = CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[],
            Some(&snapshot),
            &[receipt],
        )
        .unwrap();
        let item = preview.entries()[0].pending.as_ref().unwrap();
        assert_eq!(item.edit, WorkspaceEdit::Delete);
        assert_eq!(item.status, WorkspacePendingStatus::Conflict);
        assert!(preview.unseeded_legacy_keys().is_empty());
        let dir = tempfile::tempdir().unwrap();
        let coordinator = crate::WorkspaceCoordinator::open(
            dir.path().join("private"),
            "https://cloud.example".into(),
            "account".into(),
        )
        .unwrap();
        coordinator.bootstrap_reviewed(&preview).unwrap();
        let mut durable = coordinator.snapshot().unwrap().state;
        let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
        assert!(durable.prepare_push(&key).is_err());
        assert!(preview.matches_bootstrap(&durable));
    }

    #[test]
    fn marked_cutover_never_queues_unconfirmed_local_selections() {
        let preview = CutoverPreview::new_marked(
            "https://cloud.example".into(),
            "account".into(),
            &[selection(true)],
            None,
            &[],
        )
        .unwrap();
        assert_eq!(
            preview.entries()[0].pending.as_ref().unwrap().status,
            WorkspacePendingStatus::Conflict
        );
        let dir = tempfile::tempdir().unwrap();
        let coordinator = crate::WorkspaceCoordinator::open(
            dir.path().join("private"),
            "https://cloud.example".into(),
            "account".into(),
        )
        .unwrap();
        coordinator.bootstrap_reviewed(&preview).unwrap();
        let mut state = coordinator.snapshot().unwrap().state;
        let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
        assert!(state.prepare_push(&key).is_err());
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

    #[test]
    fn preview_reports_legacy_records_that_would_not_be_seeded() {
        let mut state = SyncState::new("https://cloud.example".into(), "account".into());
        state.entries.insert(
            "acme.old".into(),
            cloud_client::Entry {
                remote: Some(cloud_client::Record {
                    collection: "plugin-selections".into(),
                    key: "acme.old".into(),
                    revision: 1,
                    deleted: true,
                    value: None,
                }),
                pending: None,
            },
        );
        let preview = CutoverPreview::new(
            "https://cloud.example".into(),
            "account".into(),
            &[],
            Some(&StateSnapshot {
                generation: 1,
                state,
            }),
        )
        .unwrap();
        assert_eq!(preview.unseeded_legacy_keys(), vec!["acme.old"]);
        assert!(preview.entries().is_empty());
    }
}

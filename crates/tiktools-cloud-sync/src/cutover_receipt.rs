//! Native evidence for a reviewed migration of marked v1 outbox receipts.
use cloud_client::{Edit, Pending, SyncState};
use serde::Serialize;

/// Frozen receipt. Origin/account come from the bound session; this value is
/// never accepted from a UI. Construct it only from the guarded native store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1CutoverReceipt {
    key: String,
    generation: i64,
    #[serde(with = "crate::consent_generation")]
    consent_generation: u128,
    runtime_epoch: String,
    pending: Pending,
}

impl V1CutoverReceipt {
    pub fn new(
        key: String,
        generation: i64,
        consent_generation: u128,
        runtime_epoch: String,
        pending: Pending,
    ) -> Option<Self> {
        let mut probe = SyncState::new("https://validation.example".into(), "validation".into());
        if generation <= 0
            || consent_generation == 0
            || runtime_epoch.len() != 64
            || !runtime_epoch
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || pending.base_revision.is_some_and(|value| value <= 0)
            || probe.stage(key.clone(), pending.edit.clone()).is_err()
        {
            return None;
        }
        Some(Self {
            key,
            generation,
            consent_generation,
            runtime_epoch,
            pending,
        })
    }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn consent_generation(&self) -> u128 {
        self.consent_generation
    }
    pub fn pending(&self) -> &Pending {
        &self.pending
    }

    /// A 409 can promote the conflict flag after the receipt was written;
    /// no other change to the original pending request is accepted.
    pub(crate) fn covers_pending(&self, current: &Pending) -> bool {
        self.pending.base_revision == current.base_revision
            && self.pending.edit == current.edit
            && (!self.pending.conflict || current.conflict)
    }

    pub(crate) fn covers_remote(&self, remote: &cloud_client::Record) -> bool {
        if remote.revision <= self.pending.base_revision.unwrap_or(0) {
            return false;
        }
        match &self.pending.edit {
            Edit::Put(value) => !remote.deleted && remote.value.as_ref() == Some(value),
            Edit::Delete => remote.deleted && remote.value.is_none(),
        }
    }
}

//! V2 workspace synchronization state. Transport never applies application
//! source, and uncertainty never becomes permission to repeat a write.
use crate::{Session, SyncError, SyncState, WorkspaceChanges, WorkspaceRecord, WorkspaceSnapshot};
use cloud_contracts::portable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub(crate) const MAX_WORKSPACE_STATE_BYTES: usize = 48 * 1024 * 1024;
const VALUE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 1000;

#[derive(Debug, Error)]
pub enum WorkspaceStateError {
    #[error("workspace state or response is invalid")]
    Invalid,
    #[error("workspace state belongs to a different origin or account")]
    WrongIdentity,
    #[error("workspace snapshot rebuild requires explicit review")]
    NeedsRebuild,
    #[error("a current read is required before resolving this workspace edit")]
    NeedsRead,
    #[error("workspace edit requires explicit conflict or unknown-outcome review")]
    Conflict,
    #[error("workspace review changed; read and review again")]
    StaleReview,
    #[error("no pending workspace edit exists")]
    NoPendingEdit,
    #[error("workspace state exceeds its bounded capacity")]
    TooLarge,
}
type Result<T> = std::result::Result<T, WorkspaceStateError>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceKey {
    pub collection: String,
    pub key: String,
}
impl WorkspaceKey {
    pub fn new(collection: impl Into<String>, key: impl Into<String>) -> Result<Self> {
        let value = Self {
            collection: collection.into(),
            key: key.into(),
        };
        if !value.valid() {
            return Err(WorkspaceStateError::Invalid);
        }
        Ok(value)
    }
    fn valid(&self) -> bool {
        portable::valid_key(&self.collection, &self.key)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum WorkspaceEdit {
    Put(Value),
    Delete,
}
impl WorkspaceEdit {
    fn valid(&self, key: &WorkspaceKey) -> bool {
        key.valid()
            && match self {
                Self::Put(value) => portable::validate(&key.collection, &key.key, value),
                Self::Delete => true,
            }
    }
    fn bytes(&self) -> Result<usize> {
        match self {
            Self::Put(value) => value_bytes(value),
            Self::Delete => Ok(0),
        }
    }
    fn matches(&self, record: &WorkspaceRecord) -> bool {
        match self {
            Self::Put(value) => {
                !record.deleted
                    && record
                        .value
                        .as_ref()
                        .is_some_and(|remote| portable::values_equivalent(value, remote))
            }
            Self::Delete => record.deleted && record.value.is_none(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspacePendingStatus {
    Queued,
    Unconfirmed,
    Conflict,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePending {
    pub base_revision: Option<i64>,
    pub edit: WorkspaceEdit,
    pub local_generation: u64,
    pub base_record: Option<WorkspaceRecord>,
    pub status: WorkspacePendingStatus,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceLocalEdit {
    pub edit: WorkspaceEdit,
    pub local_generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceEntry {
    pub target: WorkspaceKey,
    pub remote: Option<WorkspaceRecord>,
    pub pending: Option<WorkspacePending>,
    /// A newer local decision cannot replace the frozen uncertain request.
    pub next: Option<WorkspaceLocalEdit>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceState {
    pub origin: String,
    pub account_id: String,
    pub cursor: i64,
    pub needs_rebuild: bool,
    pub bootstrap_complete: bool,
    pub next_local_generation: u64,
    pub entries: Vec<WorkspaceEntry>,
    /// Private local product-source receipts; never uploaded to Cloud.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    product_receipts: Vec<WorkspaceProductReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceProductReceipt {
    target: WorkspaceKey,
    source_generation: u64,
    decision_digest: String,
    edit: WorkspaceEdit,
    workspace_generation: u64,
}

/// Non-deserializable snapshot of the source receipt for one product edit.
/// This is evidence to compare under the product source fence, not authority
/// to send or apply anything by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceProductProof(WorkspaceProductReceipt);
impl WorkspaceProductProof {
    pub fn target(&self) -> &WorkspaceKey {
        &self.0.target
    }
    pub fn source_generation(&self) -> u64 {
        self.0.source_generation
    }
    pub fn decision_digest(&self) -> &str {
        &self.0.decision_digest
    }
    pub fn edit(&self) -> &WorkspaceEdit {
        &self.0.edit
    }
    pub fn local_generation(&self) -> u64 {
        self.0.workspace_generation
    }
}

/// Non-deserializable proof of exactly the request durably prepared for HTTP.
#[derive(Clone)]
pub struct WorkspacePush {
    origin: String,
    account_id: String,
    target: WorkspaceKey,
    pending: WorkspacePending,
}
impl WorkspacePush {
    pub fn target(&self) -> &WorkspaceKey {
        &self.target
    }
    pub fn base_revision(&self) -> Option<i64> {
        self.pending.base_revision
    }
    pub fn edit(&self) -> &WorkspaceEdit {
        &self.pending.edit
    }
    pub fn local_generation(&self) -> u64 {
        self.pending.local_generation
    }
}
/// Only an actual authenticated target GET (including verified 404) creates a
/// resolution read. A cached equal body is never an original-write receipt.
pub struct WorkspaceRead {
    origin: String,
    account_id: String,
    target: WorkspaceKey,
    record: Option<WorkspaceRecord>,
}
impl WorkspaceRead {
    pub fn target(&self) -> &WorkspaceKey {
        &self.target
    }
    pub fn record(&self) -> Option<&WorkspaceRecord> {
        self.record.as_ref()
    }
    pub fn revision(&self) -> Option<i64> {
        self.record.as_ref().map(|r| r.revision)
    }
}

fn value_bytes(value: &Value) -> Result<usize> {
    serde_json::to_vec(value)
        .map(|v| v.len())
        .map_err(|_| WorkspaceStateError::Invalid)
}
fn identity_valid(origin: &str, account: &str) -> bool {
    crate::parse_origin(origin).is_ok_and(|url| url.origin().ascii_serialization() == origin)
        && !account.is_empty()
        && account.len() <= 256
        && account.trim() == account
        && !account.chars().any(char::is_control)
}
fn valid_record(target: &WorkspaceKey, record: &WorkspaceRecord) -> bool {
    record.collection == target.collection
        && record.key == target.key
        && record.valid(record.revision)
}
fn remote_revision(entry: &WorkspaceEntry) -> Option<i64> {
    entry.remote.as_ref().map(|r| r.revision)
}
impl WorkspaceState {
    /// Capture a receipt only when it identifies the currently queued edit.
    /// Unconfirmed writes and newer edits never inherit an older receipt.
    pub fn product_queued_proof(&self, target: &WorkspaceKey) -> Option<WorkspaceProductProof> {
        let receipt = self
            .product_receipts
            .binary_search_by(|r| r.target.cmp(target))
            .ok()
            .and_then(|index| self.product_receipts.get(index))?;
        let pending = self.entry(target)?.pending.as_ref()?;
        (pending.status == WorkspacePendingStatus::Queued
            && pending.local_generation == receipt.workspace_generation
            && pending.edit == receipt.edit)
            .then(|| WorkspaceProductProof(receipt.clone()))
    }

    /// Recheck an opaque proof against the current state while source is fenced.
    pub fn verify_product_queued(&self, proof: &WorkspaceProductProof) -> Result<()> {
        if self.product_queued_proof(proof.target()).as_ref() != Some(proof) {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(())
    }

    /// Capture the exact latest product decision while its prior upload needs
    /// an explicit conflict or uncertain-outcome review. A newer staged edit
    /// supersedes the original intent, and only its own receipt can authorize
    /// a rebase. A receipt on its own is never permission to resend.
    pub fn product_recovery_proof(&self, target: &WorkspaceKey) -> Option<WorkspaceProductProof> {
        let receipt = self
            .product_receipts
            .binary_search_by(|r| r.target.cmp(target))
            .ok()
            .and_then(|index| self.product_receipts.get(index))?;
        let entry = self.entry(target)?;
        let pending = entry.pending.as_ref()?;
        if !matches!(
            pending.status,
            WorkspacePendingStatus::Unconfirmed | WorkspacePendingStatus::Conflict
        ) {
            return None;
        }
        let (edit, generation) = match &entry.next {
            Some(next) => (&next.edit, next.local_generation),
            None => (&pending.edit, pending.local_generation),
        };
        (generation == receipt.workspace_generation && edit == &receipt.edit)
            .then(|| WorkspaceProductProof(receipt.clone()))
    }

    /// Recheck an opaque recovery proof immediately before changing state.
    pub fn verify_product_recovery(&self, proof: &WorkspaceProductProof) -> Result<()> {
        if self.product_recovery_proof(proof.target()).as_ref() != Some(proof) {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(())
    }

    /// Persist this transition before the caller sends the approved request.
    pub fn prepare_product_push(&mut self, proof: &WorkspaceProductProof) -> Result<WorkspacePush> {
        self.verify_product_queued(proof)?;
        self.prepare_push(proof.target())
    }
    pub fn new(origin: String, account_id: String) -> Result<Self> {
        if !identity_valid(&origin, &account_id) {
            return Err(WorkspaceStateError::Invalid);
        }
        Ok(Self {
            origin,
            account_id,
            cursor: 0,
            needs_rebuild: false,
            bootstrap_complete: false,
            next_local_generation: 1,
            entries: Vec::new(),
            product_receipts: Vec::new(),
        })
    }
    pub fn validate(&self) -> Result<()> {
        if !identity_valid(&self.origin, &self.account_id)
            || self.cursor < 0
            || self.next_local_generation == 0
        {
            return Err(WorkspaceStateError::Invalid);
        }
        if self.entries.len() > MAX_ENTRIES {
            return Err(WorkspaceStateError::TooLarge);
        }
        if self.product_receipts.len() > MAX_ENTRIES {
            return Err(WorkspaceStateError::TooLarge);
        }
        let mut receipt_previous = None;
        let mut receipt_bytes = 0usize;
        for receipt in &self.product_receipts {
            if !receipt.target.valid()
                || receipt_previous.is_some_and(|key| key >= &receipt.target)
                || receipt.source_generation == 0
                || receipt.workspace_generation == 0
                || receipt.workspace_generation >= self.next_local_generation
                || receipt.decision_digest.len() != 64
                || !receipt
                    .decision_digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !receipt.edit.valid(&receipt.target)
            {
                return Err(WorkspaceStateError::Invalid);
            }
            receipt_previous = Some(&receipt.target);
            receipt_bytes = receipt_bytes
                .checked_add(receipt.edit.bytes()?)
                .ok_or(WorkspaceStateError::TooLarge)?;
            if receipt_bytes > VALUE_BYTES {
                return Err(WorkspaceStateError::TooLarge);
            }
        }
        let mut previous = None;
        let mut totals = [0usize; 4];
        for entry in &self.entries {
            if !entry.target.valid() || previous.is_some_and(|key| key >= &entry.target) {
                return Err(WorkspaceStateError::Invalid);
            }
            if entry.remote.is_none() && entry.pending.is_none() && entry.next.is_none() {
                return Err(WorkspaceStateError::Invalid);
            }
            previous = Some(&entry.target);
            if let Some(remote) = &entry.remote {
                if !valid_record(&entry.target, remote) {
                    return Err(WorkspaceStateError::Invalid);
                }
                if let Some(value) = &remote.value {
                    totals[0] = totals[0]
                        .checked_add(value_bytes(value)?)
                        .ok_or(WorkspaceStateError::TooLarge)?;
                }
            }
            if let Some(pending) = &entry.pending {
                if !pending.edit.valid(&entry.target)
                    || pending.local_generation == 0
                    || pending.local_generation >= self.next_local_generation
                    || pending.base_revision.is_some_and(|n| n < 1)
                    || pending.base_revision
                        != pending.base_record.as_ref().map(|record| record.revision)
                    || pending
                        .base_record
                        .as_ref()
                        .is_some_and(|record| !valid_record(&entry.target, record))
                    || (pending.status == WorkspacePendingStatus::Queued
                        && pending.base_revision != remote_revision(entry))
                {
                    return Err(WorkspaceStateError::Invalid);
                }
                totals[1] = totals[1]
                    .checked_add(pending.edit.bytes()?)
                    .ok_or(WorkspaceStateError::TooLarge)?;
                if let Some(value) = pending
                    .base_record
                    .as_ref()
                    .and_then(|record| record.value.as_ref())
                {
                    totals[3] = totals[3]
                        .checked_add(value_bytes(value)?)
                        .ok_or(WorkspaceStateError::TooLarge)?;
                }
            }
            if let Some(next) = &entry.next {
                if !next.edit.valid(&entry.target)
                    || next.local_generation >= self.next_local_generation
                    || !entry.pending.as_ref().is_some_and(|p| {
                        p.status != WorkspacePendingStatus::Queued
                            && next.local_generation > p.local_generation
                    })
                {
                    return Err(WorkspaceStateError::Invalid);
                }
                totals[2] = totals[2]
                    .checked_add(next.edit.bytes()?)
                    .ok_or(WorkspaceStateError::TooLarge)?;
            }
            if totals.iter().any(|total| *total > VALUE_BYTES) {
                return Err(WorkspaceStateError::TooLarge);
            }
        }
        Ok(())
    }
    fn identity(&self, origin: &str, account: &str) -> Result<()> {
        if self.origin != origin || self.account_id != account {
            return Err(WorkspaceStateError::WrongIdentity);
        }
        Ok(())
    }
    fn index(&self, key: &WorkspaceKey) -> std::result::Result<usize, usize> {
        self.entries.binary_search_by(|entry| entry.target.cmp(key))
    }
    fn generation(&mut self) -> Result<u64> {
        let value = self.next_local_generation;
        self.next_local_generation = value.checked_add(1).ok_or(WorkspaceStateError::TooLarge)?;
        Ok(value)
    }
    pub fn entry(&self, key: &WorkspaceKey) -> Option<&WorkspaceEntry> {
        self.index(key).ok().map(|i| &self.entries[i])
    }
    pub fn local_generation(&self, key: &WorkspaceKey) -> Option<u64> {
        self.entry(key).and_then(|entry| {
            entry
                .next
                .as_ref()
                .map(|v| v.local_generation)
                .or_else(|| entry.pending.as_ref().map(|v| v.local_generation))
        })
    }
    /// Stage is transactional in memory; callers durably save before claiming
    /// an edit was staged. A prepared original request is never overwritten.
    /// Atomically record an already durable product decision and its receipt.
    /// The source must fence its identity and generation until this state is
    /// saved. Repeating the exact decision is a no-op even if its pending
    /// write has since been sent or resolved. A stale or ambiguous decision
    /// requires explicit recovery, never automatic replay.
    pub fn stage_product_decision(
        &mut self,
        key: WorkspaceKey,
        edit: WorkspaceEdit,
        source_generation: u64,
        decision_digest: &str,
    ) -> Result<(u64, bool)> {
        if !self.bootstrap_complete
            || !key.valid()
            || !edit.valid(&key)
            || source_generation == 0
            || decision_digest.len() != 64
            || !decision_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(WorkspaceStateError::Invalid);
        }
        let position = self
            .product_receipts
            .binary_search_by(|receipt| receipt.target.cmp(&key));
        if let Ok(index) = position {
            let receipt = &self.product_receipts[index];
            if source_generation <= receipt.source_generation {
                if source_generation == receipt.source_generation
                    && decision_digest == receipt.decision_digest
                    && edit == receipt.edit
                {
                    return Ok((receipt.workspace_generation, false));
                }
                return Err(WorkspaceStateError::Invalid);
            }
        }
        let mut candidate = self.clone();
        let generation = candidate.stage(key.clone(), edit.clone())?;
        let receipt = WorkspaceProductReceipt {
            target: key,
            source_generation,
            decision_digest: decision_digest.to_owned(),
            edit,
            workspace_generation: generation,
        };
        match position {
            Ok(index) => candidate.product_receipts[index] = receipt,
            Err(index) => candidate.product_receipts.insert(index, receipt),
        }
        candidate.validate()?;
        *self = candidate;
        Ok((generation, true))
    }

    pub fn stage(&mut self, key: WorkspaceKey, edit: WorkspaceEdit) -> Result<u64> {
        if !edit.valid(&key) {
            return Err(WorkspaceStateError::Invalid);
        }
        let mut candidate = self.clone();
        let generation = candidate.generation()?;
        let index = match candidate.index(&key) {
            Ok(i) => i,
            Err(i) => {
                candidate.entries.insert(
                    i,
                    WorkspaceEntry {
                        target: key,
                        remote: None,
                        pending: None,
                        next: None,
                    },
                );
                i
            }
        };
        let entry = &mut candidate.entries[index];
        if let Some(pending) = &mut entry.pending {
            if pending.status == WorkspacePendingStatus::Queued {
                pending.edit = edit;
                pending.local_generation = generation;
            } else {
                entry.next = Some(WorkspaceLocalEdit {
                    edit,
                    local_generation: generation,
                });
            }
        } else {
            entry.pending = Some(WorkspacePending {
                base_revision: remote_revision(entry),
                base_record: entry.remote.clone(),
                edit,
                local_generation: generation,
                status: WorkspacePendingStatus::Queued,
            });
        }
        candidate.validate()?;
        *self = candidate;
        Ok(generation)
    }
    /// The changed state MUST be saved successfully before sending this ticket.
    /// A restart leaves Unconfirmed and refuses automatic repeat dispatch.
    pub fn prepare_push(&mut self, key: &WorkspaceKey) -> Result<WorkspacePush> {
        if self.needs_rebuild {
            return Err(WorkspaceStateError::NeedsRebuild);
        }
        let index = self
            .index(key)
            .map_err(|_| WorkspaceStateError::NoPendingEdit)?;
        let pending = self.entries[index]
            .pending
            .as_mut()
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        if pending.status != WorkspacePendingStatus::Queued {
            return Err(WorkspaceStateError::Conflict);
        }
        if pending.base_revision.is_none() && matches!(pending.edit, WorkspaceEdit::Delete) {
            return Err(WorkspaceStateError::NeedsRead);
        }
        pending.status = WorkspacePendingStatus::Unconfirmed;
        Ok(WorkspacePush {
            origin: self.origin.clone(),
            account_id: self.account_id.clone(),
            target: key.clone(),
            pending: pending.clone(),
        })
    }
    fn ticket_index(&self, ticket: &WorkspacePush) -> Result<usize> {
        self.identity(&ticket.origin, &ticket.account_id)?;
        let index = self
            .index(&ticket.target)
            .map_err(|_| WorkspaceStateError::StaleReview)?;
        if !self.entries[index].pending.as_ref().is_some_and(|p| {
            p.base_revision == ticket.pending.base_revision
                && p.base_record == ticket.pending.base_record
                && p.edit == ticket.pending.edit
                && p.local_generation == ticket.pending.local_generation
                && p.status != WorkspacePendingStatus::Queued
        }) {
            return Err(WorkspaceStateError::StaleReview);
        }
        Ok(index)
    }
    pub fn acknowledge(&mut self, ticket: &WorkspacePush, record: WorkspaceRecord) -> Result<()> {
        let index = self.ticket_index(ticket)?;
        if !valid_record(&ticket.target, &record)
            || !ticket.pending.edit.matches(&record)
            || ticket
                .pending
                .base_revision
                .is_some_and(|base| record.revision < base)
        {
            return Err(WorkspaceStateError::Invalid);
        }
        let mut candidate = self.clone();
        let entry = &mut candidate.entries[index];
        let revision = record.revision;
        let acknowledged = record.clone();
        if entry
            .remote
            .as_ref()
            .is_none_or(|old| old.revision < revision)
        {
            entry.remote = Some(record);
        } else if entry
            .remote
            .as_ref()
            .is_some_and(|old| old.revision == revision && !old.equivalent(&record))
        {
            return Err(WorkspaceStateError::Invalid);
        }
        entry.pending = entry.next.take().map(|next| WorkspacePending {
            base_revision: Some(revision),
            base_record: Some(acknowledged),
            edit: next.edit,
            local_generation: next.local_generation,
            status: if remote_revision(entry) == Some(revision) {
                WorkspacePendingStatus::Queued
            } else {
                WorkspacePendingStatus::Conflict
            },
        });
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn mark_conflict(&mut self, ticket: &WorkspacePush) -> Result<()> {
        let index = self.ticket_index(ticket)?;
        self.entries[index].pending.as_mut().unwrap().status = WorkspacePendingStatus::Conflict;
        Ok(())
    }
    fn apply_record(&mut self, record: WorkspaceRecord) -> Result<()> {
        let key = WorkspaceKey::new(record.collection.clone(), record.key.clone())?;
        if !valid_record(&key, &record) {
            return Err(WorkspaceStateError::Invalid);
        }
        let index = match self.index(&key) {
            Ok(i) => i,
            Err(i) => {
                self.entries.insert(
                    i,
                    WorkspaceEntry {
                        target: key,
                        remote: None,
                        pending: None,
                        next: None,
                    },
                );
                i
            }
        };
        let entry = &mut self.entries[index];
        if let Some(old) = &entry.remote {
            if old.revision > record.revision {
                return Ok(());
            }
            if old.revision == record.revision {
                return if old.equivalent(&record) {
                    Ok(())
                } else {
                    Err(WorkspaceStateError::Invalid)
                };
            }
        }
        if let Some(pending) = &mut entry.pending {
            if pending.status == WorkspacePendingStatus::Queued
                && pending.base_revision != Some(record.revision)
            {
                pending.status = WorkspacePendingStatus::Conflict;
            }
        }
        entry.remote = Some(record);
        Ok(())
    }
    pub fn apply_changes(&mut self, page: WorkspaceChanges) -> Result<bool> {
        if self.needs_rebuild {
            return Err(WorkspaceStateError::NeedsRebuild);
        }
        if !page.valid_after(self.cursor) {
            return Err(WorkspaceStateError::Invalid);
        }
        let mut candidate = self.clone();
        for record in page.changes {
            candidate.apply_record(record)?;
        }
        candidate.cursor = page.cursor;
        candidate.validate()?;
        *self = candidate;
        Ok(page.has_more)
    }
    pub fn require_rebuild(&mut self) {
        self.needs_rebuild = true;
    }
    /// Only an explicit read-only snapshot rebuild choice calls this method.
    /// It replaces the remote cache, preserving every local intent; application
    /// of these records remains a separate reviewed application-domain action.
    pub fn install_rebuild(&mut self, snapshot: WorkspaceSnapshot) -> Result<()> {
        if !snapshot.valid() {
            return Err(WorkspaceStateError::Invalid);
        }
        if snapshot.cursor < self.cursor {
            return Err(WorkspaceStateError::StaleReview);
        }
        for old in self
            .entries
            .iter()
            .filter_map(|entry| entry.remote.as_ref())
        {
            let replacement = snapshot
                .records
                .iter()
                .find(|record| record.collection == old.collection && record.key == old.key);
            if old.revision > snapshot.cursor
                || !replacement
                    .is_some_and(|record| record.revision > old.revision || record.equivalent(old))
            {
                return Err(WorkspaceStateError::StaleReview);
            }
        }
        let mut candidate = Self::new(self.origin.clone(), self.account_id.clone())?;
        candidate.cursor = snapshot.cursor;
        candidate.bootstrap_complete = self.bootstrap_complete;
        candidate.next_local_generation = self.next_local_generation;
        candidate.product_receipts = self.product_receipts.clone();
        for record in snapshot.records {
            candidate.apply_record(record)?;
        }
        for old in &self.entries {
            if old.pending.is_none() {
                continue;
            }
            let index = match candidate.index(&old.target) {
                Ok(i) => i,
                Err(i) => {
                    candidate.entries.insert(
                        i,
                        WorkspaceEntry {
                            target: old.target.clone(),
                            remote: None,
                            pending: None,
                            next: None,
                        },
                    );
                    i
                }
            };
            let entry = &mut candidate.entries[index];
            let mut pending = old.pending.clone().unwrap();
            if pending.status == WorkspacePendingStatus::Queued
                && pending.base_revision != remote_revision(entry)
            {
                pending.status = WorkspacePendingStatus::Conflict;
            }
            entry.pending = Some(pending);
            entry.next = old.next.clone();
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn observe_read(&mut self, read: &WorkspaceRead) -> Result<()> {
        self.identity(&read.origin, &read.account_id)?;
        let mut candidate = self.clone();
        match &read.record {
            Some(record) => candidate.apply_record(record.clone())?,
            None => {
                if let Some(entry) = candidate.entry(&read.target) {
                    if entry.remote.is_some() {
                        return Err(WorkspaceStateError::StaleReview);
                    }
                }
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    /// Complete only the exact queued delete captured before the target GET.
    /// A newer equal-looking deletion is a different local intent and cannot
    /// be cleared with the earlier read's absence proof.
    pub fn complete_absent_delete(
        &mut self,
        read: &WorkspaceRead,
        expected_local_generation: u64,
    ) -> Result<()> {
        self.identity(&read.origin, &read.account_id)?;
        let index = self
            .index(&read.target)
            .map_err(|_| WorkspaceStateError::StaleReview)?;
        let entry = &self.entries[index];
        if read.record.is_some()
            || entry.remote.is_some()
            || entry.next.is_some()
            || !entry.pending.as_ref().is_some_and(|pending| {
                pending.status == WorkspacePendingStatus::Queued
                    && pending.base_revision.is_none()
                    && pending.base_record.is_none()
                    && matches!(pending.edit, WorkspaceEdit::Delete)
                    && pending.local_generation == expected_local_generation
            })
        {
            return Err(WorkspaceStateError::StaleReview);
        }
        let mut candidate = self.clone();
        candidate.entries.remove(index);
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    /// A new live read must still match the revision displayed for explicit
    /// choice, and the local generation must still be the reviewed candidate.
    pub fn resolve(
        &mut self,
        read: &WorkspaceRead,
        expected_remote_revision: Option<i64>,
        expected_local_generation: u64,
        keep_local: bool,
    ) -> Result<()> {
        self.identity(&read.origin, &read.account_id)?;
        if read.revision() != expected_remote_revision
            || self.local_generation(&read.target) != Some(expected_local_generation)
        {
            return Err(WorkspaceStateError::StaleReview);
        }
        if self.entry(&read.target).and_then(remote_revision) > read.revision() {
            return Err(WorkspaceStateError::StaleReview);
        }
        let mut candidate = self.clone();
        let index = candidate
            .index(&read.target)
            .map_err(|_| WorkspaceStateError::NoPendingEdit)?;
        let entry = &mut candidate.entries[index];
        if let Some(record) = &read.record {
            if !valid_record(&read.target, record) {
                return Err(WorkspaceStateError::Invalid);
            }
        }
        if entry
            .remote
            .as_ref()
            .zip(read.record.as_ref())
            .is_some_and(|(old, new)| old.revision == new.revision && !old.equivalent(new))
        {
            return Err(WorkspaceStateError::Invalid);
        }
        entry.remote = read.record.clone();
        let desired = entry
            .next
            .take()
            .map(|next| next.edit)
            .or_else(|| entry.pending.as_ref().map(|p| p.edit.clone()))
            .ok_or(WorkspaceStateError::NoPendingEdit)?;
        if keep_local && !(read.record.is_none() && matches!(desired, WorkspaceEdit::Delete)) {
            entry.pending = Some(WorkspacePending {
                base_revision: read.revision(),
                base_record: read.record.clone(),
                edit: desired,
                local_generation: expected_local_generation,
                status: WorkspacePendingStatus::Queued,
            });
        } else {
            entry.pending = None;
        }
        if entry.remote.is_none() && entry.pending.is_none() && entry.next.is_none() {
            candidate.entries.remove(index);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn ready_remote(&self) -> Vec<WorkspaceRecord> {
        if self.needs_rebuild {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|entry| entry.pending.is_none())
            .filter_map(|entry| entry.remote.clone())
            .collect()
    }
    pub fn pending_keys(&self) -> Vec<WorkspaceKey> {
        self.entries
            .iter()
            .filter(|entry| entry.pending.is_some())
            .map(|entry| entry.target.clone())
            .collect()
    }
    pub fn conflict_keys(&self) -> Vec<WorkspaceKey> {
        self.entries
            .iter()
            .filter(|entry| {
                entry
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.status != WorkspacePendingStatus::Queued)
            })
            .map(|entry| entry.target.clone())
            .collect()
    }
    pub fn mark_bootstrap_complete(&mut self) {
        self.bootstrap_complete = true;
    }
    /// Explicit cutover only. V1 bases/cursor are never v2 preconditions. A
    /// differing current staged edit is retained as the newer local choice and
    /// requires review together with the imported original pending intent.
    pub fn import_v1_pending(&mut self, legacy: &SyncState) -> Result<usize> {
        self.identity(&legacy.origin, &legacy.account_id)?;
        if self.bootstrap_complete {
            return Err(WorkspaceStateError::StaleReview);
        }
        if self.entries.iter().any(|entry| {
            entry.next.is_some()
                || entry
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.status != WorkspacePendingStatus::Queued)
        }) {
            return Err(WorkspaceStateError::Conflict);
        }
        let mut candidate = self.clone();
        let mut count = 0;
        for (key, old) in &legacy.entries {
            let Some(old) = &old.pending else {
                continue;
            };
            let target = WorkspaceKey::new("plugin-selections", key.clone())?;
            let edit = match &old.edit {
                crate::Edit::Put(value) => WorkspaceEdit::Put(
                    serde_json::to_value(value).map_err(|_| WorkspaceStateError::Invalid)?,
                ),
                crate::Edit::Delete => WorkspaceEdit::Delete,
            };
            if !edit.valid(&target) {
                return Err(WorkspaceStateError::Invalid);
            }
            let generation = candidate.generation()?;
            let index = match candidate.index(&target) {
                Ok(i) => i,
                Err(i) => {
                    candidate.entries.insert(
                        i,
                        WorkspaceEntry {
                            target,
                            remote: None,
                            pending: None,
                            next: None,
                        },
                    );
                    i
                }
            };
            let previous = candidate.entries[index]
                .pending
                .as_ref()
                .map(|p| p.edit.clone());
            let differs = previous.as_ref().is_some_and(|value| *value != edit);
            let next = if differs {
                Some(WorkspaceLocalEdit {
                    edit: previous.unwrap(),
                    local_generation: candidate.generation()?,
                })
            } else {
                None
            };
            let entry = &mut candidate.entries[index];
            entry.pending = Some(WorkspacePending {
                base_revision: None,
                base_record: None,
                edit,
                local_generation: generation,
                status: if old.conflict || differs || entry.remote.is_some() {
                    WorkspacePendingStatus::Conflict
                } else {
                    WorkspacePendingStatus::Queued
                },
            });
            entry.next = next;
            count += 1;
        }
        candidate.validate()?;
        *self = candidate;
        Ok(count)
    }
}

impl Session {
    pub async fn workspace_read_for_resolution(
        &self,
        key: &WorkspaceKey,
    ) -> std::result::Result<WorkspaceRead, SyncError> {
        if !key.valid() {
            return Err(SyncError::Configuration);
        }
        let record = match self.workspace_record(&key.collection, &key.key).await {
            Ok(record) => Some(record),
            Err(SyncError::Http(404)) => None,
            Err(error) => return Err(error),
        };
        Ok(WorkspaceRead {
            origin: self.instance_origin(),
            account_id: self.account_id().into(),
            target: key.clone(),
            record,
        })
    }
}

#[cfg(test)]
mod tests;

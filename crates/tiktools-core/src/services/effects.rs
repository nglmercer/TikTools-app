//! Non-replayable handoff of approved external effects. These private tokens
//! are created by trusted runtime code and never deserialized from event/RPC JSON.
use crate::{
    collaboration::hash_secret,
    control::{EffectKind, EffectReceipt},
    db::{DatabaseError, DatabaseManager},
};
use std::sync::Arc;

// Two named saved identities use 16 components per event/action hop. Three
// trusted emit levels fit without weakening the separate runtime depth limit.
const MAX_SLOT_DEPTH: usize = 96;

/// One trusted execution root and its stable child/action slot path. Clone this
/// through queues and retries; creating a fresh root explicitly starts new work.
#[derive(Clone)]
pub(crate) struct EffectExecution {
    root: String,
    slots: Vec<u32>,
}

/// A literal runtime operation, never supplied by an event or RPC caller.
#[derive(Clone, Copy)]
pub(crate) enum ApprovedPointsOperation {
    Adjust,
    Subtract,
}

impl ApprovedPointsOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Adjust => "core.points",
            Self::Subtract => "core.points.subtract",
        }
    }
}

/// Balance and identity are committed together by the points writer. This is
/// deliberately separate from the external-effect journal: a preclaimed
/// receipt in another database could lose a balance update after a crash.
#[derive(Clone)]
pub(crate) struct ApprovedPointsKey {
    id: String,
    fingerprint: String,
}

impl ApprovedPointsKey {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

fn random_identity() -> Result<String, DatabaseError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| DatabaseError::Invalid("cannot create private effect identity".into()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl EffectExecution {
    pub(crate) fn fresh() -> Result<Self, DatabaseError> {
        Ok(Self {
            root: random_identity()?,
            slots: Vec::new(),
        })
    }

    pub(crate) fn child(&self, slot: u32) -> Result<Self, DatabaseError> {
        if self.slots.len() >= MAX_SLOT_DEPTH {
            return Err(DatabaseError::Invalid("effect slot depth exceeded".into()));
        }
        let mut child = self.clone();
        child.slots.push(slot);
        Ok(child)
    }

    /// Saved record IDs identify branches independently of match iteration
    /// order. Inputs come from the trusted saved-record resolver, never an
    /// event payload. A full digest prevents ordinal/hash-prefix collisions.
    pub(crate) fn named_child(&self, namespace: &str, id: &str) -> Result<Self, DatabaseError> {
        if namespace.is_empty() || namespace.len() > 128 || id.is_empty() || id.len() > 256 {
            return Err(DatabaseError::Invalid("invalid named effect slot".into()));
        }
        let digest = hash_secret(&serde_json::to_string(&(
            "tiktools-effect-slot-v1",
            namespace,
            id,
        ))?);
        let mut child = self.clone();
        for part in digest.as_bytes().as_chunks::<8>().0 {
            let part = std::str::from_utf8(part)
                .ok()
                .and_then(|part| u32::from_str_radix(part, 16).ok())
                .ok_or_else(|| DatabaseError::Invalid("invalid named effect slot digest".into()))?;
            child = child.child(part)?;
        }
        Ok(child)
    }

    /// The actual typed native ingress identity remains stable across
    /// reconnects/restarts. Editable event JSON and connection generations do
    /// not participate. Missing upstream identity explicitly starts fresh work.
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn native(
        event: &crate::NativeLiveEvent,
        context: &crate::LiveContext,
    ) -> Result<Self, DatabaseError> {
        if event.msg_id() == 0
            || context.unique_id.trim().is_empty()
            || context.room_id.trim().is_empty()
        {
            return Self::fresh();
        }
        let identity = serde_json::to_string(&(
            "tiktools-native-effect-execution-v1",
            &context.unique_id,
            &context.room_id,
            event.method(),
            crate::live_events::dedup::event_tag(&event.base),
            event.msg_id(),
        ))?;
        Ok(Self {
            root: hash_secret(&identity),
            slots: Vec::new(),
        })
    }

    /// Semantic fingerprints are checked separately from identity: changing an
    /// approved definition or a rendered request never creates a new effect ID.
    pub(crate) fn key(
        &self,
        kind: EffectKind,
        binding_digest: &str,
        request_digest: &str,
    ) -> Result<EffectKey, DatabaseError> {
        if !valid_digest(binding_digest) || !valid_digest(request_digest) {
            return Err(DatabaseError::Invalid("invalid effect fingerprint".into()));
        }
        let path = self
            .slots
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("/");
        Ok(EffectKey {
            id: hash_secret(&format!(
                "tiktools-external-effect-v1:{}:{path}:{}",
                self.root,
                kind.as_str()
            )),
            kind,
            binding_digest: binding_digest.into(),
            request_digest: request_digest.into(),
        })
    }

    pub(crate) fn points_key(
        &self,
        operation: ApprovedPointsOperation,
        binding_digest: &str,
        request_digest: &str,
    ) -> Result<ApprovedPointsKey, DatabaseError> {
        if !valid_digest(binding_digest) || !valid_digest(request_digest) {
            return Err(DatabaseError::Invalid("invalid points fingerprint".into()));
        }
        Ok(ApprovedPointsKey {
            id: hash_secret(&serde_json::to_string(&(
                "tiktools-approved-points-execution-v1",
                &self.root,
                &self.slots,
                operation.as_str(),
            ))?),
            fingerprint: hash_secret(&serde_json::to_string(&(
                "tiktools-approved-points-semantics-v1",
                binding_digest,
                request_digest,
            ))?),
        })
    }
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl EffectKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Audio => "audio",
            Self::Plugin => "plugin",
            Self::Emit => "emit",
            Self::Script => "script",
        }
    }
}

#[derive(Clone)]
pub(crate) struct EffectKey {
    id: String,
    kind: EffectKind,
    binding_digest: String,
    request_digest: String,
}
impl EffectKey {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    pub(crate) fn kind(&self) -> EffectKind {
        self.kind
    }
    pub(crate) fn binding_digest(&self) -> &str {
        &self.binding_digest
    }
    pub(crate) fn request_digest(&self) -> &str {
        &self.request_digest
    }
}

struct JournalInner {
    db: Arc<DatabaseManager>,
    instance: String,
}

/// Share one journal per running host. Opening a successor fences the old
/// instance and changes unfinished handoffs to Unknown without dispatching them.
#[derive(Clone)]
pub(crate) struct EffectJournal(Arc<JournalInner>);

pub(crate) enum EffectAdmission {
    Fresh(EffectTicket),
    Retained(EffectReceipt),
}

/// Only a fresh successful reservation can construct this one-shot ticket.
pub(crate) struct EffectTicket {
    journal: EffectJournal,
    key: EffectKey,
    armed: bool,
}

/// The successful durable dispatch transition is the irrevocable handoff point.
/// This token is consumed once by completion; cancellation records Unknown.
pub(crate) struct EffectDispatch {
    journal: EffectJournal,
    key: EffectKey,
    armed: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum EffectOutcome {
    Succeeded,
    /// A definite known failure, never an ambiguous timeout/cancellation.
    Failed,
    Unknown,
}

impl EffectJournal {
    pub(crate) fn open(db: Arc<DatabaseManager>) -> Result<Self, DatabaseError> {
        let instance = random_identity()?;
        db.recover_external_effects(&instance)?;
        Ok(Self(Arc::new(JournalInner { db, instance })))
    }

    /// The caller invokes `commit` exactly once from its final synchronous
    /// authorization callback. That callback must hold every subject, saved
    /// binding, grant/configuration and host-settings fence until commit returns.
    /// No request, result or error payload is passed to persistent storage.
    pub(crate) fn reserve(
        &self,
        key: EffectKey,
        admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
    ) -> Result<EffectAdmission, String> {
        match self
            .0
            .db
            .reserve_external_effect(&key, &self.0.instance, admit)?
        {
            Some(receipt) => Ok(EffectAdmission::Retained(receipt)),
            None => Ok(EffectAdmission::Fresh(EffectTicket {
                journal: self.clone(),
                key,
                armed: true,
            })),
        }
    }
}

impl EffectTicket {
    /// Consumes the accepted ticket. Revalidate immediately before this call;
    /// after it succeeds, invoke the sink once with no intervening await.
    pub(crate) fn dispatch(
        mut self,
        admit: impl FnOnce(&mut dyn FnMut() -> Result<(), String>) -> Result<(), String>,
    ) -> Result<EffectDispatch, String> {
        self.journal
            .0
            .db
            .dispatch_external_effect(&self.key, &self.journal.0.instance, admit)?;
        self.armed = false;
        Ok(EffectDispatch {
            journal: self.journal.clone(),
            key: self.key.clone(),
            armed: true,
        })
    }

    /// An accepted effect canceled before handoff is a definite failure. Its
    /// immutable identity remains consumed and cannot be reserved again.
    #[cfg(test)]
    pub(crate) fn cancel(mut self) -> Result<EffectReceipt, String> {
        let receipt = self.journal.0.db.finish_external_effect(
            &self.key,
            &self.journal.0.instance,
            false,
            EffectOutcome::Failed,
        )?;
        self.armed = false;
        Ok(receipt)
    }
}

impl EffectDispatch {
    pub(crate) fn complete(mut self, outcome: EffectOutcome) -> Result<EffectReceipt, String> {
        let receipt = self.journal.0.db.finish_external_effect(
            &self.key,
            &self.journal.0.instance,
            true,
            outcome,
        )?;
        self.armed = false;
        Ok(receipt)
    }
}

impl Drop for EffectTicket {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.journal.0.db.finish_external_effect(
                &self.key,
                &self.journal.0.instance,
                false,
                EffectOutcome::Failed,
            );
        }
    }
}

impl Drop for EffectDispatch {
    fn drop(&mut self) {
        if self.armed {
            // A canceled future cannot prove that its external sink did not
            // act. Never classify that ambiguity as a retryable failure.
            let _ = self.journal.0.db.finish_external_effect(
                &self.key,
                &self.journal.0.instance,
                true,
                EffectOutcome::Unknown,
            );
        }
    }
}

#[cfg(test)]
mod tests;

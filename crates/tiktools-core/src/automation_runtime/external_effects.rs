//! Durable handoff while the exact saved approval and current subjects are held.
use super::context::ExecutionContext;
use crate::{
    collaboration::ExecutionIdentity,
    control::{EffectKind, EffectReceipt},
    services::effects::{EffectAdmission, EffectDispatch},
    AppCore, Value,
};

pub(super) enum ExternalStart {
    Dispatched(EffectDispatch),
    Retained(EffectReceipt),
}

impl AppCore {
    /// Keep SQLite admission/fsync off the LIVE executor. Cancelling this wait
    /// never transfers its result to a sink: the owned dispatch token is
    /// dropped as Unknown when the blocking task's result is abandoned.
    pub(super) async fn start_external_effect_owned(
        self: &std::sync::Arc<Self>,
        action: Value,
        context: ExecutionContext,
        kind: EffectKind,
        request: Value,
        check: impl Fn(&[ExecutionIdentity]) -> Result<(), String> + Send + 'static,
    ) -> Result<ExternalStart, String> {
        let core = self.clone();
        tokio::task::spawn_blocking(move || {
            core.start_external_effect(&action, &context, kind, &request, check)
        })
        .await
        .map_err(|_| "external effect admission is unavailable".to_owned())?
    }
    pub(super) fn start_external_effect(
        &self,
        action: &Value,
        context: &ExecutionContext,
        kind: EffectKind,
        request: &Value,
        check: impl Fn(&[ExecutionIdentity]) -> Result<(), String>,
    ) -> Result<ExternalStart, String> {
        // All automation writers take this gate before their business SQLite
        // transaction. Keep that order here as well; never acquire it from the
        // journal's commit callback after opening a transaction.
        let _writer = self.automation_gate().map_err(|error| error.to_string())?;
        let journal = self.effects.as_ref().map_err(Clone::clone)?;
        let binding = crate::collaboration::hash_secret(&action.to_string());
        let request = crate::collaboration::hash_secret(&request.to_string());
        let key = context
            .execution()?
            .key(kind, &binding, &request)
            .map_err(|error| error.to_string())?;
        let check_current =
            |identities: &[ExecutionIdentity], commit: &mut dyn FnMut() -> Result<(), String>| {
                if self.is_shutdown() {
                    return Err("host is shutting down".into());
                }
                Self::check_core_subjects(action, identities)?;
                check(identities)?;
                for identity in identities {
                    identity
                        .check_deadline()
                        .map_err(|_| "saved execution subject has expired".to_owned())?;
                }
                commit()
            };
        // Preflight retained receipts under the same live authority. The
        // journal repeats this under its own writer before each commit.
        context.with_action_admission(self, action, |identities| {
            check_current(identities, &mut || Ok(()))
        })?;
        let admit = |commit: &mut dyn FnMut() -> Result<(), String>| {
            if context.is_competition() {
                context.with_competition_journal_admission(self, action, |identities| {
                    check_current(identities, commit)
                })
            } else {
                context.with_current_subjects(action, &self.automation, |identities| {
                    check_current(identities, commit)
                })
            }
        };
        match journal.reserve(key, admit)? {
            EffectAdmission::Retained(receipt) => Ok(ExternalStart::Retained(receipt)),
            EffectAdmission::Fresh(ticket) => ticket.dispatch(admit).map(ExternalStart::Dispatched),
        }
    }
}

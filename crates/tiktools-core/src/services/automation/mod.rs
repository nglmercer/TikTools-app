//! Automation orchestration: snapshot index, trigger matching, and run history.
//!
//! [`AutomationService`] stays the single orchestration boundary; this module
//! only groups its implementation by responsibility: [`snapshot`](self) owns the
//! behavior snapshot index and plugin trigger ownership, [`matching`](self)
//! trigger matching and action lookup, [`cooldown`](self) per-event cooldown
//! windows, [`filters`](self) the filter predicate language, and [`runs`](self)
//! the capped run history.

#[cfg(feature = "persistence")]
pub(crate) mod approval;
mod authority;
mod cooldown;
pub(crate) use authority::{RuntimeAuthorities, RuntimeAuthority};
mod filters;
#[cfg(feature = "persistence")]
pub(crate) mod manifest;
mod matching;
pub(crate) mod moderation;
mod runs;
mod snapshot;
#[cfg(test)]
mod tests;

pub(crate) use filters::read_event_path;

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, RwLock,
    },
};

use serde_json::Value;
use tiktools_plugin_api::sync::{recover_mutex, recover_rwlock_read, recover_rwlock_write};

use super::ScriptService;

/// Automation orchestration boundary. The workflow persistence and event
/// routing stay in `AppCore`; script evaluation is isolated here so it can be
/// tested without a desktop object or a plugin runtime.
pub struct AutomationService {
    script: ScriptService,
    index: RwLock<BehaviorIndex>,
    cooldowns: Mutex<BTreeMap<String, u64>>,
    runs: Mutex<runs::RunHistory>,
    sequence: AtomicU64,
}

impl Default for AutomationService {
    fn default() -> Self {
        Self {
            script: ScriptService,
            index: RwLock::new(BehaviorIndex::default()),
            cooldowns: Mutex::new(BTreeMap::new()),
            runs: Mutex::new(runs::RunHistory::default()),
            sequence: AtomicU64::new(0),
        }
    }
}

#[derive(Clone)]
struct IndexedRecord {
    definition: Value,
    persisted_revision: Option<String>,
    generation: Option<u64>,
}
struct BehaviorIndex {
    #[cfg(feature = "persistence")]
    identity: std::sync::Arc<()>,
    actions: BTreeMap<String, IndexedRecord>,
    events: BTreeMap<String, IndexedRecord>,
    trigger_owners: BTreeMap<String, String>,
    enabled_plugins: std::collections::BTreeSet<String>,
    sequence: u64,
    catalog_epoch: Option<u64>,
    authorities: Option<RuntimeAuthorities>,
    authority_available: bool,
    #[cfg(feature = "persistence")]
    approvals: BTreeMap<(String, String), crate::db::BehaviorApprovalSnapshot>,
    #[cfg(feature = "persistence")]
    retired_approvals: BTreeMap<(String, String), u64>,
}
impl Default for BehaviorIndex {
    fn default() -> Self {
        Self {
            #[cfg(feature = "persistence")]
            identity: std::sync::Arc::new(()),
            actions: BTreeMap::new(),
            events: BTreeMap::new(),
            trigger_owners: BTreeMap::new(),
            enabled_plugins: Default::default(),
            sequence: 0,
            catalog_epoch: Some(0),
            authorities: None,
            authority_available: true,
            #[cfg(feature = "persistence")]
            approvals: BTreeMap::new(),
            #[cfg(feature = "persistence")]
            retired_approvals: BTreeMap::new(),
        }
    }
}
impl BehaviorIndex {
    fn next(&mut self) -> Option<u64> {
        let next = self.sequence.checked_add(1)?;
        self.sequence = next;
        Some(next)
    }
}
#[derive(Clone)]
pub(crate) struct MatchedAction {
    pub definition: Value,
    generation: u64,
}
pub(crate) struct MatchedRecord {
    pub definition: Value,
    generation: Option<u64>,
    catalog_epoch: u64,
    pub actions: Vec<MatchedAction>,
    #[cfg(feature = "persistence")]
    approval: Option<approval::ApprovalCandidate>,
}

impl MatchedRecord {
    #[cfg(feature = "persistence")]
    pub(crate) fn approval_candidate(&self) -> Option<approval::ApprovalCandidate> {
        self.approval.clone()
    }
}

impl AutomationService {
    pub fn evaluate_script(
        &self,
        source: &str,
        event: &Value,
        inputs: &Value,
    ) -> Result<Value, String> {
        self.script.evaluate(source, event, inputs)
    }

    pub fn validate_script(&self, source: &str) -> Result<(), String> {
        self.script.validate(source)
    }

    /// Synchronous, thread-confined evaluator for an already approved saved
    /// script. Call from an owned blocking task, without any host leases held.
    #[cfg(feature = "persistence")]
    pub(crate) fn evaluate_scoped_script(
        &self,
        source: &str,
        event: &Value,
        cancelled: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<Value, String> {
        self.script.evaluate_scoped(source, event, cancelled)
    }
}

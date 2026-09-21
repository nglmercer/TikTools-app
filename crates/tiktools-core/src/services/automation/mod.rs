//! Automation orchestration: snapshot index, trigger matching, and run history.
//!
//! [`AutomationService`] stays the single orchestration boundary; this module
//! only groups its implementation by responsibility: [`snapshot`](self) owns the
//! behavior snapshot index and plugin trigger ownership, [`matching`](self)
//! trigger matching and action lookup, [`cooldown`](self) per-event cooldown
//! windows, [`filters`](self) the filter predicate language, and [`runs`](self)
//! the capped run history.

mod cooldown;
mod filters;
mod matching;
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
    actions: RwLock<BTreeMap<String, Value>>,
    events: RwLock<BTreeMap<String, Value>>,
    trigger_owners: RwLock<BTreeMap<String, String>>,
    enabled_plugins: RwLock<std::collections::BTreeSet<String>>,
    cooldowns: Mutex<BTreeMap<String, u64>>,
    runs: Mutex<Vec<Value>>,
    sequence: AtomicU64,
}

impl Default for AutomationService {
    fn default() -> Self {
        Self {
            script: ScriptService,
            actions: RwLock::new(BTreeMap::new()),
            events: RwLock::new(BTreeMap::new()),
            trigger_owners: RwLock::new(BTreeMap::new()),
            enabled_plugins: RwLock::new(std::collections::BTreeSet::new()),
            cooldowns: Mutex::new(BTreeMap::new()),
            runs: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
        }
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
}

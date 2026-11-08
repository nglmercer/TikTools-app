//! Capped automation run history and run id sequence.

use super::*;

#[derive(Default)]
pub(super) struct RunHistory {
    entries: Vec<TrackedRun>,
}
struct TrackedRun {
    value: Value,
    restricted: bool,
}
impl RunHistory {
    fn insert(&mut self, value: Value, restricted: bool) {
        let inherited = value.get("id").and_then(Value::as_str).is_some_and(|id| {
            self.entries.iter().any(|entry| {
                entry.restricted && entry.value.get("id").and_then(Value::as_str) == Some(id)
            })
        });
        self.entries.insert(
            0,
            TrackedRun {
                value,
                restricted: restricted || inherited,
            },
        );
        self.entries.truncate(60);
    }
    fn values(&self) -> Vec<Value> {
        self.entries
            .iter()
            .map(|entry| entry.value.clone())
            .collect()
    }
    fn restricted(&self) -> bool {
        self.entries.iter().any(|entry| entry.restricted)
    }
}

impl AutomationService {
    pub fn next_run_id(&self, prefix: &str, at: u64) -> String {
        format!(
            "{}-{}-{}",
            prefix,
            self.sequence.fetch_add(1, Ordering::AcqRel) + 1,
            at
        )
    }

    pub fn record_run(&self, run: Value) -> Vec<Value> {
        let mut runs = recover_mutex(&self.runs, "automation runs");
        runs.insert(run, false);
        runs.values()
    }

    pub fn record_run_only(&self, run: Value) {
        self.record_run_only_scoped(run, false);
    }

    /// Scope comes only from a trusted execution context, never JSON fields.
    pub(crate) fn record_run_only_scoped(&self, run: Value, restricted: bool) {
        recover_mutex(&self.runs, "automation runs").insert(run, restricted);
    }

    pub fn recent_runs(&self) -> Vec<Value> {
        recover_mutex(&self.runs, "automation runs").values()
    }

    /// Keep the snapshot and its observer restriction atomic. A later Owner
    /// completion cannot publish earlier restricted runs to plugin observers.
    pub(crate) fn recent_runs_scoped(&self) -> (Vec<Value>, bool) {
        let runs = recover_mutex(&self.runs, "automation runs");
        (runs.values(), runs.restricted())
    }

    #[cfg(test)]
    pub(crate) fn has_restricted_run_history(&self) -> bool {
        recover_mutex(&self.runs, "automation runs").restricted()
    }

    pub fn clear_runs(&self) {
        recover_mutex(&self.runs, "automation runs").entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scoped_history_survives_later_owner_records_and_json_claims() {
        let service = AutomationService::default();
        service.record_run_only_scoped(json!({"id":"scoped","authority":"owner"}), true);
        service.record_run_only(json!({"id":"owner"}));
        let (values, restricted) = service.recent_runs_scoped();
        assert!(restricted);
        assert_eq!(values.len(), 2);
        // Reuse of an ID cannot clear its trusted restriction.
        service.record_run_only(json!({"id":"scoped","restricted":false}));
        for i in 0..58 {
            service.record_run_only(json!({"id":format!("owner-{i}")}));
        }
        assert!(service.has_restricted_run_history());
        service.record_run_only(json!({"id":"last-owner"}));
        assert!(service.has_restricted_run_history());
        service.record_run_only(json!({"id":"evicts-last-scoped"}));
        assert!(!service.has_restricted_run_history());
        service.clear_runs();
        assert_eq!(service.recent_runs_scoped(), (vec![], false));
    }
}

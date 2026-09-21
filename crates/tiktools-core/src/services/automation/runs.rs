//! Capped automation run history and run id sequence.

use super::*;

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
        runs.insert(0, run);
        runs.truncate(60);
        runs.clone()
    }

    pub fn recent_runs(&self) -> Vec<Value> {
        recover_mutex(&self.runs, "automation runs").clone()
    }

    pub fn clear_runs(&self) {
        recover_mutex(&self.runs, "automation runs").clear();
    }
}

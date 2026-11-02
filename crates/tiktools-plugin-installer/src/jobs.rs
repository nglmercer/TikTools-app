use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{layout::now, InstallError};

const MAX_JOBS: usize = 50;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum JobKind {
    Install,
    Update,
    Uninstall,
    Rollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum JobStage {
    Queued,
    Resolve,
    Download,
    Verify,
    Extract,
    Activate,
    Cleanup,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub done: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    pub kind: JobKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_version: Option<String>,
    pub stage: JobStage,
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<JobProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<InstallError>,
    pub created_at: u64,
    pub updated_at: u64,
}

pub trait JobReporter: Send + Sync {
    fn stage(&self, stage: JobStage);
    fn progress(&self, done: u64, total: Option<u64>);
    fn is_cancelled(&self) -> bool;

    fn check(&self) -> Result<(), InstallError> {
        if self.is_cancelled() {
            Err(InstallError::cancelled())
        } else {
            Ok(())
        }
    }
}

pub struct NoopReporter;

impl JobReporter for NoopReporter {
    fn stage(&self, _stage: JobStage) {}
    fn progress(&self, _done: u64, _total: Option<u64>) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}

pub type JobListener = Arc<dyn Fn(&JobSnapshot) + Send + Sync>;

struct JobEntry {
    snapshot: JobSnapshot,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct JobState {
    jobs: VecDeque<JobEntry>,
    next: u64,
}

#[derive(Clone, Default)]
pub struct JobRegistry {
    state: Arc<Mutex<JobState>>,
    listener: Option<JobListener>,
}

impl JobRegistry {
    pub fn new(listener: Option<JobListener>) -> Self {
        Self {
            state: Arc::default(),
            listener,
        }
    }

    pub fn start(
        &self,
        kind: JobKind,
        plugin_id: Option<String>,
        spec: Option<String>,
    ) -> JobHandle {
        let cancel = Arc::new(AtomicBool::new(false));
        let snapshot = {
            let mut state = self.lock();
            state.next += 1;
            let timestamp = now();
            let snapshot = JobSnapshot {
                id: format!("job-{}-{}", timestamp, state.next),
                kind,
                plugin_id,
                spec,
                from_version: None,
                to_version: None,
                stage: JobStage::Queued,
                status: JobStatus::Queued,
                progress: None,
                error: None,
                created_at: timestamp,
                updated_at: timestamp,
            };
            state.jobs.push_back(JobEntry {
                snapshot: snapshot.clone(),
                cancel: cancel.clone(),
            });
            while state.jobs.len() > MAX_JOBS {
                let Some(position) = state
                    .jobs
                    .iter()
                    .position(|entry| entry.snapshot.status.is_finished())
                else {
                    break;
                };
                state.jobs.remove(position);
            }
            snapshot
        };
        self.emit(&snapshot);
        JobHandle {
            id: snapshot.id,
            registry: self.clone(),
            cancel,
            last_progress: Mutex::new(None),
        }
    }

    pub fn list(&self) -> Vec<JobSnapshot> {
        self.lock()
            .jobs
            .iter()
            .rev()
            .map(|entry| entry.snapshot.clone())
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<JobSnapshot> {
        self.lock()
            .jobs
            .iter()
            .find(|entry| entry.snapshot.id == id)
            .map(|entry| entry.snapshot.clone())
    }

    pub fn cancel(&self, id: &str) -> bool {
        self.lock()
            .jobs
            .iter()
            .find(|entry| entry.snapshot.id == id && !entry.snapshot.status.is_finished())
            .map(|entry| entry.cancel.store(true, Ordering::Release))
            .is_some()
    }

    pub fn active_for(&self, plugin_id: &str) -> Option<JobSnapshot> {
        self.lock()
            .jobs
            .iter()
            .find(|entry| {
                entry.snapshot.plugin_id.as_deref() == Some(plugin_id)
                    && !entry.snapshot.status.is_finished()
            })
            .map(|entry| entry.snapshot.clone())
    }

    fn update(&self, id: &str, change: impl FnOnce(&mut JobSnapshot)) -> Option<JobSnapshot> {
        let snapshot = {
            let mut state = self.lock();
            let entry = state
                .jobs
                .iter_mut()
                .find(|entry| entry.snapshot.id == id)?;
            change(&mut entry.snapshot);
            entry.snapshot.updated_at = now();
            entry.snapshot.clone()
        };
        self.emit(&snapshot);
        Some(snapshot)
    }

    fn emit(&self, snapshot: &JobSnapshot) {
        if let Some(listener) = &self.listener {
            listener(snapshot);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, JobState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct JobHandle {
    id: String,
    registry: JobRegistry,
    cancel: Arc<AtomicBool>,
    last_progress: Mutex<Option<Instant>>,
}

impl JobHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub fn snapshot(&self) -> Option<JobSnapshot> {
        self.registry.get(&self.id)
    }

    pub fn describe(&self, plugin_id: Option<&str>, from: Option<&str>, to: Option<&str>) {
        self.registry.update(&self.id, |snapshot| {
            if let Some(plugin_id) = plugin_id {
                snapshot.plugin_id = Some(plugin_id.to_owned());
            }
            snapshot.from_version = from.map(ToOwned::to_owned).or(snapshot.from_version.take());
            if let Some(to) = to {
                snapshot.to_version = Some(to.to_owned());
            }
        });
    }

    pub fn finish<T>(&self, result: &Result<T, InstallError>) -> Option<JobSnapshot> {
        self.registry.update(&self.id, |snapshot| match result {
            Ok(_) => {
                snapshot.status = JobStatus::Succeeded;
                snapshot.stage = JobStage::Done;
                snapshot.error = None;
            }
            Err(error) => {
                snapshot.status = if error.code == crate::InstallErrorCode::Cancelled {
                    JobStatus::Cancelled
                } else {
                    JobStatus::Failed
                };
                snapshot.error = Some(error.clone());
            }
        })
    }
}

impl JobReporter for JobHandle {
    fn stage(&self, stage: JobStage) {
        self.registry.update(&self.id, |snapshot| {
            snapshot.stage = stage;
            snapshot.status = JobStatus::Running;
            snapshot.progress = None;
        });
    }

    fn progress(&self, done: u64, total: Option<u64>) {
        let finished = total.is_some_and(|total| done >= total);
        {
            let mut last = self
                .last_progress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !finished && last.is_some_and(|instant| instant.elapsed() < PROGRESS_INTERVAL) {
                return;
            }
            *last = Some(Instant::now());
        }
        self.registry.update(&self.id, |snapshot| {
            snapshot.progress = Some(JobProgress { done, total });
        });
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
}

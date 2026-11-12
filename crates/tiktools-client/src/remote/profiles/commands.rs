//! Metadata-only client command journal. Pending IDs are never pruned/replayed.
//! All filesystem methods run on the blocking pool; snapshots hold no I/O lock.
mod documents;
mod files;
mod mutations;
mod opening;
mod operations;
mod validation;

#[cfg(test)]
use files::options;

use super::{directory, *};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, RwLock,
    },
};

const BYTES: u64 = 64 * 1024;
const HISTORY: usize = 16;
const VERSION: u32 = 9;
pub fn timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(9_007_199_254_740_991) as i64
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub profile_id: String,
    pub origin: String,
    pub host_id: String,
    pub principal_id: String,
    pub session_id: String,
}
impl From<&RemoteProfile> for Binding {
    fn from(p: &RemoteProfile) -> Self {
        Self {
            profile_id: p.profile_id.clone(),
            origin: p.origin.clone(),
            host_id: p.host_id.clone(),
            principal_id: p.principal_id.clone(),
            session_id: p.session_id.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    Disable,
    Delete,
    AdjustPoints,
    DisableEvent,
    DisableAction,
    EnableEvent,
    EnableAction,
    DeleteEvent,
    CreateEventDraft,
    UpdateEventDraft,
    CreateActionDraft,
    UpdateActionDraft,
    CreateEventActive,
    UpdateEventActive,
    CreateActionActive,
    UpdateActionActive,
    StartCompetition,
    ResumeCompetition,
    PauseCompetition,
    CancelCompetition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Intent {
    pub command_id: String,
    pub operation: Operation,
    pub resource_id: String,
    pub host_instance_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_generation: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Succeeded,
    Rejected,
    ReviewedUnknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub intent: Intent,
    pub outcome: Outcome,
    pub revision: Option<String>,
    pub error_code: Option<String>,
    pub recorded_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points_result: Option<PointsResult>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointsResult {
    pub delta: f64,
    pub total_points: f64,
    pub level: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JournalView {
    pub binding: Binding,
    pub pending: Option<Intent>,
    pub recent: Vec<Record>,
    pub unavailable: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    version: u32,
    binding: Binding,
    pending: Option<Intent>,
    recent: Vec<Record>,
}
pub struct Journal {
    directory: PathBuf,
    path: PathBuf,
    _lock: File,
    data: RwLock<Data>,
    writing: Mutex<()>,
    failed: AtomicBool,
}
fn invalid() -> ClientError {
    fail(
        "command_journal_invalid",
        "command journal is invalid; no command was sent",
    )
}
fn unavailable() -> ClientError {
    fail(
        "command_journal_unavailable",
        "command journal could not be persisted; review after reopening the profile",
    )
}

impl Journal {
    #[cfg(test)]
    pub(crate) fn hold_writer_for_test(
        &self,
        entered: std::sync::mpsc::Sender<()>,
        released: std::sync::mpsc::Receiver<()>,
    ) {
        let _guard = self.writing.lock().unwrap();
        entered.send(()).unwrap();
        released
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
    }

    pub fn matches(&self, profile: &RemoteProfile) -> bool {
        self.view().binding == Binding::from(profile)
    }

    pub fn view(&self) -> JournalView {
        let d = self.data.read().unwrap_or_else(|p| p.into_inner());
        JournalView {
            binding: d.binding.clone(),
            pending: d.pending.clone(),
            recent: d.recent.clone(),
            unavailable: self.failed.load(Ordering::Acquire),
        }
    }
}

#[cfg(test)]
fn migration_stage(stage: &str) {
    if std::env::var("TIKTOOLS_JOURNAL_MIGRATION_CRASH").as_deref() == Ok(stage) {
        std::process::exit(87);
    }
}

#[cfg(test)]
mod tests;

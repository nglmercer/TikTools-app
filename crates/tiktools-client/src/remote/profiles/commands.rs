//! Metadata-only client command journal. Pending IDs are never pruned/replayed.
//! All filesystem methods run on the blocking pool; snapshots hold no I/O lock.
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
const VERSION: u32 = 8;
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
impl Operation {
    pub fn method(self) -> &'static str {
        match self {
            Self::Disable => "workflows.disable",
            Self::Delete => "workflows.delete",
            Self::AdjustPoints => "points.adjust",
            Self::StartCompetition => "competitions.start",
            Self::ResumeCompetition => "competitions.resume",
            Self::PauseCompetition => "competitions.pause",
            Self::CancelCompetition => "competitions.cancel",
            Self::DisableEvent => "automation.disable",
            Self::EnableEvent | Self::EnableAction => "automation.enable",
            Self::DeleteEvent => "automation.delete",
            Self::CreateEventDraft
            | Self::CreateActionDraft
            | Self::CreateEventActive
            | Self::CreateActionActive => "automation.create",
            Self::UpdateEventDraft
            | Self::UpdateActionDraft
            | Self::UpdateEventActive
            | Self::UpdateActionActive => "automation.update",
        }
    }
    pub fn feature(self) -> &'static str {
        match self {
            Self::Disable => "workflowDisable",
            Self::Delete => "workflowDelete",
            Self::AdjustPoints => "pointsAdjust",
            Self::StartCompetition | Self::ResumeCompetition => "competitionActivation",
            Self::PauseCompetition | Self::CancelCompetition => "competitionStop",
            Self::DisableEvent | Self::DeleteEvent => "automationEventDeactivate",
            Self::EnableEvent | Self::EnableAction => "automationReusableActivation",
            Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft => "automationDrafts",
            Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => "automationReusableWrite",
        }
    }
    pub fn capability(self) -> tiktools_collaboration::Capability {
        match self {
            Self::Disable => tiktools_collaboration::Capability::WorkflowsToggle,
            Self::Delete => tiktools_collaboration::Capability::WorkflowsEdit,
            Self::AdjustPoints => tiktools_collaboration::Capability::PointsAdjust,
            Self::StartCompetition
            | Self::ResumeCompetition
            | Self::PauseCompetition
            | Self::CancelCompetition => tiktools_collaboration::Capability::CompetitionsManage,
            Self::DisableEvent | Self::EnableEvent | Self::EnableAction => {
                tiktools_collaboration::Capability::AutomationToggle
            }
            Self::DeleteEvent => tiktools_collaboration::Capability::AutomationEdit,
            Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft
            | Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => tiktools_collaboration::Capability::AutomationEdit,
        }
    }
    pub fn kind(self) -> tiktools_collaboration::ResourceKind {
        match self {
            Self::AdjustPoints => tiktools_collaboration::ResourceKind::Viewer,
            Self::StartCompetition
            | Self::ResumeCompetition
            | Self::PauseCompetition
            | Self::CancelCompetition => tiktools_collaboration::ResourceKind::Competition,
            Self::DisableEvent
            | Self::EnableEvent
            | Self::EnableAction
            | Self::DeleteEvent
            | Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft
            | Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => tiktools_collaboration::ResourceKind::Automation,
            Self::Disable | Self::Delete => tiktools_collaboration::ResourceKind::Workflow,
        }
    }
    pub fn scope(self) -> &'static str {
        if matches!(
            self,
            Self::EnableAction
                | Self::CreateActionDraft
                | Self::UpdateActionDraft
                | Self::CreateActionActive
                | Self::UpdateActionActive
        ) {
            "action"
        } else if self.is_event()
            || matches!(
                self,
                Self::CreateEventDraft
                    | Self::UpdateEventDraft
                    | Self::CreateEventActive
                    | Self::UpdateEventActive
            )
        {
            "event"
        } else {
            "host"
        }
    }
    pub fn is_event(self) -> bool {
        matches!(
            self,
            Self::DisableEvent | Self::DeleteEvent | Self::EnableEvent
        )
    }
    pub fn is_competition(self) -> bool {
        matches!(
            self,
            Self::StartCompetition
                | Self::ResumeCompetition
                | Self::PauseCompetition
                | Self::CancelCompetition
        )
    }
    pub fn is_activation(self) -> bool {
        matches!(self, Self::EnableEvent | Self::EnableAction)
    }
    pub fn is_automation_command(self) -> bool {
        self.is_event() || self.is_activation()
    }
    pub fn is_delete(self) -> bool {
        matches!(self, Self::Delete | Self::DeleteEvent)
    }
    pub fn is_proposal(self) -> bool {
        matches!(
            self,
            Self::CreateEventDraft
                | Self::UpdateEventDraft
                | Self::CreateActionDraft
                | Self::UpdateActionDraft
                | Self::CreateEventActive
                | Self::UpdateEventActive
                | Self::CreateActionActive
                | Self::UpdateActionActive
        )
    }
    pub fn is_active_proposal(self) -> bool {
        matches!(
            self,
            Self::CreateEventActive
                | Self::UpdateEventActive
                | Self::CreateActionActive
                | Self::UpdateActionActive
        )
    }
    pub fn proposal_family(self) -> Option<tiktools_collaboration::AutomationFamily> {
        if self.is_proposal() {
            tiktools_collaboration::AutomationFamily::from_scope(self.scope())
        } else {
            None
        }
    }
    fn supported_in(self, version: u32) -> bool {
        match self {
            Self::Disable | Self::Delete => matches!(version, 1..=VERSION),
            Self::AdjustPoints => matches!(version, 2..=VERSION),
            Self::DisableEvent | Self::DeleteEvent => matches!(version, 3..=VERSION),
            Self::EnableEvent | Self::EnableAction => matches!(version, 5..=VERSION),
            Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft => matches!(version, 4..=VERSION),
            Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => matches!(version, 6..=VERSION),
            Self::PauseCompetition | Self::CancelCompetition => matches!(version, 7..=VERSION),
            Self::StartCompetition | Self::ResumeCompetition => matches!(version, 8..=VERSION),
        }
    }
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
impl Intent {
    pub fn validate(&self) -> Result<(), ClientError> {
        if self.operation.is_competition() {
            if self
                .expected_generation
                .as_deref()
                .and_then(|s| tiktools_collaboration::stream_sequence(s).ok())
                .is_none_or(|n| n > i64::MAX as u64)
                || self
                    .expected_revision
                    .as_deref()
                    .and_then(|s| tiktools_collaboration::stream_sequence(s).ok())
                    .is_none_or(|n| n == 0 || n > i64::MAX as u64)
            {
                return Err(invalid());
            }
        } else if self.expected_generation.is_some() {
            return Err(invalid());
        }
        if self.operation.is_proposal() {
            if self
                .proposal_revision
                .as_deref()
                .and_then(|n| tiktools_collaboration::stream_sequence(n).ok())
                .is_none_or(|n| n == 0 || n > i64::MAX as u64)
                || self
                    .expected_revision
                    .as_deref()
                    .and_then(|n| tiktools_collaboration::stream_sequence(n).ok())
                    .is_none_or(|n| {
                        n > i64::MAX as u64
                            || n == 0
                                && matches!(
                                    self.operation,
                                    Operation::UpdateEventDraft
                                        | Operation::UpdateActionDraft
                                        | Operation::UpdateEventActive
                                        | Operation::UpdateActionActive
                                )
                    })
            {
                return Err(invalid());
            }
        } else if self.proposal_revision.is_some() {
            return Err(invalid());
        }
        if !valid_id(&self.command_id)
            || !valid_id(&self.host_instance_id)
            || self.resource_id.is_empty()
            || self.resource_id.len() > 128
            || self.resource_id.trim() != self.resource_id
            || self.resource_id.contains('*')
            || self.resource_id.chars().any(char::is_control)
            || self.created_at < 0
            || self.created_at > 9_007_199_254_740_991
        {
            return Err(invalid());
        }
        match self.operation {
            Operation::AdjustPoints => {
                if self.expected_revision.is_some()
                    || self.resource_id.starts_with('@')
                    || !self
                        .delta
                        .is_some_and(|n| n.is_finite() && n != 0.0 && n.abs() <= 1_000_000_000.0)
                    || !self.currency_name.as_deref().is_some_and(|s| {
                        !s.is_empty()
                            && s.trim() == s
                            && s.chars().count() <= 64
                            && !s.chars().any(char::is_control)
                    })
                {
                    return Err(invalid());
                }
            }
            _ => {
                if self.delta.is_some()
                    || self.currency_name.is_some()
                    || self
                        .expected_revision
                        .as_deref()
                        .is_none_or(|s| tiktools_collaboration::stream_sequence(s).is_err())
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
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
impl PointsResult {
    pub(crate) fn valid_for(&self, requested: f64) -> bool {
        let rounded = |n: f64| (n * 100.0).round() / 100.0;
        self.delta.is_finite()
            && self.total_points.is_finite()
            && self.total_points >= 0.0
            && self.level > 0
            && if requested < 0.0 {
                self.delta == rounded(requested)
            } else {
                // The host bounds its positive subscriber bonus to 10,000%.
                self.delta >= rounded(requested)
                    && self.delta <= rounded(requested + requested * 100.0)
            }
    }
}
impl Record {
    fn validate(&self) -> Result<(), ClientError> {
        self.intent.validate()?;
        if !(0..=9_007_199_254_740_991).contains(&self.recorded_at) {
            return Err(invalid());
        }
        if (self.intent.operation != Operation::AdjustPoints || self.outcome != Outcome::Succeeded)
            && self.points_result.is_some()
        {
            return Err(invalid());
        }
        match self.outcome {
            Outcome::Succeeded => {
                if self.error_code.is_some() {
                    return Err(invalid());
                }
                let after = tiktools_collaboration::stream_sequence(
                    self.revision.as_deref().ok_or_else(invalid)?,
                )
                .map_err(|_| invalid())?;
                if self.intent.operation == Operation::AdjustPoints {
                    let result = self.points_result.as_ref().ok_or_else(invalid)?;
                    let requested = self.intent.delta.ok_or_else(invalid)?;
                    if after == 0 || !result.valid_for(requested) {
                        return Err(invalid());
                    }
                    return Ok(());
                }
                let before = tiktools_collaboration::stream_sequence(
                    self.intent
                        .expected_revision
                        .as_deref()
                        .ok_or_else(invalid)?,
                )
                .map_err(|_| invalid())?;
                if before.checked_add(1) != Some(after)
                    && !(self.intent.operation.is_delete() && after == before)
                {
                    return Err(invalid());
                }
            }
            Outcome::Rejected => {
                if self.error_code.as_deref() == Some("proposal_conflict")
                    && !self.intent.operation.is_proposal()
                {
                    return Err(invalid());
                }
                if (self.error_code.as_deref() == Some("automation_not_found")
                    && !self.intent.operation.is_automation_command()
                    && !self.intent.operation.is_proposal())
                    || self.revision.is_some()
                    || !matches!(
                        self.error_code.as_deref(),
                        Some(
                            "invalid_params"
                                | "currency_changed"
                                | "revision_conflict"
                                | "not_found"
                                | "automation_not_found"
                                | "overloaded"
                                | "stale_view"
                                | "not_ready"
                                | "forbidden"
                                | "proposal_conflict"
                        )
                    )
                {
                    return Err(invalid());
                }
            }
            Outcome::ReviewedUnknown => {
                if self.revision.is_some() || self.error_code.as_deref() != Some("command_unknown")
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
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
fn private_file(file: &File) -> Result<(), ClientError> {
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() {
        return Err(invalid());
    }
    directory::private(&metadata).map_err(|_| invalid())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(invalid());
        }
    }
    Ok(())
}
fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    options
}
fn read(path: &Path) -> Result<Option<Data>, ClientError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(unavailable()),
        Ok(m) if !m.is_file() => return Err(invalid()),
        _ => {}
    }
    let file = options().read(true).open(path).map_err(|_| invalid())?;
    private_file(&file)?;
    let mut bytes = Vec::new();
    file.take(BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() as u64 > BYTES {
        return Err(invalid());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .is_some_and(|v| v < 4)
        && (value
            .get("pending")
            .filter(|v| !v.is_null())
            .is_some_and(|i| i.get("proposalRevision").is_some())
            || value
                .get("recent")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|rs| {
                    rs.iter().any(|r| {
                        r.get("intent")
                            .is_some_and(|i| i.get("proposalRevision").is_some())
                    })
                }))
    {
        return Err(invalid());
    }
    if value.get("version").and_then(serde_json::Value::as_u64) == Some(1) {
        let old_intent = |i: &serde_json::Value| {
            i.get("delta").is_none()
                && i.get("currencyName").is_none()
                && i.get("expectedRevision")
                    .and_then(serde_json::Value::as_str)
                    .is_some()
                && matches!(
                    i.get("operation").and_then(serde_json::Value::as_str),
                    Some("disable" | "delete")
                )
        };
        if value
            .get("pending")
            .filter(|v| !v.is_null())
            .is_some_and(|i| !old_intent(i))
            || value
                .get("recent")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|rows| {
                    rows.iter().any(|r| {
                        r.get("pointsResult").is_some()
                            || r.get("intent").is_none_or(|i| !old_intent(i))
                    })
                })
        {
            return Err(invalid());
        }
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|_| invalid())
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
    /// Holds an independent process lease for the selected profile journal.
    pub fn open(root: &Path, profile: &RemoteProfile) -> Result<Self, ClientError> {
        profile.validate()?;
        directory::check_directory(root, false)?;
        let directory = root.join("commands");
        directory::check_directory(&directory, true)?;
        let lock_path = directory.join(format!("{}.lock", profile.profile_id));
        if fs::symlink_metadata(&lock_path).is_ok_and(|m| !m.is_file()) {
            return Err(invalid());
        }
        let lock = options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|_| invalid())?;
        private_file(&lock)?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            fail(
                "command_journal_in_use",
                "another controller is using this profile journal",
            )
        })?;
        let path = directory.join(format!("{}.journal", profile.profile_id));
        let binding = Binding::from(profile);
        let mut data = read(&path)?.unwrap_or_else(|| Data {
            version: VERSION,
            binding: binding.clone(),
            pending: None,
            recent: Vec::new(),
        });
        if !matches!(data.version, 1..=VERSION)
            || data.binding != binding
            || data.recent.len() > HISTORY
        {
            return Err(invalid());
        }
        if data
            .pending
            .iter()
            .chain(data.recent.iter().map(|r| &r.intent))
            .any(|i| !i.operation.supported_in(data.version))
        {
            return Err(invalid());
        }
        if let Some(intent) = &data.pending {
            intent.validate()?;
        }
        let mut ids = std::collections::BTreeSet::new();
        if let Some(intent) = &data.pending {
            ids.insert(&intent.command_id);
        }
        for record in &data.recent {
            record.validate()?;
            if !ids.insert(&record.intent.command_id) {
                return Err(invalid());
            }
        }
        let migrate = data.version != VERSION;
        data.version = VERSION;
        let journal = Self {
            directory,
            path,
            _lock: lock,
            data: RwLock::new(data),
            writing: Mutex::new(()),
            failed: AtomicBool::new(false),
        };
        if migrate {
            #[cfg(test)]
            migration_stage("before");
            journal.persist(&*journal.data.read().map_err(|_| unavailable())?)?;
            #[cfg(test)]
            migration_stage("after");
        }
        Ok(journal)
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
    fn persist(&self, data: &Data) -> Result<(), ClientError> {
        directory::check_directory(&self.directory, false).map_err(|_| unavailable())?;
        if fs::symlink_metadata(&self.path).is_ok() {
            let file = options()
                .read(true)
                .open(&self.path)
                .map_err(|_| invalid())?;
            private_file(&file)?;
        }
        let bytes = serde_json::to_vec(data).map_err(|_| invalid())?;
        if bytes.len() as u64 > BYTES {
            return Err(invalid());
        }
        let mut temp =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| unavailable())?;
        private_file(temp.as_file())?;
        temp.write_all(&bytes)
            .and_then(|_| temp.flush())
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|_| unavailable())?;
        temp.persist(&self.path).map_err(|_| unavailable())?;
        #[cfg(test)]
        migration_stage("renamed");
        directory::sync_directory(&self.directory).map_err(|_| unavailable())
    }
    pub fn begin(&self, intent: Intent) -> Result<(), ClientError> {
        intent.validate()?;
        let _writing = self.writing.lock().map_err(|_| unavailable())?;
        if self.failed.load(Ordering::Acquire) {
            return Err(unavailable());
        }
        let mut next = self.data.read().map_err(|_| unavailable())?.clone();
        if next.pending.is_some() {
            return Err(fail(
                "command_pending",
                "resolve or explicitly review the existing command first",
            ));
        }
        if next
            .recent
            .iter()
            .any(|r| r.intent.command_id == intent.command_id)
        {
            return Err(invalid());
        }
        next.pending = Some(intent);
        if let Err(error) = self.persist(&next) {
            // A rename may have succeeded before directory sync failed. Never
            // dispatch after a partial persistence failure, or silently replace it.
            *self.data.write().map_err(|_| unavailable())? = next;
            self.failed.store(true, Ordering::Release);
            return Err(error);
        }
        *self.data.write().map_err(|_| unavailable())? = next;
        Ok(())
    }
    pub fn finish(&self, record: Record) -> Result<(), ClientError> {
        record.validate()?;
        let _writing = self.writing.lock().map_err(|_| unavailable())?;
        if self.failed.load(Ordering::Acquire) {
            return Err(unavailable());
        }
        let mut next = self.data.read().map_err(|_| unavailable())?.clone();
        if next.pending.as_ref() != Some(&record.intent) {
            return Err(fail("stale_view", "pending command changed"));
        }
        next.pending = None;
        next.recent.insert(0, record);
        next.recent.truncate(HISTORY);
        if let Err(error) = self.persist(&next) {
            self.failed.store(true, Ordering::Release);
            return Err(error);
        }
        *self.data.write().map_err(|_| unavailable())? = next;
        Ok(())
    }
    pub fn remove_resolved(&self) -> Result<(), ClientError> {
        let _writing = self.writing.lock().map_err(|_| unavailable())?;
        if self.view().pending.is_some() || self.failed.load(Ordering::Acquire) {
            return Err(fail(
                "command_pending",
                "review the pending command before forgetting this profile",
            ));
        }
        super::drafts::remove(&self.path.with_extension("drafts"))?;
        super::proposals::remove(&self.path.with_extension("proposals"))?;
        match fs::remove_file(&self.path) {
            Ok(()) => directory::sync_directory(&self.directory).map_err(|_| unavailable()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(unavailable()),
        }
    }
    pub(in crate::remote) fn drafts(
        &self,
    ) -> Result<(String, Vec<super::drafts::Draft>), ClientError> {
        let _writing = self.writing.lock().map_err(|_| super::drafts::invalid())?;
        let data = super::drafts::read(&self.path.with_extension("drafts"), &self.view().binding)?;
        Ok((data.revision, data.records.into_values().collect()))
    }
    pub(in crate::remote) fn change_draft(
        &self,
        expected: &str,
        id: &str,
        draft: Option<super::drafts::Draft>,
    ) -> Result<String, ClientError> {
        let _writing = self.writing.lock().map_err(|_| super::drafts::invalid())?;
        super::drafts::change(
            &self.path.with_extension("drafts"),
            &self.view().binding,
            expected,
            id,
            draft,
        )
    }
    pub(in crate::remote) fn proposals(
        &self,
    ) -> Result<(String, Vec<super::proposals::ProposalDraft>), ClientError> {
        let _writing = self
            .writing
            .lock()
            .map_err(|_| super::proposals::invalid())?;
        let data =
            super::proposals::read(&self.path.with_extension("proposals"), &self.view().binding)?;
        Ok((data.revision, data.records.into_values().collect()))
    }
    pub(in crate::remote) fn change_proposal(
        &self,
        expected: &str,
        kind: tiktools_collaboration::AutomationFamily,
        id: &str,
        draft: Option<super::proposals::ProposalDraft>,
    ) -> Result<String, ClientError> {
        let _writing = self
            .writing
            .lock()
            .map_err(|_| super::proposals::invalid())?;
        super::proposals::change(
            &self.path.with_extension("proposals"),
            &self.view().binding,
            expected,
            kind,
            id,
            draft,
        )
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

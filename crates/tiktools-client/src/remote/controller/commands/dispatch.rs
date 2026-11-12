//! Serialized native command tasks, journal ownership and controller entry points.
use super::*;

async fn action(
    shared: Arc<Shared>,
    gate: Arc<Mutex<()>>,
    epoch: String,
    action: Action,
) -> Result<Value, ClientError> {
    let _guard = gate.try_lock_owned().map_err(|_| {
        fail(
            "command_pending",
            "a controller command is already being processed",
        )
    })?;
    selected(&shared, &epoch, false)?;
    if let Action::Mutate(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    if let Action::Points(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed_points(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    if let Action::Automation(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed_automation(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    if let Action::Competition(p) = &action {
        let selected = selected(&shared, &epoch, true)?;
        allowed_competition(&shared, p, selected.remote.as_ref().ok_or_else(network)?)?;
    }
    shared.change(&epoch, |v| {
        v.status.command_busy = true;
        v.status.command_error_code = None;
    });
    let result = async {
        match action {
            Action::Mutate(p) => mutate(&shared, &epoch, p).await,
            Action::Points(p) => mutate_points(&shared, &epoch, p).await,
            Action::Automation(p) => mutate_automation(&shared, &epoch, p).await,
            Action::Competition(p) => mutate_competition(&shared, &epoch, p).await,
            Action::Proposal(p) => proposals::mutate(&shared, p).await,
            Action::Resolve(id) => resolve(&shared, &epoch, id.as_deref()).await,
            Action::Review(id) => {
                let selected = selected(&shared, &epoch, false)?;
                let intent = selected
                    .journal
                    .view()
                    .pending
                    .ok_or_else(|| fail("stale_view", "pending command changed"))?;
                if intent.command_id != id {
                    Err(fail("stale_view", "pending command changed"))
                } else {
                    finish(
                        &selected.journal,
                        intent,
                        Outcome::ReviewedUnknown,
                        None,
                        Some("command_unknown".into()),
                    )
                    .await
                }
            }
        }
    }
    .await;
    points::dirty(&shared, &epoch);
    shared.change(&epoch, |v| {
        v.status.command_busy = false;
        v.status.command_error_code = result.as_ref().err().map(code);
    });
    selected(&shared, &epoch, false)?;
    result?;
    let view = shared.view.read().unwrap_or_else(|p| p.into_inner());
    if view.status.epoch != epoch {
        return Err(fail("stale_view", "controller selection changed"));
    }
    serde_json::to_value(journal_projection(&view).ok_or_else(protocol)?).map_err(|_| protocol())
}

pub(in crate::remote::controller) fn schedule_resolve(
    shared: Arc<Shared>,
    gate: Arc<Mutex<()>>,
    epoch: String,
) {
    if selected(&shared, &epoch, false).is_ok_and(|s| s.journal.view().pending.is_some()) {
        tokio::spawn(async move {
            let _ = action(shared, gate, epoch, Action::Resolve(None)).await;
        });
    }
}

impl Controller {
    pub(in crate::remote::controller) async fn proposal_command(
        &self,
        p: proposals::Submit,
    ) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Proposal(p)).await
    }
    pub async fn automation_command(&self, p: AutomationCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Automation(p)).await
    }
    pub async fn competition_command(&self, p: CompetitionCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Competition(p)).await
    }
    pub async fn points_command(&self, p: PointsCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Points(p)).await
    }
    pub(in crate::remote::controller) async fn open_journal(
        &self,
        profile: &profiles::RemoteProfile,
    ) -> Result<Arc<Journal>, ClientError> {
        if let Some(journal) = self
            .journals
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&profile.profile_id)
            .and_then(std::sync::Weak::upgrade)
        {
            return if journal.matches(profile) {
                Ok(journal)
            } else {
                Err(fail("command_journal_invalid", "profile identity changed"))
            };
        }
        let root = self.profiles.directory().to_path_buf();
        let p = profile.clone();
        let journal = Arc::new(
            tokio::task::spawn_blocking(move || Journal::open(&root, &p))
                .await
                .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??,
        );
        let mut cache = self.journals.lock().unwrap_or_else(|p| p.into_inner());
        cache.retain(|_, v| v.strong_count() > 0);
        cache.insert(profile.profile_id.clone(), Arc::downgrade(&journal));
        Ok(journal)
    }
    pub async fn workflow_command(&self, p: WorkflowCommand) -> Result<Value, ClientError> {
        let epoch = p.epoch.clone();
        self.command_task(epoch, Action::Mutate(p)).await
    }
    pub async fn resolve_command(&self, epoch: String, id: String) -> Result<Value, ClientError> {
        self.command_task(epoch, Action::Resolve(Some(id))).await
    }
    pub async fn review_command(&self, epoch: String, id: String) -> Result<Value, ClientError> {
        self.command_task(epoch, Action::Review(id)).await
    }
    async fn command_task(&self, epoch: String, operation: Action) -> Result<Value, ClientError> {
        let shared = self.shared.clone();
        let commands = self.commands.clone();
        // Dropping the WebView request does not cancel known-result persistence.
        tokio::spawn(async move { action(shared, commands, epoch, operation).await })
            .await
            .map_err(|_| {
                fail(
                    "command_unknown",
                    "controller command task ended; check the durable intent",
                )
            })?
    }
}

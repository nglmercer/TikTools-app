//! Serialized profile selection, connection bootstrap and session reset.
use super::*;

impl Controller {
    pub(super) fn reset(&self, lifecycle: &mut Lifecycle) -> String {
        if let Some(worker) = lifecycle.worker.take() {
            worker.abort();
        }
        lifecycle.pairing = None;
        let mut v = self.shared.view.write().unwrap_or_else(|p| p.into_inner());
        if let Some(remote) = v.remote.take() {
            remote.close();
        }
        let epoch = v
            .status
            .epoch
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_add(1)
            .to_string();
        v.rows.clear();
        v.points = points::State::default();
        v.automation = automations::State::default();
        v.competition_observation = None;
        v.journal = None;
        v.status = ControllerStatus {
            drafts_supported: true,
            proposals_supported: true,
            epoch: epoch.clone(),
            view_revision: "0".into(),
            points_revision: "0".into(),
            automation_revision: "0".into(),
            competition_revision: "0".into(),
            points_loading: true,
            points_error_code: None,
            phase: "idle".into(),
            profile: None,
            origin: None,
            hello: None,
            error_code: None,
            pairing_request_id: None,
            command_journal: None,
            command_busy: false,
            command_error_code: None,
        };
        self.shared.changed.send_modify(|n| *n = n.wrapping_add(1));
        epoch
    }

    pub async fn disconnect(&self) {
        let mut l = self.lifecycle.lock().await;
        self.reset(&mut l);
    }

    pub async fn connect(&self, profile: &str) -> Result<(), ClientError> {
        let mut l = self.lifecycle.lock().await;
        let epoch = self.reset(&mut l);
        self.shared.change(&epoch, |v| {
            v.status.phase = "connecting".into();
            v.status.profile = Some(profile.into());
        });
        let result = async {
            let metadata = self.profiles.get(profile).await?;
            let journal = self.open_journal(&metadata).await?;
            self.shared.change(&epoch, |v| {
                v.status.origin = Some(metadata.origin);
                v.journal = Some(journal);
            });
            let remote = self.profiles.open(profile).await?;
            // Keep a handle during bootstrap so disconnect/shutdown closes it.
            self.shared
                .change(&epoch, |v| v.remote = Some(remote.clone()));
            let recovery = points::stream(&remote).await?;
            self.shared.ready(
                &epoch,
                &remote,
                recovery.as_ref().and_then(|s| s.baseline()),
            )?;
            if let Err(e) = points::refresh(&self.shared, &epoch, &remote).await {
                points::failure(&self.shared, &epoch, &e);
            }
            let shared = self.shared.clone();
            let commands = self.commands.clone();
            commands::schedule_resolve(shared.clone(), commands.clone(), epoch.clone());
            l.worker = Some(tokio::spawn(async move {
                run(shared, commands, epoch, remote, recovery).await;
            }));
            Ok(())
        }
        .await;
        if let Err(error) = &result {
            let epoch = self.status().epoch;
            self.shared.change(&epoch, |v| {
                if let Some(r) = v.remote.take() {
                    r.close();
                }
                v.rows.clear();
                automations::clear(v);
            });
            self.shared.phase(&epoch, "failed", Some(error));
        }
        result
    }
}

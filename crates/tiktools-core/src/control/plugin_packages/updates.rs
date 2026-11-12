use super::*;

impl AppCore {
    pub async fn plugin_updates_check(self: &Arc<Self>) -> Result<UpdateReport, OperationError> {
        let packages = self.packages()?;
        let _check = packages
            .updates_gate
            .try_lock()
            .map_err(|_| OperationError::conflict("an update check is already in progress"))?;
        let receipts = packages
            .installer
            .receipts()
            .list()
            .map_err(OperationError::install)?;
        let mut report = UpdateReport {
            checked_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ..UpdateReport::default()
        };
        // Each result is derived from the same signed document used for its
        // live status, not a second registry read from another generation.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        for receipt in &receipts {
            let PackageOrigin::Registry { registry, .. } = &receipt.origin else {
                continue;
            };
            let lookup = bounded_update_query(
                deadline,
                registry_package_live(&packages, &receipt.id, Some(registry)),
            )
            .await;
            let (source, document, live) = match lookup {
                Some(Ok((source, document, live))) => (source, document, live),
                Some(Err(error)) => {
                    report.errors.push(format!("{}: {error}", receipt.id));
                    report.updates.push(status_only_update(receipt, registry));
                    continue;
                }
                None => {
                    report
                        .errors
                        .push(format!("{}: update check timed out", receipt.id));
                    report.updates.push(status_only_update(receipt, registry));
                    continue;
                }
            };
            let current = installed_live_status(receipt, &source, &document, &live);
            let update =
                update_for_document(receipt, registry, &document, packages.installer.host());
            if update.is_none() && current == LiveReleaseStatus::Approved {
                continue;
            }
            let mut update = update.unwrap_or_else(|| status_only_update(receipt, registry));
            update.current_status = current;
            update.latest_status = update
                .latest
                .as_ref()
                .and_then(|version| live.get(version))
                .map_or(LiveReleaseStatus::Unknown, |entry| entry.status);
            update.auto_applicable &= update.latest_status == LiveReleaseStatus::Approved;
            report.updates.push(update);
        }
        packages.store_update_report(report.clone());
        if let Ok(value) = serde_json::to_value(&report) {
            self.events
                .publish_domain(DomainEvent::PluginUpdates { report: value });
        }
        for update in report
            .updates
            .iter()
            .filter(|update| update.auto_applicable)
        {
            if let Err(error) = self.plugin_package_update(&update.id, update.latest.clone(), None)
            {
                tracing::warn!(plugin = %update.id, %error, "automatic plugin update could not start");
            }
        }
        Ok(report)
    }

    pub fn plugin_updates(&self) -> Result<UpdateReport, OperationError> {
        Ok(self.packages()?.update_report())
    }

    pub fn spawn_plugin_update_checker(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self.plugin_packages().is_err() {
            return;
        }
        let core = Arc::clone(self);
        runtime.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            loop {
                if core
                    .shutdown_started
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    break;
                }
                let has_registry_packages = core.plugin_package_receipts().is_ok_and(|receipts| {
                    receipts.iter().any(|receipt| receipt.origin.is_registry())
                });
                if has_registry_packages {
                    if let Err(error) = core.plugin_updates_check().await {
                        tracing::warn!(%error, "plugin update check failed");
                    }
                }
                let hours = core
                    .registry_defaults()
                    .map(|defaults| defaults.check_interval_hours)
                    .unwrap_or(12)
                    .clamp(1, 168);
                let deadline = std::time::Instant::now()
                    + std::time::Duration::from_secs(u64::from(hours) * 3600);
                while std::time::Instant::now() < deadline {
                    if core
                        .shutdown_started
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }
        });
    }
}

fn status_only_update(receipt: &InstallReceipt, registry: &str) -> UpdateInfo {
    UpdateInfo {
        id: receipt.id.clone(),
        registry: registry.to_owned(),
        current: receipt.version.clone(),
        latest: None,
        kind: None,
        channel: receipt.policy.channel.clone(),
        pinned: receipt.policy.pinned,
        permission_diff: Default::default(),
        advisory: None,
        changelog: None,
        auto_applicable: false,
        current_status: LiveReleaseStatus::Unknown,
        latest_status: LiveReleaseStatus::Unknown,
    }
}

pub(super) async fn bounded_update_query<T>(
    deadline: tokio::time::Instant,
    query: impl std::future::Future<Output = T>,
) -> Option<T> {
    if tokio::time::Instant::now() >= deadline {
        return None;
    }
    tokio::time::timeout_at(deadline, query).await.ok()
}

//! Explicit, anonymous compatibility discovery; it never activates v2.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_cloud_sync::SyncError;

use super::OperationError;
use crate::AppCore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceV2DiscoveryState {
    SyncDisabled,
    SignInRequired,
    BackendNotSupported,
    BackendCompatible,
}

/// Reviewed local-only reconciliation of install flags for absent packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInventoryReview {
    pub review: String,
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceInventoryRepairParams {
    pub review: String,
}

/// Compatibility is informational, not permission to write or apply data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceV2Discovery {
    pub state: WorkspaceV2DiscoveryState,
    pub desktop_sync_available: bool,
    /// True only for the currently signed-in account with a completed v2
    /// marker and matching, enabled account-scoped consent.
    pub selected: bool,
}

/// Informational local migration estimate; it is not an authorization token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceV2Preview {
    pub selections: usize,
    pub pending: usize,
    pub conflicts: usize,
    pub retained_v1_only: usize,
    pub migration_available: bool,
}

/// Result of the one-time v1 to v2 migration. `completed` means the local
/// cutover committed; `policy_activated` means the server write-epoch policy
/// is active for this account. A completed migration with
/// `policy_activated` false can be retried by calling migrate again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceV2Migration {
    pub selections: usize,
    pub pending: usize,
    pub conflicts: usize,
    pub retained_v1_only: usize,
    pub completed: bool,
    pub policy_activated: bool,
}

fn discovery(state: WorkspaceV2DiscoveryState) -> WorkspaceV2Discovery {
    WorkspaceV2Discovery {
        state,
        desktop_sync_available: false,
        selected: false,
    }
}

fn changed() -> OperationError {
    OperationError::conflict("Cloud session or sync setting changed; check again")
}

impl AppCore {
    pub async fn workspace_inventory_review(
        self: &std::sync::Arc<Self>,
        expected: Option<String>,
    ) -> Result<WorkspaceInventoryReview, OperationError> {
        #[cfg(all(feature = "persistence", feature = "plugin-install"))]
        {
            let session = self.cloud.auth().session().await.ok_or_else(changed)?;
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                core.review_missing_plugin_inventory(session, expected.as_deref())
            })
            .await
            .map_err(|_| OperationError::unavailable("inventory review worker unavailable"))?
        }
        #[cfg(not(all(feature = "persistence", feature = "plugin-install")))]
        {
            let _ = expected;
            Err(OperationError::unavailable(
                "inventory review requires package support",
            ))
        }
    }

    /// Review local inputs without changing either protocol or contacting Cloud.
    pub async fn workspace_v2_preview(
        self: &std::sync::Arc<Self>,
    ) -> Result<WorkspaceV2Preview, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let session = self.cloud.auth().session().await.ok_or_else(changed)?;
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                let preview = core.preview_local_v2_cutover(session)?;
                let candidate = preview.preview();
                let result = WorkspaceV2Preview {
                    selections: candidate.selections().len(),
                    pending: candidate
                        .entries()
                        .iter()
                        .filter(|entry| entry.pending.is_some())
                        .count(),
                    conflicts: candidate
                        .entries()
                        .iter()
                        .filter(|entry| {
                            entry.pending.as_ref().is_some_and(|pending| {
                                pending.status == cloud_client::WorkspacePendingStatus::Conflict
                            })
                        })
                        .count(),
                    retained_v1_only: candidate.unseeded_legacy_keys().len(),
                    migration_available: true,
                };
                preview.check_local_fresh(&core)?;
                Ok(result)
            })
            .await
            .map_err(|_| {
                OperationError::unavailable("Cloud migration review worker is unavailable")
            })?
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud migration review requires persistence",
            ))
        }
    }

    /// Execute the one-time v1 to v2 cutover, then activate the server
    /// write-epoch policy. Idempotent: a retry after a completed cutover
    /// only retries policy activation. The user must confirm first; the
    /// policy is irreversible on the server and affects older v2 clients.
    pub async fn workspace_v2_migrate(
        self: &std::sync::Arc<Self>,
    ) -> Result<WorkspaceV2Migration, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let session = self.cloud.auth().session().await.ok_or_else(changed)?;
            let core = self.clone();
            let core_for_policy = self.clone();
            let origin = session.instance_origin();
            let account = session.account_id().to_owned();
            let origin_check = origin.clone();
            let account_check = account.clone();
            let counts = tokio::task::spawn_blocking(
                move || -> Result<(usize, usize, usize, usize), OperationError> {
                    use tiktools_cloud_sync::ProductLocalEditContext;
                    let coordinator = core.sync.coordinator().map_err(|_| changed())?;
                    let already_v2 = coordinator
                        .with_product_local_edit(|context| match context {
                            ProductLocalEditContext::V2(Some(identity)) => {
                                identity.origin == origin && identity.account_id == account
                            }
                            ProductLocalEditContext::V2(None)
                            | ProductLocalEditContext::Legacy
                            | ProductLocalEditContext::OptedOut => false,
                        })
                        .map_err(|_| changed())?;
                    if already_v2 {
                        return Ok((0, 0, 0, 0));
                    }
                    // Recover a crash between marker commit and consent scoping.
                    // Best effort: a stale window error is ignored; the fresh
                    // preview and execute below handle the normal case.
                    let _ = coordinator.complete_interrupted_v2_scope(&session);
                    let already_v2 = coordinator
                        .with_product_local_edit(|context| match context {
                            ProductLocalEditContext::V2(Some(identity)) => {
                                identity.origin == origin && identity.account_id == account
                            }
                            _ => false,
                        })
                        .map_err(|_| changed())?;
                    if already_v2 {
                        return Ok((0, 0, 0, 0));
                    }
                    let preview = core.preview_local_v2_cutover(session.clone())?;
                    let candidate = preview.preview();
                    let counts = (
                        candidate.selections().len(),
                        candidate
                            .entries()
                            .iter()
                            .filter(|entry| entry.pending.is_some())
                            .count(),
                        candidate
                            .entries()
                            .iter()
                            .filter(|entry| {
                                entry.pending.as_ref().is_some_and(|pending| {
                                    pending.status == cloud_client::WorkspacePendingStatus::Conflict
                                })
                            })
                            .count(),
                        candidate.unseeded_legacy_keys().len(),
                    );
                    core.execute_local_v2_cutover(&preview)?;
                    Ok(counts)
                },
            )
            .await
            .map_err(|_| OperationError::unavailable("Cloud migration worker is unavailable"))??;
            // Policy activation requires the current v2-scoped consent.
            let coordinator = self.sync.coordinator().map_err(|_| changed())?.clone();
            let session = self.cloud.auth().session().await.ok_or_else(changed)?;
            if session.instance_origin() != origin_check || session.account_id() != account_check {
                return Err(changed());
            }
            let expected = tokio::task::spawn_blocking(
                move || -> Result<tiktools_cloud_sync::V2LocalEditContext, OperationError> {
                    let coordinator = core_for_policy.sync.coordinator().map_err(|_| changed())?;
                    match coordinator
                        .with_product_local_edit(|context| context)
                        .map_err(|_| changed())?
                    {
                        tiktools_cloud_sync::ProductLocalEditContext::V2(Some(identity)) => {
                            Ok(identity)
                        }
                        _ => Err(changed()),
                    }
                },
            )
            .await
            .map_err(|_| OperationError::unavailable("Cloud migration worker is unavailable"))??;
            let policy = coordinator
                .ensure_v2_write_epoch_policy(session, expected)
                .await;
            let policy_activated = policy.is_ok();
            Ok(WorkspaceV2Migration {
                selections: counts.0,
                pending: counts.1,
                conflicts: counts.2,
                retained_v1_only: counts.3,
                completed: true,
                policy_activated,
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud migration requires persistence",
            ))
        }
    }

    /// Read the selected protocol locally, without contacting Cloud. A v2
    /// selection is reported only for the current account and consent.
    pub async fn workspace_v2_selected(&self) -> Result<bool, OperationError> {
        let coordinator = self.sync.coordinator().map_err(|_| changed())?.clone();
        let selected = tokio::task::spawn_blocking(move || {
            coordinator.with_product_local_edit(|context| context)
        })
        .await
        .map_err(|_| changed())?
        .map_err(|_| changed())?;
        match selected {
            tiktools_cloud_sync::ProductLocalEditContext::Legacy
            | tiktools_cloud_sync::ProductLocalEditContext::OptedOut => Ok(false),
            tiktools_cloud_sync::ProductLocalEditContext::V2(None) => Err(changed()),
            tiktools_cloud_sync::ProductLocalEditContext::V2(Some(identity)) => {
                let session = self.cloud.auth().session().await.ok_or_else(changed)?;
                let coordinator = self.sync.coordinator().map_err(|_| changed())?;
                let epoch = coordinator
                    .capture_v2_config_epoch(&session)
                    .ok_or_else(changed)?;
                if identity.origin != session.instance_origin()
                    || identity.account_id != session.account_id()
                {
                    return Err(changed());
                }
                self.cloud
                    .auth()
                    .with_current_session(&session, || {
                        coordinator.is_v2_config_epoch_current(&epoch, &session)
                    })
                    .map_err(|_| changed())?
                    .then_some(true)
                    .ok_or_else(changed)
            }
        }
    }

    /// Only this explicitly invoked check contacts the fixed capabilities URL.
    /// It never sends a credential, reads account data, or enables a protocol.
    pub async fn workspace_v2_discover(&self) -> Result<WorkspaceV2Discovery, OperationError> {
        let coordinator = self
            .sync
            .coordinator()
            .map_err(|_| OperationError::unavailable("Cloud sync storage is unavailable"))?;
        let before = self.cloud.auth().session().await;
        let mut result = discover_v2(self.cloud.auth(), coordinator).await?;
        if result.state == WorkspaceV2DiscoveryState::BackendCompatible {
            let Some(session) = self.cloud.auth().session().await else {
                return Err(changed());
            };
            if !before
                .as_ref()
                .is_some_and(|previous| std::sync::Arc::ptr_eq(previous, &session))
            {
                return Err(changed());
            }
            let selected = coordinator
                .with_product_local_edit(|context| context)
                .map_err(|_| changed())?;
            if let tiktools_cloud_sync::ProductLocalEditContext::V2(Some(identity)) = selected {
                let epoch = coordinator
                    .capture_v2_config_epoch(&session)
                    .ok_or_else(changed)?;
                if identity.origin != session.instance_origin()
                    || identity.account_id != session.account_id()
                {
                    return Err(changed());
                }
                result.selected = self
                    .cloud
                    .auth()
                    .with_current_session(&session, || {
                        coordinator.is_v2_config_epoch_current(&epoch, &session)
                    })
                    .map_err(|_| changed())?;
                if !result.selected {
                    return Err(changed());
                }
            }
        }
        Ok(result)
    }
}

async fn discover_v2(
    auth: &tiktools_cloud_auth::CloudAuth,
    coordinator: &tiktools_cloud_sync::SyncCoordinator,
) -> Result<WorkspaceV2Discovery, OperationError> {
    if !coordinator.is_enabled() {
        return Ok(discovery(WorkspaceV2DiscoveryState::SyncDisabled));
    }
    let Some(session) = auth.session().await else {
        return Ok(discovery(WorkspaceV2DiscoveryState::SignInRequired));
    };
    let epoch = coordinator
        .capture_config_epoch(&session)
        .ok_or_else(changed)?;
    auth.with_current_session(&session, || {
        coordinator.is_config_epoch_current(&epoch, &session)
    })
    .map_err(|_| changed())?
    .then_some(())
    .ok_or_else(changed)?;
    let client = cloud_client::CloudCatalog::new(&session.instance_origin())
        .map_err(|_| OperationError::unavailable("Cloud origin is unavailable"))?;
    let result = client.workspace_sync_capabilities().await;
    auth.with_current_session(&session, || {
        coordinator.is_config_epoch_current(&epoch, &session)
    })
    .map_err(|_| changed())?
    .then_some(())
    .ok_or_else(changed)?;
    match result {
        Ok(_) => Ok(discovery(WorkspaceV2DiscoveryState::BackendCompatible)),
        Err(SyncError::Http(404)) => Ok(discovery(WorkspaceV2DiscoveryState::BackendNotSupported)),
        Err(_) => Err(OperationError::unavailable(
            "Cloud v2 capabilities could not be verified",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
    };
    use tiktools_cloud_auth::{CloudAuth, Persistence, TokenStore, TokenStoreError};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::Notify,
    };

    struct TestStore;
    impl TokenStore for TestStore {
        fn persistence(&self) -> Persistence {
            Persistence::MemoryOnly
        }
        fn load(&self, _: &str, _: &str) -> Result<Option<secrecy::SecretString>, TokenStoreError> {
            Ok(Some(secrecy::SecretString::from(
                "private-test-token".to_owned(),
            )))
        }
        fn store(
            &self,
            _: &str,
            _: &str,
            _: &cloud_client::Session,
        ) -> Result<(), TokenStoreError> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
            Ok(())
        }
    }

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "workspace-discovery-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn discovery_requires_opt_in_and_discards_response_after_opt_out_and_in() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let requests = Arc::new(AtomicUsize::new(0));
        let leaked_credential = Arc::new(AtomicBool::new(false));
        let server = {
            let seen = seen.clone();
            let release = release.clone();
            let requests = requests.clone();
            let leaked_credential = leaked_credential.clone();
            tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    let seen = seen.clone();
                    let release = release.clone();
                    let requests = requests.clone();
                    let leaked_credential = leaked_credential.clone();
                    tokio::spawn(async move {
                        let mut bytes = Vec::new();
                        while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                            let mut buf = [0u8; 1024];
                            let Ok(n) = socket.read(&mut buf).await else {
                                return;
                            };
                            if n == 0 || bytes.len() > 16 * 1024 {
                                return;
                            }
                            bytes.extend_from_slice(&buf[..n]);
                        }
                        let request = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
                        let (status, body) = if request.starts_with("get /api/v1/native/me ") {
                            ("200 OK", serde_json::json!({"id":"user-1"}).to_string())
                        } else if request.starts_with("get /api/v2/capabilities ") {
                            requests.fetch_add(1, Ordering::SeqCst);
                            if request.contains("authorization:") || request.contains("cookie:") {
                                leaked_credential.store(true, Ordering::SeqCst);
                            }
                            seen.notify_one();
                            release.notified().await;
                            ("200 OK", serde_json::json!({"apiVersion":2,"instanceMode":"single","userConfigurableRegistries":false,
                                "sync":{"basePath":"/api/v2/sync","schemaVersion":2,"collections":["plugin-selections","rule-templates","profiles","plugin-settings","preferences"],
                                "maxRecords":1000,"maxBytes":8388608,"maxDocumentBytes":262144,"conflictStatus":409,"expiredCursorStatus":410,"preconditionsRequired":true}}).to_string())
                        } else {
                            ("404 Not Found", "{}".to_owned())
                        };
                        let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        let _ = socket.write_all(head.as_bytes()).await;
                        let _ = socket.write_all(body.as_bytes()).await;
                    });
                }
            })
        };
        let temp = TestDir::new();
        let sidecar = temp.0.join("account.json");
        let auth = CloudAuth::new(Some(origin.clone()), sidecar.clone(), Arc::new(TestStore));
        let coordinator = Arc::new(
            tiktools_cloud_sync::SyncCoordinator::new(auth.clone(), temp.0.join("sync")).unwrap(),
        );
        assert_eq!(
            discover_v2(&auth, &coordinator).await.unwrap().state,
            WorkspaceV2DiscoveryState::SyncDisabled
        );
        coordinator.set_enabled(true).unwrap();
        assert_eq!(
            discover_v2(&auth, &coordinator).await.unwrap().state,
            WorkspaceV2DiscoveryState::SignInRequired
        );
        assert_eq!(requests.load(Ordering::SeqCst), 0);
        std::fs::write(
            &sidecar,
            serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"user-1"}))
                .unwrap(),
        )
        .unwrap();
        auth.restore().await.unwrap();
        let pending = {
            let auth = auth.clone();
            let coordinator = coordinator.clone();
            tokio::spawn(async move { discover_v2(&auth, &coordinator).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), seen.notified())
            .await
            .unwrap();
        coordinator.set_enabled(false).unwrap();
        coordinator.set_enabled(true).unwrap();
        release.notify_one();
        assert_eq!(pending.await.unwrap().unwrap_err().code, "conflict");
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        assert!(!leaked_credential.load(Ordering::SeqCst));
        server.abort();
    }
}

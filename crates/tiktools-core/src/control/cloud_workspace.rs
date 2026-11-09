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

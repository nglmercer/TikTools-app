//! Headless Cloud account operations. Everything crossing this boundary
//! is nonsecret ([`AccountStatus`]); tokens stay in native code.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::OperationError;
use crate::{
    cloud_account::{AuthError, CloudLoginBegin},
    events::{DomainEvent, EventBus},
    AppCore,
};

/// Best-effort, nonsecret invalidation for other clients of this AppCore.
/// This is not an acknowledgment or a cross-process authorization fence.
pub(super) struct CloudContextInvalidation<'a>(&'a EventBus);

impl<'a> CloudContextInvalidation<'a> {
    pub(super) fn new(events: &'a EventBus) -> Self {
        events.publish_presentation_domain(DomainEvent::CloudContextInvalidated {});
        Self(events)
    }
}

impl Drop for CloudContextInvalidation<'_> {
    fn drop(&mut self) {
        self.0
            .publish_presentation_domain(DomainEvent::CloudContextInvalidated {});
    }
}

#[cfg(test)]
mod invalidation_tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn context_invalidation_is_nonsecret_and_brackets_the_transition() {
        let events = EventBus::new(4);
        let mut owner = events.subscribe_projection();
        let mut plugin = events.subscribe_plugin_domain();
        let guard = CloudContextInvalidation::new(&events);
        let first = owner.recv().await.unwrap();
        assert_eq!(first.topic(), "cloud.context.invalidated");
        assert_eq!(first.to_envelope().unwrap().data, serde_json::json!({}));
        assert!(!first.is_lossy());
        drop(guard);
        assert_eq!(owner.recv().await.unwrap().topic(), first.topic());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), plugin.recv_shared())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn real_account_and_consent_operations_notify_owner_without_secrets() {
        struct Emitter;
        impl crate::HostEmitter for Emitter {
            fn emit(&self, _: crate::HostMessage) {}
        }
        let root = std::env::temp_dir().join(format!(
            "cloud-context-events-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = crate::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("no-builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(crate::NoopMediaHost),
            paths,
        ));
        let mut owner = core.events.subscribe_projection();
        core.sync_set_enabled(false).unwrap();
        for _ in 0..2 {
            let event = owner.recv().await.unwrap();
            assert_eq!(event.topic(), "cloud.context.invalidated");
            assert_eq!(event.to_envelope().unwrap().data, serde_json::json!({}));
        }
        core.cloud_logout().await.unwrap();
        for _ in 0..2 {
            let event = owner.recv().await.unwrap();
            assert_eq!(event.topic(), "cloud.context.invalidated");
            assert_eq!(event.to_envelope().unwrap().data, serde_json::json!({}));
        }
        core.sync_set_enabled(true).unwrap();
        for _ in 0..2 {
            assert_eq!(
                owner.recv().await.unwrap().topic(),
                "cloud.context.invalidated"
            );
        }
        // sync_now has no active account, so the coordinator itself attempts
        // restore. That indirect path must notify the same owner listeners.
        assert!(core.sync_now().await.is_err());
        for _ in 0..2 {
            assert_eq!(
                owner.recv().await.unwrap().topic(),
                "cloud.context.invalidated"
            );
        }
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudLoginUrl {
    pub browser_url: String,
}

impl From<CloudLoginBegin> for CloudLoginUrl {
    fn from(begin: CloudLoginBegin) -> Self {
        Self {
            browser_url: begin.browser_url,
        }
    }
}

fn auth_error(error: AuthError) -> OperationError {
    let reason = error.code();
    let message = error.to_string();
    let data = serde_json::json!({ "reason": reason });
    let mut operation = match &error {
        AuthError::Unconfigured | AuthError::NoSavedSession | AuthError::StorageUnavailable => {
            OperationError::unavailable(message)
        }
        AuthError::AlreadyInProgress
        | AuthError::NoLoginInProgress
        | AuthError::AlreadySignedIn => OperationError::conflict(message),
        AuthError::Cancelled | AuthError::Expired | AuthError::Failed(_) => {
            OperationError::internal(message)
        }
    };
    operation.data = Some(data);
    operation
}

impl AppCore {
    fn cloud(&self) -> &crate::cloud_account::CloudAccount {
        &self.cloud
    }

    pub async fn cloud_status(&self) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        Ok(self.cloud().auth().status().await)
    }

    pub async fn cloud_begin_login(&self) -> Result<CloudLoginUrl, OperationError> {
        self.cloud_begin_login_for(None).await
    }

    pub async fn cloud_reserve_login(&self) -> Result<String, OperationError> {
        self.cloud()
            .auth()
            .reserve_login()
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_begin_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<CloudLoginUrl, OperationError> {
        self.cloud()
            .auth()
            .begin_login_for(operation)
            .await
            .map(|begin| CloudLoginUrl {
                browser_url: begin.browser_url,
            })
            .map_err(auth_error)
    }

    pub async fn cloud_open_browser(&self) -> Result<(), OperationError> {
        self.cloud_open_browser_for(None).await
    }

    pub async fn cloud_open_browser_for(
        &self,
        operation: Option<&str>,
    ) -> Result<(), OperationError> {
        self.cloud()
            .auth()
            .open_browser_for(operation)
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_complete_login(
        &self,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud_complete_login_for(None).await
    }

    pub async fn cloud_complete_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud()
            .auth()
            .complete_login_for(operation)
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_cancel_login(
        &self,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud_cancel_login_for(None).await
    }

    pub async fn cloud_cancel_login_for(
        &self,
        operation: Option<&str>,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud()
            .auth()
            .cancel_login_for(operation)
            .await
            .map_err(auth_error)
    }

    pub async fn cloud_restore(
        &self,
    ) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        self.cloud().auth().restore().await.map_err(auth_error)
    }

    pub async fn cloud_logout(&self) -> Result<tiktools_cloud_auth::AccountStatus, OperationError> {
        let result = self.cloud().auth().logout().await.map_err(auth_error);
        let service = self
            .collaboration
            .lock()
            .map_err(|_| OperationError::internal("collaboration lock unavailable"))?
            .clone();
        if let Some(service) = service {
            service
                .retain_cloud_peer_source(None)
                .map_err(|error| OperationError {
                    code: error.code(),
                    message: error.to_string(),
                    data: None,
                })?;
        }
        result
    }
}

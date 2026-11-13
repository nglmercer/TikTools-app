use super::OperationError;
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_live::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformConnectParams {
    pub platform: PlatformId,
    pub target: String,
    pub room_id: Option<String>,
    pub credential_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformDisconnectParams {
    pub connection_id: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformCredentialParams {
    pub platform: PlatformId,
    pub token: String,
}
impl std::fmt::Debug for PlatformCredentialParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformCredentialParams")
            .field("platform", &self.platform)
            .field("token", &"[REDACTED]")
            .finish()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCredentialRef {
    pub credential_ref: String,
}

impl AppCore {
    pub fn live_platform_descriptors(&self) -> Vec<PlatformDescriptor> {
        self.live_platforms.manager.descriptors()
    }
    pub fn live_platform_connections(&self) -> Vec<ConnectionSnapshot> {
        let connections = self.live_platforms.manager.snapshots();
        #[cfg(feature = "native-tiktok")]
        let connections = {
            let mut connections = connections;
            let status = self.live_status();
            if status.connected && !connections.iter().any(|s| s.platform == PlatformId::Tiktok) {
                connections.push(ConnectionSnapshot {
                    connection_id: status.connection_id.unwrap_or_else(|| {
                        format!("connection-{}", self.live.connection_snapshot().generation)
                    }),
                    platform: PlatformId::Tiktok,
                    target: status.unique_id.clone().unwrap_or_default(),
                    state: ConnectionState::Connected,
                    channel: status.unique_id.map(|id| ChannelIdentity {
                        platform: PlatformId::Tiktok,
                        id: id.clone(),
                        handle: id,
                        room_id: status.room_id.unwrap_or_default(),
                    }),
                    capabilities: tiktools_tiktok::adapter::descriptor().capabilities,
                    reconnect_attempt: 0,
                    retry_after_ms: None,
                    error: None,
                });
            }
            connections
        };
        connections
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn ensure_legacy_tiktok_available(&self) -> Result<(), OperationError> {
        if self
            .live_platforms
            .manager
            .snapshots()
            .iter()
            .any(|s| s.platform == PlatformId::Tiktok)
        {
            return Err(OperationError::invalid(
                "Disconnect the managed TikTok connection first",
            ));
        }
        Ok(())
    }
    pub async fn live_platform_connect(
        self: &Arc<Self>,
        params: PlatformConnectParams,
    ) -> Result<ConnectionSnapshot, OperationError> {
        if self.is_shutdown() {
            return Err(OperationError::unavailable("Application is shutting down"));
        }
        let credential = match params.credential_ref {
            Some(reference) => {
                let path = self.platform_credential_path(params.platform, &reference)?;
                let bytes = tokio::fs::read(path)
                    .await
                    .map_err(|_| OperationError::invalid("Credential reference is unavailable"))?;
                if bytes.len() > 16_384 {
                    return Err(OperationError::invalid(
                        "Stored credential exceeds its limit",
                    ));
                }
                Credential::new(
                    String::from_utf8(bytes)
                        .map_err(|_| OperationError::invalid("Stored credential is invalid"))?,
                )
            }
            None => Credential::default(),
        };
        #[cfg(feature = "native-tiktok")]
        if params.platform == PlatformId::Tiktok {
            if !matches!(
                self.live.connection_snapshot().state,
                tiktools_tiktok::ConnectionState::Idle | tiktools_tiktok::ConnectionState::Failed
            ) {
                return Err(OperationError::invalid(
                    "Disconnect the current TikTok connection first",
                ));
            }
            self.start_live_event_pump();
        }
        self.start_platform_connections();
        self.live_platforms
            .manager
            .connect(
                params.platform,
                AdapterRequest {
                    target: params
                        .target
                        .trim()
                        .trim_start_matches(['@', '#'])
                        .to_owned(),
                    room_id: params.room_id,
                    credential,
                },
            )
            .await
            .map_err(platform_error)
    }
    pub async fn live_platform_disconnect(
        self: &Arc<Self>,
        params: PlatformDisconnectParams,
    ) -> Result<(), OperationError> {
        #[cfg(feature = "native-tiktok")]
        if self.live_status().connection_id.as_deref() == Some(params.connection_id.as_str()) {
            self.live_disconnect().await;
            return Ok(());
        }
        let tiktok = self
            .live_platform_connections()
            .iter()
            .any(|s| s.connection_id == params.connection_id && s.platform == PlatformId::Tiktok);
        self.live_platforms
            .manager
            .disconnect(&params.connection_id)
            .await
            .map_err(platform_error)?;
        if tiktok {
            self.live_disconnect().await;
        }
        Ok(())
    }
    pub async fn live_platform_store_credential(
        &self,
        params: PlatformCredentialParams,
    ) -> Result<PlatformCredentialRef, OperationError> {
        if params.token.is_empty()
            || params.token.len() > 16_384
            || params.token.chars().any(char::is_control)
        {
            return Err(OperationError::invalid("Invalid platform credential"));
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random)
            .map_err(|_| OperationError::unavailable("Could not create credential reference"))?;
        let reference: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let path = self.platform_credential_path(params.platform, &reference)?;
        tokio::task::spawn_blocking(move || {
            crate::private_file::replace_atomic(&path, params.token.as_bytes())
        })
        .await
        .map_err(|_| OperationError::unavailable("Credential storage task failed"))?
        .map_err(|_| OperationError::unavailable("Could not store platform credential"))?;
        Ok(PlatformCredentialRef {
            credential_ref: reference,
        })
    }
    pub async fn live_platform_delete_credential(
        &self,
        platform: PlatformId,
        reference: &str,
    ) -> Result<(), OperationError> {
        let path = self.platform_credential_path(platform, reference)?;
        match tokio::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(OperationError::unavailable("Could not delete credential")),
        }
    }
    fn platform_credential_path(
        &self,
        platform: PlatformId,
        reference: &str,
    ) -> Result<std::path::PathBuf, OperationError> {
        if reference.len() != 32 || !reference.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(OperationError::invalid("Invalid credential reference"));
        }
        Ok(self
            .db
            .paths()
            .data
            .join("platform-credentials")
            .join(format!("{}-{reference}", platform.as_str())))
    }
}
fn platform_error(error: LiveError) -> OperationError {
    OperationError {
        code: match error.code.as_str() {
            "invalid_params" => "invalid_params",
            "not_found" => "not_found",
            _ => "unavailable",
        },
        message: error.message,
        data: Some(json!({"reason":error.code,"retryAfterMs":error.retry_after_ms})),
    }
}

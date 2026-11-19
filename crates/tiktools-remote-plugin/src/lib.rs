//! First-party remote-control plugin.
//!
//! This crate owns the remote-control feature's lifecycle and transport
//! selection. The desktop host no longer calls `tiktools_remote` directly: it
//! starts this plugin and injects only its authoritative `ControlApi`.
//!
//! Security-critical logic deliberately stays in the host:
//! `tiktools-remote` still routes every frame through
//! `ControlApi::execute_value_with_context`, and the control-api remote policy
//! allowlist still decides which methods an authorized controller may call.
//! The plugin owns *which* transport runs, *how* it is restarted, and *what*
//! status is reported; it never receives a Remake session token, and its
//! `remote.control` declaration is not a grant.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tiktools_control_api::ControlApi;
use tiktools_plugin_api::{
    capabilities::REMOTE_CONTROL, CapabilitySet, PermissionSet, PluginManifest,
};

/// Stable identity of the first-party plugin that owns remote control.
pub const PLUGIN_ID: &str = "tiktools.remote-control";
pub const PLUGIN_VERSION: &str = "1.0.0";

/// A real plugin manifest, parsed and validated by `tiktools-plugin-api`.
/// The plugin is native/trusted: a first-party adapter compiled with the host,
/// not an arbitrary downloaded package.
pub const MANIFEST_JSON: &str = r#"{
    "schemaVersion": 2,
    "id": "tiktools.remote-control",
    "name": "Remote Control",
    "version": "1.0.0",
    "runtime": "native",
    "entry": "tiktools-remote-plugin",
    "capabilities": ["remote.control"],
    "permissions": []
}"#;

type PluginResult<T> = Result<T, String>;

/// Transport owned and selected by the plugin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TransportMode {
    /// HTTPS/WSS gateway on a configured listener.
    #[default]
    Http,
    /// Iroh peer gateway (outbound, no inbound listener).
    Peer,
}

/// Plugin-owned configuration. These values used to be implicit in the
/// desktop startup hook; the plugin now decides them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteControlConfig {
    pub transport: TransportMode,
    pub restart_on_failure: bool,
    pub max_restarts: u32,
    pub restart_backoff_ms: u64,
}

impl Default for RemoteControlConfig {
    fn default() -> Self {
        Self {
            transport: TransportMode::Http,
            restart_on_failure: true,
            max_restarts: 5,
            restart_backoff_ms: 500,
        }
    }
}

/// Observable state of the plugin-owned gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayStatus {
    pub transport: TransportMode,
    pub running: bool,
    pub restarts: u32,
    pub last_error: Option<String>,
    pub address: Option<String>,
}

impl Default for GatewayStatus {
    fn default() -> Self {
        Self {
            transport: TransportMode::Http,
            running: false,
            restarts: 0,
            last_error: None,
            address: None,
        }
    }
}

/// A minimal, runtime-neutral description of what the plugin asks the host
/// for. The host still decides whether to grant it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDeclaration {
    pub id: String,
    pub version: String,
    pub runtime: String,
    pub trust: String,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
}

/// One running gateway instance, owned by the plugin.
pub enum GatewayHandle {
    Http(tiktools_remote::RemoteGateway),
    #[cfg(feature = "peer")]
    Peer(tiktools_remote::PeerGateway),
}

impl GatewayHandle {
    pub fn transport(&self) -> TransportMode {
        match self {
            Self::Http(_) => TransportMode::Http,
            #[cfg(feature = "peer")]
            Self::Peer(_) => TransportMode::Peer,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self {
            Self::Http(gateway) => gateway.is_finished(),
            #[cfg(feature = "peer")]
            Self::Peer(gateway) => gateway.is_finished(),
        }
    }

    pub fn address(&self) -> Option<String> {
        match self {
            Self::Http(gateway) => Some(gateway.local_addr().to_string()),
            #[cfg(feature = "peer")]
            Self::Peer(gateway) => Some(format!("{:?}", gateway.address())),
        }
    }

    pub async fn shutdown(self) -> PluginResult<()> {
        match self {
            Self::Http(gateway) => gateway.shutdown().await.map_err(|e| e.to_string()),
            #[cfg(feature = "peer")]
            Self::Peer(gateway) => {
                gateway.shutdown().await;
                Ok(())
            }
        }
    }
}

/// Handle to the plugin's supervision loop, allowing a host to observe status
/// or request a clean stop.
#[derive(Clone)]
pub struct SupervisorHandle {
    status: Arc<Mutex<GatewayStatus>>,
    stop: Arc<tokio::sync::Notify>,
}

impl SupervisorHandle {
    pub fn status(&self) -> GatewayStatus {
        self.status
            .lock()
            .expect("remote-control status lock poisoned")
            .clone()
    }

    pub fn stop(&self) {
        self.stop.notify_one();
    }
}

/// The plugin that owns remote control.
#[derive(Debug, Clone)]
pub struct RemoteControlPlugin {
    id: String,
    version: String,
    config: RemoteControlConfig,
}

impl Default for RemoteControlPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteControlPlugin {
    pub fn new() -> Self {
        Self {
            id: PLUGIN_ID.to_owned(),
            version: PLUGIN_VERSION.to_owned(),
            config: RemoteControlConfig::default(),
        }
    }

    pub fn with_config(config: RemoteControlConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn config(&self) -> &RemoteControlConfig {
        &self.config
    }

    /// Parse and validate the embedded manifest with the real plugin API.
    pub fn manifest(&self) -> PluginResult<PluginManifest> {
        PluginManifest::from_json_str(MANIFEST_JSON).map_err(|e| e.to_string())
    }

    /// The capability this plugin declares. A declaration is not a grant:
    /// the host's `CapabilityBroker` and the remote policy remain authoritative.
    pub fn declaration(&self) -> PluginDeclaration {
        PluginDeclaration {
            id: self.id.clone(),
            version: self.version.clone(),
            runtime: "native".to_owned(),
            trust: "trusted".to_owned(),
            capabilities: vec![REMOTE_CONTROL.to_owned()],
            permissions: Vec::new(),
        }
    }

    pub fn declared_capabilities(&self) -> CapabilitySet {
        CapabilitySet::from_strings(self.declaration().capabilities)
    }

    pub fn declared_permissions(&self) -> PermissionSet {
        PermissionSet::from_strings(self.declaration().permissions)
    }

    /// Start exactly one gateway with the configured transport.
    pub async fn start_once(&self, api: Arc<ControlApi>) -> PluginResult<GatewayHandle> {
        match self.config.transport {
            TransportMode::Http => tiktools_remote::start(api)
                .await
                .map(GatewayHandle::Http)
                .map_err(|e| e.to_string()),
            TransportMode::Peer => {
                #[cfg(feature = "peer")]
                {
                    tiktools_remote::start_peer(api)
                        .await
                        .map(GatewayHandle::Peer)
                        .map_err(|e| e.to_string())
                }
                #[cfg(not(feature = "peer"))]
                {
                    let _ = api;
                    Err("peer transport is not compiled into this build".to_owned())
                }
            }
        }
    }

    /// Supervision loop owned by the plugin: start the chosen transport,
    /// watch it, and restart with bounded backoff when it fails.
    pub async fn supervise(self, api: Arc<ControlApi>) {
        let status = Arc::new(Mutex::new(GatewayStatus {
            transport: self.config.transport,
            ..GatewayStatus::default()
        }));
        let stop = Arc::new(tokio::sync::Notify::new());
        self.supervise_inner(api, status, stop).await;
    }

    /// Spawn the supervision loop and return a handle for status/stop.
    pub fn spawn(self, api: Arc<ControlApi>) -> SupervisorHandle {
        let status = Arc::new(Mutex::new(GatewayStatus {
            transport: self.config.transport,
            ..GatewayStatus::default()
        }));
        let stop = Arc::new(tokio::sync::Notify::new());
        let handle = SupervisorHandle {
            status: Arc::clone(&status),
            stop: Arc::clone(&stop),
        };
        let plugin = self;
        tokio::spawn(async move { plugin.supervise_inner(api, status, stop).await });
        handle
    }

    async fn supervise_inner(
        self,
        api: Arc<ControlApi>,
        status: Arc<Mutex<GatewayStatus>>,
        stop: Arc<tokio::sync::Notify>,
    ) {
        let mut restarts = 0u32;
        loop {
            match self.start_once(Arc::clone(&api)).await {
                Ok(gateway) => {
                    let address = gateway.address();
                    {
                        let mut state = status.lock().expect("status lock poisoned");
                        state.running = true;
                        state.last_error = None;
                        state.address = address;
                    }
                    // Watch the gateway until it finishes or the host stops us.
                    loop {
                        tokio::select! {
                            _ = stop.notified() => {
                                let _ = gateway.shutdown().await;
                                if let Ok(mut state) = status.lock() {
                                    state.running = false;
                                }
                                return;
                            }
                            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                                if gateway.is_finished() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    if let Ok(mut state) = status.lock() {
                        state.last_error = Some(error.clone());
                    }
                }
            }

            if let Ok(mut state) = status.lock() {
                state.running = false;
                state.restarts += 1;
                restarts = state.restarts;
            }
            if !self.config.restart_on_failure || restarts >= self.config.max_restarts {
                return;
            }
            tokio::time::sleep(Duration::from_millis(self.config.restart_backoff_ms)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_declares_remote_control_but_no_permissions() {
        let plugin = RemoteControlPlugin::new();
        let declaration = plugin.declaration();
        assert_eq!(declaration.id, PLUGIN_ID);
        assert_eq!(declaration.version, PLUGIN_VERSION);
        assert_eq!(declaration.runtime, "native");
        assert_eq!(declaration.trust, "trusted");
        assert!(declaration.permissions.is_empty());
        assert!(declaration.capabilities.iter().any(|c| c == REMOTE_CONTROL));
    }

    #[test]
    fn declared_capability_is_recognized_by_the_plugin_api() {
        let plugin = RemoteControlPlugin::new();
        let capabilities = plugin.declared_capabilities();
        assert!(capabilities.contains(REMOTE_CONTROL));
        assert!(!capabilities.contains("http.request"));
        assert_eq!(plugin.declared_permissions().iter().count(), 0);
    }

    #[test]
    fn embedded_manifest_is_valid_and_declares_remote_control() {
        let plugin = RemoteControlPlugin::new();
        let manifest = plugin.manifest().expect("valid embedded manifest");
        assert_eq!(manifest.id, PLUGIN_ID);
        assert_eq!(manifest.version, PLUGIN_VERSION);
        assert!(manifest.capabilities.iter().any(|c| c == REMOTE_CONTROL));
        assert!(manifest.permissions.is_empty());
    }

    #[test]
    fn plugin_owns_transport_and_restart_configuration() {
        let plugin = RemoteControlPlugin::new();
        assert_eq!(plugin.config().transport, TransportMode::Http);
        assert!(plugin.config().restart_on_failure);
        assert_eq!(plugin.config().max_restarts, 5);

        let custom = RemoteControlConfig {
            transport: TransportMode::Peer,
            restart_on_failure: false,
            max_restarts: 0,
            restart_backoff_ms: 10,
        };
        let peer_plugin = RemoteControlPlugin::with_config(custom.clone());
        assert_eq!(peer_plugin.config(), &custom);

        let status = GatewayStatus {
            transport: custom.transport,
            running: true,
            restarts: 2,
            last_error: Some("boom".to_owned()),
            address: None,
        };
        assert_eq!(status.transport, TransportMode::Peer);
        assert!(status.running);
        assert_eq!(status.restarts, 2);
    }

    #[test]
    fn declaration_is_not_a_grant() {
        // The plugin only declares capabilities. The host's broker and the
        // remote policy allowlist decide what is actually executed.
        let plugin = RemoteControlPlugin::new();
        assert_eq!(plugin.declared_permissions().iter().count(), 0);
        assert_eq!(
            plugin.declared_capabilities().iter().count(),
            1,
            "only remote.control is requested; no blanket host access"
        );
    }
}

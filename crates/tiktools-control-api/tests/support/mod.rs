//! Fixture shared by the Owner API integration tests: a real `AppCore` over a
//! throwaway data directory, the control API on top of it, and a paired remote
//! session built from a caller-chosen grant.
#![allow(dead_code)]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{
        hash_secret, Authorization, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem,
    },
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};

pub struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
pub struct Fixture {
    pub core: Arc<AppCore>,
    pub api: ControlApi,
    pub root: std::path::PathBuf,
    pub paths: AppPaths,
}
impl Fixture {
    /// `prefix` names the temp directory so concurrent test binaries do not collide.
    pub fn with_prefix(prefix: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        Self::open(paths)
    }
    /// Starts a core over existing `paths`, e.g. to restart over the same data.
    pub fn open(paths: AppPaths) -> Self {
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ));
        Self {
            api: ControlApi::new(core.clone()),
            core,
            root: paths.root.clone(),
            paths,
        }
    }
    /// Enables collaboration, redeems an invitation carrying `grant` and
    /// returns its live authorization. `exchange` is the fill
    /// character of the 64-character exchange secret.
    pub fn pair_authorization(
        &self,
        grant: Grant,
        client_label: &str,
        exchange: char,
    ) -> Authorization {
        let service = self.core.collaboration().unwrap();
        let mut configuration = service.configuration().unwrap();
        configuration.enabled = true;
        configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(configuration).unwrap();
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = exchange.to_string().repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: client_label.into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&pending.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap()
    }
    /// [`Self::pair_authorization`] as a remote request context.
    pub fn pair_with(&self, grant: Grant, client_label: &str, exchange: char) -> RequestContext {
        let auth = self.pair_authorization(grant, client_label, exchange);
        RequestContext::Remote(RemoteSession::from_authorization(auth).unwrap())
    }
    pub async fn call_as(
        &self,
        context: &RequestContext,
        method: &str,
        params: Value,
        meta: Value,
    ) -> RpcResponse {
        let mut request = json!({"id":1,"method":method,"params":params});
        if !meta.is_null() {
            request["meta"] = meta;
        }
        self.api.execute_value_with_context(context, &request).await
    }
}

/// The capabilities a broad remote client is paired with in these tests:
/// everything except HTTP, audio, event emission and script execution.
pub fn broad_grant() -> Grant {
    Grant {
        capabilities: [
            Capability::LiveRead,
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::RankingsRead,
            Capability::RankingsManage,
            Capability::CompetitionsRead,
            Capability::CompetitionsManage,
            Capability::WorkflowsRead,
            Capability::WorkflowsEdit,
            Capability::WorkflowsToggle,
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::AutomationToggle,
            Capability::PluginsRead,
            Capability::PluginsToggle,
            Capability::PluginSettingsRead,
            Capability::PluginSettingsWrite,
            Capability::PluginActionExecute,
            Capability::WidgetsRead,
            Capability::WidgetsConfigure,
            Capability::CommandsRead,
        ]
        .into(),
        ..Grant::default()
    }
}

/// [`broad_grant`] plus HTTP, audio, event emission and script execution.
pub fn all_capabilities_grant() -> Grant {
    let mut grant = broad_grant();
    grant.capabilities.extend([
        Capability::HttpRequest,
        Capability::AudioPlay,
        Capability::EventsEmit,
        Capability::ScriptsExecute,
    ]);
    grant
}

//! Helpers shared by the owner journal unit tests: a silent host emitter, the
//! app paths under one root and a core opened over them.
use crate::{
    collaboration::{
        hash_secret, Authorization, ClientKind, CollaborationService, Collect, Grant, InviteCreate,
        Redeem,
    },
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};
use std::{path::Path, sync::Arc};

pub(crate) struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}

/// App paths for a throwaway root, with an absent built-in plugin directory.
pub(crate) fn paths(root: &Path) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.into(),
    }
}

/// A core opened over `paths`; reopening over the same paths simulates a restart.
pub(crate) fn open_core(paths: &AppPaths) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths.clone(),
    ))
}

/// Pairs a remote client holding `grant` and returns its live authorization.
/// Collaboration is switched on the first time; `exchange` is the fill
/// character of the 64-character exchange secret.
pub(crate) fn pair_authorization(
    core: &AppCore,
    grant: Grant,
    client_label: &str,
    exchange: char,
) -> Authorization {
    let service = core.collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    if !config.enabled {
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(config).unwrap();
    }
    pair_service(&service, grant, client_label, exchange)
}

/// Invites, redeems, approves and collects a client on an already enabled
/// collaboration `service`.
pub(crate) fn pair_service(
    service: &CollaborationService,
    grant: Grant,
    client_label: &str,
    exchange: char,
) -> Authorization {
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

/// The accessors every owner journal test fixture shares. The fixture has
/// `core: Option<Arc<AppCore>>` and `paths: AppPaths` fields; `reopen` drops
/// the core and opens a new one over the same data, as after a restart.
macro_rules! owner_fixture_core_methods {
    () => {
        fn core(&self) -> &std::sync::Arc<crate::AppCore> {
            self.core.as_ref().unwrap()
        }
        fn db(&self) -> rusqlite::Connection {
            rusqlite::Connection::open(self.paths.automation_database()).unwrap()
        }
        fn reopen(&mut self) {
            drop(self.core.take());
            self.core = Some($crate::control::test_support::open_core(&self.paths));
        }
    };
}
pub(crate) use owner_fixture_core_methods;

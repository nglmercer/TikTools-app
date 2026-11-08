use super::*;
use crate::{collaboration::*, control::CommandCaller, paths::AppPaths};
use std::sync::Arc;

const SECRET: &str = "manifest-private-fixture-source";
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-manifest-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
        };
        paths.ensure_directories().unwrap();
        for p in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(p).unwrap();
        }
        Self {
            core: Some(Self::open(&paths)),
            paths,
        }
    }
    fn open(paths: &AppPaths) -> Arc<AppCore> {
        Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ))
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn reopen(&mut self) {
        drop(self.core.take());
        self.core = Some(Self::open(&self.paths));
    }
    fn action(&self, id: &str) {
        self.core().automation_create(AutomationKind::Action,json!({"id":id,"name":"Private action","enabled":false,"typeId":"core.log","config":{"message":SECRET}})).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}
fn author(core: &AppCore) -> Authorization {
    let service = core.collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(config).unwrap();
    let invite = service
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let exchange = "b".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Manifest fixture author".into(),
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
fn delegated(core: &AppCore, auth: &Authorization, id: &str) {
    core.automation_command(CommandCaller::Remote(auth.clone()),CommandRequest{command_id:format!("manifest-proposal-{}",fastrand::u64(..)),host_instance_id:core.collaboration().unwrap().instance_id().into(),expected_revision:"0".into()},AutomationMutation::Create{kind:AutomationKind::Event,record:json!({"id":id,"name":"Saved proposal","enabled":false,"trigger":"tiktok.chat","actionIds":["same"],"authority":"owner","provenance":{"authority":{"kind":"owner"}},"principalId":"forged-body-principal","sessionId":"forged-body-session"})}).unwrap();
}

#[test]
fn owner_manifest_binds_every_dependency_without_source_or_execution_and_changes_with_a_disabled_dependency(
) {
    let f = Fixture::new();
    f.action("same");
    f.core().automation_create(AutomationKind::Event,json!({"id":"same","name":"Random event","enabled":true,"trigger":"tiktok.chat","runMode":"random","actionIds":["same","missing","same"]})).unwrap();
    let before = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "same")
        .unwrap();
    assert_eq!(
        before
            .resource_versions
            .iter()
            .map(|v| (v.scope.as_str(), v.id.as_str(), v.revision.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("event", "same", "1"),
            ("action", "same", "1"),
            ("action", "missing", "0")
        ]
    );
    assert!(!before.value.to_string().contains(SECRET));
    assert!(before.value.get("policyVersion").is_some());
    assert!(f.core().automation_runs().is_empty());
    f.core().automation_update(AutomationKind::Action,"same",json!({"id":"same","name":"Private action","enabled":false,"typeId":"core.log","config":{"message":"changed private source"}})).unwrap();
    let after = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "same")
        .unwrap();
    assert_ne!(before.value, after.value);
    assert_eq!(after.resource_versions[0].revision, "1");
    assert_eq!(after.resource_versions[1].revision, "2");
    assert!(!after.value.to_string().contains("changed private source"));
    assert!(f.core().automation_runs().is_empty());
    assert_eq!(
        f.core()
            .automation_get(AutomationKind::Action, "same")
            .unwrap()["enabled"],
        false
    );
}

#[test]
fn delegated_manifest_uses_trusted_saved_identity_and_current_grant_instead_of_definition_fields() {
    let f = Fixture::new();
    f.action("same");
    let auth = author(f.core());
    let identity = auth.snapshot().unwrap();
    delegated(f.core(), &auth, "proposal");
    let before = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "proposal")
        .unwrap();
    let encoded = before.value.to_string();
    assert!(encoded.contains(&identity.principal_id));
    assert!(encoded.contains(&identity.session_id));
    assert!(!encoded.contains("forged-body-principal"));
    assert!(!encoded.contains("forged-body-session"));
    assert!(!encoded.contains(SECRET));
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id.clone(),
            expected_revision: identity.grant_revision,
            grant: Grant {
                capabilities: [Capability::AutomationRead].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let after = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "proposal")
        .unwrap();
    assert_ne!(before.value, after.value);
    assert_eq!(before.resource_versions, after.resource_versions);
    f.core()
        .collaboration()
        .unwrap()
        .revoke_session(&identity.session_id, "local-owner")
        .unwrap();
    assert_eq!(
        f.core()
            .automation_execution_manifest(AutomationKind::Event, "proposal")
            .unwrap_err()
            .code,
        "unauthorized"
    );
    assert!(f.core().automation_runs().is_empty());
}

#[test]
fn manifest_reopens_from_provenance_and_session_storage_without_a_bearer_or_live_connection() {
    let mut f = Fixture::new();
    f.action("same");
    let auth = author(f.core());
    let identity = auth.snapshot().unwrap();
    delegated(f.core(), &auth, "proposal");
    let before = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "proposal")
        .unwrap();
    drop(auth);
    f.reopen();
    let after = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "proposal")
        .unwrap();
    assert_eq!(before, after);
    f.core()
        .collaboration()
        .unwrap()
        .revoke_principal(&identity.principal_id)
        .unwrap();
    assert_eq!(
        f.core()
            .automation_execution_manifest(AutomationKind::Event, "proposal")
            .unwrap_err()
            .code,
        "unauthorized"
    );
}

#[test]
fn missing_saved_primary_never_becomes_an_empty_owner_execution_manifest() {
    let f = Fixture::new();
    assert_eq!(
        f.core()
            .automation_execution_manifest(AutomationKind::Action, "never-created")
            .unwrap_err()
            .code,
        "not_found"
    );
    assert!(f.core().automation_runs().is_empty());
}

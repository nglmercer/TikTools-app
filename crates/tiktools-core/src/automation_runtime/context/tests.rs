use super::*;
use crate::{
    collaboration::*, live_events::AutomationEvent, paths::AppPaths, HostEmitter, NoopMediaHost,
};
use serde_json::json;
use std::sync::Arc;

#[cfg(feature = "persistence")]
#[test]
fn saved_effect_identity_survives_action_reordering_and_unrelated_insertions() {
    let incoming = ExecutionContext::owner();
    let identities = |actions: &[&str]| {
        let mut occurrences = std::collections::BTreeMap::<String, u32>::new();
        let mut identities = std::collections::BTreeMap::new();
        for action in actions {
            let occurrence = occurrences.entry((*action).into()).or_default();
            let context = incoming
                .effect_named_slot("saved-event", action, *occurrence)
                .unwrap();
            let key = context
                .execution()
                .unwrap()
                .key(
                    crate::control::EffectKind::Http,
                    &hash_secret("exact-approved-binding"),
                    &hash_secret("exact-rendered-request"),
                )
                .unwrap();
            identities.insert(((*action).to_owned(), *occurrence), key.id().to_owned());
            *occurrence += 1;
        }
        identities
    };
    let original = identities(&["http-action", "audio-action", "http-action"]);
    let changed = identities(&[
        "unrelated-action",
        "http-action",
        "http-action",
        "audio-action",
    ]);
    for (slot, identity) in &original {
        assert_eq!(changed.get(slot), Some(identity), "{slot:?}");
    }
    assert_ne!(
        original[&("http-action".into(), 0)],
        original[&("http-action".into(), 1)]
    );
    let other_event = incoming
        .effect_named_slot("another-event", "http-action", 0)
        .unwrap();
    let key = other_event
        .execution()
        .unwrap()
        .key(
            crate::control::EffectKind::Http,
            &hash_secret("exact-approved-binding"),
            &hash_secret("exact-rendered-request"),
        )
        .unwrap();
    assert_ne!(original[&("http-action".into(), 0)], key.id());
}

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
    identity: Authenticated,
    authorization: Authorization,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-context-{}-{}",
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
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ));
        let s = core.collaboration().unwrap();
        let mut config = s.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        s.configure(config).unwrap();
        let invite = s
            .create_invitation(InviteCreate {
                grant: Grant {
                    capabilities: [
                        Capability::AutomationRead,
                        Capability::AutomationEdit,
                        Capability::PointsRead,
                        Capability::PointsAdjust,
                        Capability::PluginActionExecute,
                    ]
                    .into(),
                    ..Grant::default()
                },
            })
            .unwrap();
        let secret = "c".repeat(64);
        let pending = s
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Context fixture".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&secret),
                },
                "fixture",
            )
            .unwrap();
        s.decide_request(&pending.request_id, true).unwrap();
        let c = s
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: secret,
                },
                "fixture",
            )
            .unwrap();
        let identity = s.authenticate(&c.access_token, false, false).unwrap();
        let authorization = s.authenticate_live(&c.access_token, false, false).unwrap();
        Self {
            core: Some(core),
            paths,
            identity,
            authorization,
        }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn context(&self) -> ExecutionContext {
        let s = self.core().collaboration().unwrap();
        ExecutionContext::restricted(
            s.execution_authorizations(&[(
                self.identity.principal_id.clone(),
                self.identity.session_id.clone(),
            )])
            .unwrap(),
        )
        .unwrap()
    }
}

#[cfg(feature = "native-tiktok")]
mod approved_points_replay;
mod scoped_flow;
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}

#[test]
fn context_intersection_never_promotes_restrictions_or_resets_trusted_depth() {
    let f = Fixture::new();
    let restricted = f.context();
    let owner = ExecutionContext::owner();
    for context in [
        owner.intersect(&restricted).unwrap(),
        restricted.intersect(&owner).unwrap(),
        restricted.intersect(&restricted).unwrap(),
    ] {
        assert!(!context.is_owner());
        assert!(context.require_owner().is_err());
        assert!(context.check_current().is_ok());
        let nested = context.child().unwrap().child().unwrap();
        assert_eq!(nested.depth(), 2);
        assert!(nested.child().is_err());
        assert_eq!(nested.intersect(&owner).unwrap().depth(), 2);
    }
    assert!(ExecutionContext::denied()
        .intersect(&owner)
        .unwrap()
        .require_owner()
        .is_err());
    let s = f.core().collaboration().unwrap();
    assert!(ExecutionContext::restricted(s.execution_authorizations(&[]).unwrap()).is_err());
}

#[test]
fn intersections_require_one_service_and_revalidate_retained_subjects_after_revocation() {
    let a = Fixture::new();
    let b = Fixture::new();
    let context = a.context();
    assert!(context.intersect(&b.context()).is_err());
    let child = context.child().unwrap();
    a.core()
        .collaboration()
        .unwrap()
        .revoke_session(&a.identity.session_id, "local-owner")
        .unwrap();
    assert!(context.check_current().is_err());
    assert!(child.check_current().is_err());
    assert!(context.intersect(&context).is_err());
    assert!(context.require_owner().is_err());
}

#[tokio::test]
async fn forged_payload_fields_do_not_authorize_restricted_actions_or_reset_an_envelope() {
    let f = Fixture::new();
    let context = f.context().child().unwrap();
    let forged = json!({"id":"fake","type":"tiktok.chat","authority":"owner","executionAuthorized":true,"source":{"kind":"owner"},"data":{"depth":0},"event":{"context":"owner"}});
    let envelope = AutomationEvent::new(forged.clone(), context.clone());
    assert!(!envelope.context().is_owner());
    assert_eq!(envelope.context().depth(), 1);
    let action = json!({"id":"fresh","typeId":"core.log","config":{"message":"must-not-execute"}});
    let run = f
        .core()
        .execute_action_with_context(&action, &forged, None, &context, false)
        .await;
    assert_eq!(run["error"], "authority_required");
    assert!(f.core().automation_runs().is_empty());
    f.core().publish_automation_event(envelope).await;
    assert!(f.core().automation_runs().is_empty());
    assert!(f.core().last_event_for("tiktok.chat").is_none());
}

#[tokio::test]
async fn every_effect_transport_rejects_restricted_context_before_templates_plugins_or_network() {
    let f = Fixture::new();
    let context = f.context();
    let core = f.core();
    core.globals_set("privateContextValue", "private-context-secret")
        .unwrap();
    assert!(core
        .automation_globals(&context)
        .unwrap_err()
        .contains("approval"));
    assert_eq!(
        core.automation_globals(&ExecutionContext::owner()).unwrap()["privateContextValue"],
        "private-context-secret"
    );
    let event = json!({"type":"tiktok.chat","user":{"uniqueId":"viewer"}});
    let globals =
        std::collections::BTreeMap::from([("secret".into(), "private-context-secret".into())]);
    let mut logs = Vec::new();
    for (kind, config) in [
        ("core.log", json!({"message":"{{globals.secret}}"})),
        ("core.points", json!({"uniqueId":"viewer","delta":1})),
        (
            "core.code",
            json!({"source":"throw new Error('must-not-run')"}),
        ),
        ("core.emit", json!({"type":"overlay.alert","data":{}})),
    ] {
        let config = config.as_object().unwrap();
        let result = core
            .execute_action_impl(
                kind,
                config,
                &json!({}),
                &event,
                &globals,
                &context,
                &mut logs,
                false,
            )
            .await;
        assert!(result.unwrap_err().contains("approval"));
    }
    let fetch = json!({"url":"http://127.0.0.1:1/{{globals.secret}}"});
    assert!(core
        .execute_http_action(
            fetch.as_object().unwrap(),
            &event,
            &globals,
            &context,
            &mut logs,
            None,
            false
        )
        .await
        .unwrap_err()
        .contains("approval"));
    let audio = json!({"fileRef":{"path":"{{globals.secret}}"}});
    assert!(core
        .execute_audio_action(
            audio.as_object().unwrap(),
            &event,
            &globals,
            &context,
            &mut logs,
            false
        )
        .await
        .unwrap_err()
        .contains("approval"));
    assert!(core
        .execute_plugin_action(
            "unknown-plugin.action",
            &json!({}),
            &event,
            &context,
            &mut logs,
            false
        )
        .await
        .unwrap_err()
        .contains("approval"));
    let request = crate::automation_runtime::HardenedHttpRequest {
        method: "GET".into(),
        url: "http://127.0.0.1:1".into(),
        headers: vec![],
        body: None,
        timeout_ms: 50,
    };
    assert!(core
        .send_hardened_http_request(&request, "127.0.0.1", &context)
        .await
        .err()
        .unwrap()
        .contains("approval"));
    assert!(logs.is_empty());
    assert!(core.automation_runs().is_empty());
}

#[tokio::test]
async fn shutdown_blocks_even_an_otherwise_owner_context_before_action_dispatch() {
    let f = Fixture::new();
    f.core().shutdown().await;
    let owner = ExecutionContext::owner();
    assert!(f.core().execution_context_guard(&owner).is_err());
    let result = f
        .core()
        .execute_action_with_context(
            &json!({"typeId":"core.log","config":{"message":"after-shutdown"}}),
            &json!({}),
            None,
            &owner,
            false,
        )
        .await;
    assert_eq!(result["error"], "authority_required");
    assert!(f.core().automation_runs().is_empty());
}

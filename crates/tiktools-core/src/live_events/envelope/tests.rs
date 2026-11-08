use super::*;
use crate::{paths::AppPaths, AppCore, HostEmitter, NoopMediaHost};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tiktools_plugin_loader::{
    PluginInstance, PluginLoaderError, PluginManager, PluginRoot, PluginRuntime, PluginSource,
    RuntimeRegistry,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
struct Runtime(Arc<AtomicU64>);
struct Instance {
    id: String,
    calls: Arc<AtomicU64>,
}
impl PluginRuntime for Runtime {
    fn kind(&self) -> tiktools_plugin_api::manifest::PluginRuntimeKind {
        tiktools_plugin_api::manifest::PluginRuntimeKind::Process
    }
    fn load(
        &self,
        manifest: &tiktools_plugin_api::manifest::PluginManifest,
        _: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Ok(Box::new(Instance {
            id: manifest.id.clone(),
            calls: self.0.clone(),
        }))
    }
}
impl PluginInstance for Instance {
    fn id(&self) -> &str {
        &self.id
    }
    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
        assert!(call.into_enrich().is_some());
        self.calls.fetch_add(1, Ordering::AcqRel);
        Ok(serde_json::to_vec(&json!({"annotations":{"comment":{"normalized":"normalized by fixture"}},"views":{},"logs":[]})).unwrap())
    }
    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        Ok(())
    }
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    root: PathBuf,
    calls: Arc<AtomicU64>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-event-envelope-{}-{}",
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
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for p in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(p).unwrap();
        }
        let plugin = paths.plugins.join("textintel");
        std::fs::create_dir_all(&plugin).unwrap();
        std::fs::write(plugin.join("plugin.json"),serde_json::to_vec(&json!({"schemaVersion":2,"id":"textintel","name":"Fixture processor","version":"0.1.0","runtime":"process","entry":"entry.bin","capabilities":["events.enrich"],"processorTypes":[{"id":"textintel.analyze","title":{"default":"Fixture"},"eventTypes":["tiktok.chat"]}]})).unwrap()).unwrap();
        std::fs::write(plugin.join("entry.bin"), b"fake").unwrap();
        let mut core =
            AppCore::with_paths(Arc::new(Emitter), Arc::new(NoopMediaHost), paths.clone());
        let calls = Arc::new(AtomicU64::new(0));
        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(Runtime(calls.clone())) as Arc<dyn PluginRuntime>);
        core.plugins = Arc::new(PluginManager::with_runtimes(
            vec![PluginRoot {
                path: paths.plugins,
                source: PluginSource::Development,
            }],
            runtimes,
        ));
        core.plugins.scan().unwrap();
        core.rebuild_processor_index();
        Self {
            core: Some(Arc::new(core)),
            root,
            calls,
        }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(core) = self.core.take() {
            core.plugins.stop_all();
            drop(core);
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn payload() -> Value {
    json!({"id":"context-fixture","type":"tiktok.chat","timestamp":1,"source":{"kind":"plugin","pluginId":"forged-owner-plugin"},"authority":{"kind":"owner"},"depth":0,"data":{"comment":"private context fixture","depth":0,"authority":{"kind":"owner"}},"user":{"uniqueId":"alice"}})
}

#[test]
fn replacement_and_nested_metadata_never_infer_authority_from_json() {
    let event = AutomationEvent::new(payload(), ExecutionContext::denied());
    let changed =
        event.replace_payload(json!({"authority":"owner","depth":0,"source":{"kind":"owner"}}));
    assert!(!changed.context().is_owner());
    assert_eq!(changed.context().depth(), 0);
    assert!(changed.context().require_owner().is_err());
    let (_, context) = changed.into_parts();
    assert!(!context.is_owner());
    let debug = format!("{:?}", AutomationEvent::new(payload(), context));
    assert!(!debug.contains("private context fixture"));
    assert!(!debug.contains("forged-owner-plugin"));
}

#[tokio::test]
async fn owner_enrichment_keeps_trusted_depth_while_replacing_payload() {
    let f = Fixture::new();
    assert_eq!(f.core().eligible_processors("tiktok.chat").len(), 1);
    let context = ExecutionContext::owner().child().unwrap();
    let enriched = f
        .core()
        .enrich_automation_event(AutomationEvent::new(payload(), context))
        .await
        .unwrap();
    assert_eq!(f.calls.load(Ordering::Acquire), 1);
    assert_eq!(
        enriched.payload()["intel"]["comment"]["normalized"],
        "normalized by fixture"
    );
    assert!(enriched.context().is_owner());
    assert_eq!(enriched.context().depth(), 1);
    f.core().remember_automation_event(&enriched);
    assert_eq!(
        f.core()
            .automation_state
            .last_event
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .context()
            .depth(),
        1
    );
    assert_eq!(
        f.core().automation_context().0.unwrap()["intel"]["comment"]["normalized"],
        "normalized by fixture"
    );
}

#[tokio::test]
async fn denied_events_stop_before_plugins_domain_observers_context_memory_or_slots() {
    let f = Fixture::new();
    let core = f.core();
    assert_eq!(core.eligible_processors("tiktok.chat").len(), 1);
    let mut domain = core.events.subscribe_domain();
    let slots = core.automation_state.slots.available_permits();
    let event = AutomationEvent::new(payload(), ExecutionContext::denied());
    assert!(!core.publish_live_domain_event(&event));
    assert!(core.enrich_automation_event(event.clone()).await.is_err());
    core.publish_automation_event(event.clone()).await;
    core.queue_automation_event(event);
    tokio::task::yield_now().await;
    assert_eq!(f.calls.load(Ordering::Acquire), 0);
    assert!(domain.try_recv().is_err());
    assert!(core.plugin_last_events.lock().unwrap().is_empty());
    assert!(core.automation_context().0.is_none());
    assert_eq!(core.automation_state.slots.available_permits(), slots);
    assert!(core.processor_state.metrics.lock().unwrap().is_empty());
    assert!(core.automation_runs().is_empty());
    // The same host-created Owner input does reach the observer-visible bus;
    // the negative result above is authority admission, not an inert fixture.
    assert!(core.publish_live_domain_event(&AutomationEvent::owner(payload())));
    assert!(domain.try_recv().is_ok());
    assert!(!core.plugin_last_events.lock().unwrap().is_empty());
}

#[test]
fn nested_plugin_and_internal_event_builders_use_private_depth_and_keep_denied_context() {
    let f = Fixture::new();
    let core = f.core();
    let context = ExecutionContext::denied();
    let one = core
        .make_internal_automation_event(
            &payload(),
            "overlay.alert",
            json!({"depth":0,"authority":"owner"}),
            &context,
        )
        .unwrap();
    assert_eq!(one.context().depth(), 1);
    assert_eq!(one.payload()["data"]["depth"], 1);
    assert!(!one.context().is_owner());
    let two = core
        .make_plugin_event(
            "textintel",
            one.payload(),
            "fixture.changed",
            json!({"depth":0}),
            one.context(),
        )
        .unwrap();
    assert_eq!(two.context().depth(), 2);
    assert_eq!(two.payload()["data"]["depth"], 2);
    assert!(!two.context().is_owner());
    assert!(core
        .make_internal_automation_event(
            &json!({"data":{"depth":0}}),
            "overlay.alert",
            json!({}),
            two.context()
        )
        .is_err());
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn a_real_saved_subject_survives_payload_replacement_and_cannot_publish_as_owner() {
    use crate::collaboration::*;
    let f = Fixture::new();
    let core = f.core();
    let service = core.collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(config).unwrap();
    let invite = service
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    let exchange = "a".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Pipeline subject".into(),
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
    let subjects = service
        .execution_authorizations(&[(
            credentials.principal_id.clone(),
            credentials.session_id.clone(),
        )])
        .unwrap();
    let context = ExecutionContext::restricted(subjects).unwrap();
    let event = AutomationEvent::new(payload(), context).replace_payload(payload());
    let child = core
        .make_internal_automation_event(
            event.payload(),
            "overlay.alert",
            json!({}),
            event.context(),
        )
        .unwrap();
    assert!(child.context().check_current().is_ok());
    assert!(!child.context().is_owner());
    assert!(!core.publish_live_domain_event(&child));
    assert!(core.enrich_automation_event(child.clone()).await.is_err());
    core.publish_automation_event(child.clone()).await;
    assert_eq!(f.calls.load(Ordering::Acquire), 0);
    core.remember_automation_event(&child);
    assert!(core.automation_context().0.is_none());
    // Model a retained entry from a future scoped producer or an older
    // in-process generation. The private handle remains alongside the data;
    // legacy Owner getters must not expose or replay that payload as Owner.
    *core.automation_state.last_event.write().unwrap() = Some(child.clone());
    *core.automation_state.last_event_at.write().unwrap() = Some(42);
    core.automation_state
        .last_events
        .write()
        .unwrap()
        .insert("plugin.emit".into(), child.clone());
    core.automation_state
        .last_events_at
        .write()
        .unwrap()
        .insert("plugin.emit".into(), 42);
    assert_eq!(core.automation_context(), (None, None));
    assert_eq!(core.automation_context_for("plugin.emit"), (None, None));
    assert!(core.last_event_for("plugin.emit").is_none());
    assert!(!core
        .automation_state
        .last_event
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .context()
        .is_owner());
    service
        .revoke_session(&credentials.session_id, "local-owner")
        .unwrap();
    assert!(child.context().check_current().is_err());
    assert!(!child.context().is_owner());
    assert!(core
        .automation_state
        .last_event
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .context()
        .check_current()
        .is_err());
}

use super::*;
use std::path::PathBuf;

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

fn fixture() -> (AppCore, PathBuf) {
    let root = std::env::temp_dir().join(format!("tiktools-plugin-binding-{}", fastrand::u128(..)));
    let directory = root.join("packages/demo");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("entry.bin"), b"reviewed executable").unwrap();
    std::fs::write(
        directory.join("plugin.json"),
        json!({
            "schemaVersion":3,"id":"demo.bound","name":"Bound","version":"1.0.0",
            "runtime":"process","entry":"entry.bin", "permissions":["network.connect"],
            "actionTypes":[{"id":"demo.bound.send","fields":[
                {"key":"message","kind":"text","required":true},
                {"key":"count","kind":"number","min":1,"max":5},
                {"key":"mode","kind":"select","options":[{"value":"safe"}]}]}],
        "settings":{"schema":{"type":"object","properties":{
            "label":{"type":"string"},"token":{"type":"string","secret":true}}}}
        })
        .to_string(),
    )
    .unwrap();
    let paths = AppPaths {
        root: root.clone(),
        data: root.join("data"),
        plugins: root.join("packages"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
    };
    let core = AppCore::with_paths(Arc::new(Emitter), Arc::new(NoopMediaHost), paths);
    (core, root)
}

fn capture(core: &AppCore) -> TrustedPluginBinding {
    core.with_trusted_plugin_binding("demo.bound.send", |binding, _, _| Ok(binding))
        .unwrap()
}

#[test]
fn exact_artifact_settings_catalog_activation_and_restart_bindings_do_not_revive() {
    let (core, root) = fixture();
    let initial = capture(&core);
    assert_eq!(initial, capture(&core));
    core.plugin_settings_save(
        "demo.bound",
        BTreeMap::from([("label".into(), json!("same"))]),
    )
    .unwrap();
    let settings = capture(&core);
    assert_ne!(initial, settings);
    core.plugin_settings_save(
        "demo.bound",
        BTreeMap::from([("label".into(), json!("same"))]),
    )
    .unwrap();
    assert_ne!(settings, capture(&core));
    let before_file = capture(&core);
    std::fs::write(root.join("packages/demo/entry.bin"), b"changed executable").unwrap();
    let changed_file = capture(&core);
    assert_ne!(before_file.artifact_digest, changed_file.artifact_digest);
    std::fs::write(root.join("packages/demo/entry.bin"), b"reviewed executable").unwrap();
    core.plugins.scan().unwrap();
    assert_ne!(initial, capture(&core));
    core.plugin_set_installed("demo.bound", false).unwrap();
    assert!(core
        .with_trusted_plugin_binding("demo.bound.send", |_, _, _| Ok(()))
        .is_err());
    core.plugin_set_installed("demo.bound", true).unwrap();
    assert_ne!(initial, capture(&core));
    let before_restart = capture(&core);
    let broker = crate::services::CapabilityBroker::new(root.join("plugin-data"));
    let manifest = core.plugins.get("demo.bound").unwrap().manifest;
    let restarted = broker
        .with_plugin_settings_binding(&manifest, |binding, _| Ok(binding))
        .unwrap();
    assert_ne!(
        before_restart.settings.boot_identity,
        restarted.boot_identity
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn action_and_settings_scopes_are_explicit_conjunctive_and_schema_checked() {
    let (core, root) = fixture();
    core.with_trusted_plugin_binding("demo.bound.send", |binding, plugin, _| {
        let mut allowed = Grant::default();
        allowed.capabilities.insert(Capability::PluginActionExecute);
        assert!(binding.check_grants(&[&allowed]).is_err());
        allowed
            .actions
            .insert("demo.bound".into(), ["demo.bound.send".into()].into());
        assert!(binding.check_grants(&[&allowed]).is_ok());
        let denied = Grant::default();
        assert!(binding.check_grants(&[&allowed, &denied]).is_err());
        allowed.resources.insert(ResourceKind::Plugin, [].into());
        assert!(binding.check_grants(&[&allowed]).is_err());
        allowed.resources.clear();
        assert!(binding
            .project_arguments(&json!({"message":"hello","count":2,"mode":"safe"}))
            .is_ok());
        for invalid in [
            json!({"message":"hello","owner":true}),
            json!({"message":{}}),
            json!({"message":"hello","count":6}),
            json!({"message":"hello","mode":"unsafe"}),
            json!({}),
        ] {
            assert!(binding.project_arguments(&invalid).is_err());
        }
        let settings = json!({"label":"visible","token":"owner-secret","unknown":"hidden"});
        assert_eq!(
            binding.project_settings(&plugin.manifest, &settings, &[&allowed]),
            json!({})
        );
        allowed.capabilities.insert(Capability::PluginSettingsRead);
        allowed
            .settings_fields
            .insert("demo.bound".into(), ["label".into()].into());
        assert_eq!(
            binding.project_settings(&plugin.manifest, &settings, &[&allowed]),
            json!({"label":"visible"})
        );
        let mut narrower = allowed.clone();
        narrower
            .settings_fields
            .get_mut("demo.bound")
            .unwrap()
            .clear();
        assert_eq!(
            binding.project_settings(&plugin.manifest, &settings, &[&allowed, &narrower]),
            json!({})
        );
        allowed
            .settings_fields
            .get_mut("demo.bound")
            .unwrap()
            .insert("token".into());
        assert_eq!(
            binding.project_settings(&plugin.manifest, &settings, &[&allowed]),
            json!({"label":"visible","token":"owner-secret"})
        );
        Ok(())
    })
    .unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn lifecycle_writer_waits_until_final_binding_callback_exits() {
    let (core, root) = fixture();
    let core = Arc::new(core);
    let (started, start_rx) = std::sync::mpsc::channel();
    let (finished, finish_rx) = std::sync::mpsc::channel();
    let mut worker = None;
    core.with_trusted_plugin_binding("demo.bound.send", |_, _, _| {
        let writer = Arc::clone(&core);
        worker = Some(std::thread::spawn(move || {
            started.send(()).unwrap();
            writer.plugin_set_installed("demo.bound", false).unwrap();
            finished.send(()).unwrap();
        }));
        start_rx.recv().unwrap();
        assert!(finish_rx.try_recv().is_err());
        Ok(())
    })
    .unwrap();
    worker.unwrap().join().unwrap();
    finish_rx.recv().unwrap();
    let _ = std::fs::remove_dir_all(root);
}

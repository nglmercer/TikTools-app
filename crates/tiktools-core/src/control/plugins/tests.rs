use super::ui::is_confined_ui_entry;
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{collections::BTreeMap, sync::Arc};
use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};

struct NullEmitter;

impl crate::HostEmitter for NullEmitter {
    fn emit(&self, _message: crate::HostMessage) {}
}

fn manifest(id: &str, actions: &[&str]) -> String {
    let actions = actions
        .iter()
        .map(|action| {
            format!(
                r#"{{"id": "{action}", "title": {{"default": "{action}"}}, "tag": "test", "requiredCapabilities": []}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schemaVersion": 3, "id": "{id}", "name": "{id}", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": [], "actionTypes": [{actions}]}}"#
    )
}

/// Builds an AppCore whose catalog holds two declarative plugins. The
/// actions declare no `http` block, so dry runs resolve and return
/// without touching any runtime or network.
fn core_with_two_plugins() -> (Arc<AppCore>, std::path::PathBuf) {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "tiktools-ownership-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    // plugin-b declares `plugina.spoof`: the action NAME looks like
    // plugin-a's, but the catalog owner is plugin-b. Ownership must
    // resolve through the catalog, never by prefix.
    for (dir, id, actions) in [
        ("plugin-a", "plugina", &["plugina.echo"][..]),
        (
            "plugin-b",
            "pluginb",
            &["pluginb.echo", "plugina.spoof"][..],
        ),
    ] {
        let dir = root.join(dir);
        std::fs::create_dir_all(&dir).expect("test plugin dir");
        std::fs::write(dir.join("plugin.json"), manifest(id, actions)).expect("manifest");
    }
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().expect("test scan");
    assert_eq!(manager.list().len(), 2);
    let mut core = AppCore::new(Arc::new(NullEmitter));
    core.plugins = Arc::new(manager);
    (Arc::new(core), root)
}

fn cleanup(root: &std::path::Path) {
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn scoped_execute_allows_own_actions() {
    let (core, root) = core_with_two_plugins();
    let outcome = core
        .plugin_action_execute_scoped("plugina", "plugina.echo", BTreeMap::new(), false)
        .await
        .expect("own action executes");
    assert!(outcome.ok, "dry run should succeed: {outcome:?}");
    cleanup(&root);
}

#[tokio::test]
async fn scoped_execute_rejects_cross_plugin_actions() {
    let (core, root) = core_with_two_plugins();
    // plugin-a cannot invoke plugin-b's action.
    let error = core
        .plugin_action_execute_scoped("plugina", "pluginb.echo", BTreeMap::new(), false)
        .await
        .expect_err("cross-plugin execute must fail");
    assert_eq!(error.code(), "forbidden");
    // Prefix spoofing buys nothing: `plugina.spoof` is owned by
    // plugin-b even though the name starts with plugin-a's id.
    let error = core
        .plugin_action_execute_scoped("plugina", "plugina.spoof", BTreeMap::new(), false)
        .await
        .expect_err("prefix-spoofed execute must fail");
    assert_eq!(error.code(), "forbidden");
    // Unknown actions report not_found, not forbidden.
    let error = core
        .plugin_action_execute_scoped("plugina", "nobody.there", BTreeMap::new(), false)
        .await
        .expect_err("unknown action must fail");
    assert_eq!(error.code(), "not_found");
    cleanup(&root);
}

#[tokio::test]
async fn scoped_options_rejects_cross_plugin_sources() {
    let (core, root) = core_with_two_plugins();
    // plugin-a cannot read plugin-b's option source.
    let error = core
        .plugin_action_options_scoped("plugina", "plugin-action-options:pluginb.echo:field", false)
        .await
        .expect_err("cross-plugin options read must fail");
    assert_eq!(error.code(), "forbidden");
    // Prefix-spoofed source resolves to plugin-b and is rejected.
    let error = core
        .plugin_action_options_scoped(
            "plugina",
            "plugin-action-options:plugina.spoof:field",
            false,
        )
        .await
        .expect_err("prefix-spoofed options read must fail");
    assert_eq!(error.code(), "forbidden");
    // Own source passes ownership: it proceeds to resolution, which
    // reports the (declared-shape) absence of options — never forbidden.
    let error = core
        .plugin_action_options_scoped("plugina", "plugin-action-options:plugina.echo:field", false)
        .await
        .expect_err("undeclared options must fail resolution");
    assert_ne!(error.code(), "forbidden");
    cleanup(&root);
}

/// Stages the real Minecraft example bytes (not an inline fixture)
/// so discovery runs against exactly what ships in `examples/`.
fn core_with_minecraft_example() -> (Arc<AppCore>, std::path::PathBuf) {
    static COUNTER: AtomicU64 = AtomicU64::new(2000);
    let root = std::env::temp_dir().join(format!(
        "tiktools-minecraft-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let dir = root.join("minecraft.server");
    std::fs::create_dir_all(&dir).expect("test plugin dir");
    std::fs::write(
        dir.join("plugin.json"),
        include_str!("../../../../../tests/fixtures/plugins/minecraft-server.plugin.json"),
    )
    .expect("manifest");
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().expect("test scan");
    assert_eq!(manager.list().len(), 1);
    let mut core = AppCore::new(Arc::new(NullEmitter));
    core.plugins = Arc::new(manager);
    (Arc::new(core), root)
}

#[test]
fn minecraft_example_merges_into_behavior_snapshot() {
    let (core, root) = core_with_minecraft_example();
    let snapshot = core.load_merged_behavior_snapshot();
    let action_types = snapshot
        .get("actionTypes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let minecraft: Vec<&Value> = action_types
        .iter()
        .filter(|descriptor| {
            descriptor
                .get("source")
                .and_then(|source| source.get("pluginId"))
                .and_then(Value::as_str)
                == Some("minecraft.server")
        })
        .collect();
    // The six structured actions surface as picker cards with their
    // presentation metadata and plugin origin stamp intact.
    assert_eq!(minecraft.len(), 6);
    for descriptor in minecraft {
        assert_eq!(
            descriptor.get("source"),
            Some(&serde_json::json!({"kind": "plugin", "pluginId": "minecraft.server"}))
        );
        assert!(descriptor
            .get("icon")
            .and_then(Value::as_str)
            .is_some_and(|icon| !icon.is_empty()));
        assert!(descriptor
            .get("category")
            .and_then(Value::as_str)
            .is_some_and(|category| !category.is_empty()));
        assert!(descriptor
            .get("fields")
            .and_then(Value::as_array)
            .is_some_and(|fields| !fields.is_empty()));
    }
    cleanup(&root);
}

#[tokio::test]
async fn scoped_options_allow_host_sources_for_any_plugin() {
    let (core, root) = core_with_two_plugins();
    // Host-owned data belongs to no plugin: either caller may read it.
    for requesting in ["plugina", "pluginb"] {
        let (options, selected) = core
            .plugin_action_options_scoped(requesting, "host.gifts", false)
            .await
            .expect("host source must resolve for any plugin");
        assert!(selected.is_none());
        assert_eq!(
            options,
            crate::services::option_sources::map_host_gift_options(&core.gift_catalog())
        );
    }
    cleanup(&root);
}

fn webview_manifest(id: &str) -> String {
    format!(
        r#"{{"schemaVersion": 3, "id": "{id}", "name": "{id}", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": [], "actionTypes": [], "ui": {{"apiVersion": 1, "mode": "webview", "entry": "ui/dist/index.html", "pages": [{{"id": "main", "title": {{"default": "Main"}}}}]}}}}"#
    )
}

fn core_with_webview_plugin() -> (Arc<AppCore>, std::path::PathBuf) {
    static COUNTER: AtomicU64 = AtomicU64::new(1000);
    let root = std::env::temp_dir().join(format!(
        "tiktools-uitarget-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let dir = root.join("webui");
    std::fs::create_dir_all(dir.join("ui/dist")).expect("test ui dir");
    std::fs::write(dir.join("plugin.json"), webview_manifest("webui")).expect("manifest");
    std::fs::write(dir.join("ui/dist/index.html"), "<html></html>").expect("entry");
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().expect("test scan");
    let mut core = AppCore::new(Arc::new(NullEmitter));
    core.plugins = Arc::new(manager);
    (Arc::new(core), root)
}

#[test]
fn ui_target_resolves_entry_and_confines_asset_root() {
    let (core, root) = core_with_webview_plugin();
    let target = core.plugin_ui_target("webui", "main").expect("ui target");
    assert_eq!(target.id, "webui");
    assert_eq!(target.asset_root, root.join("webui/ui/dist"));
    // Unknown page and unknown plugin fail; nothing resolves.
    assert_eq!(
        core.plugin_ui_target("webui", "nope")
            .expect_err("unknown page must fail")
            .code(),
        "not_found"
    );
    assert_eq!(
        core.plugin_ui_target("missing", "main")
            .expect_err("unknown plugin must fail")
            .code(),
        "not_found"
    );
    // An unbuilt UI (entry asset absent) is unavailable, not servable.
    std::fs::remove_file(root.join("webui/ui/dist/index.html")).expect("remove entry");
    assert_eq!(
        core.plugin_ui_target("webui", "main")
            .expect_err("missing entry must fail")
            .code(),
        "unavailable"
    );
    cleanup(&root);
}

#[test]
fn ui_assets_resolve_without_selecting_a_page() {
    let (core, root) = core_with_webview_plugin();
    let assets = core.plugin_ui_assets("webui").expect("ui assets");
    assert_eq!(assets.asset_root, root.join("webui/ui/dist"));
    assert_eq!(assets.entry_file, "index.html");
    assert_eq!(assets.page_ids, vec!["main".to_owned()]);
    // The shared asset server rejects declarative plugins before
    // touching the filesystem.
    let (legacy, legacy_root) = core_with_two_plugins();
    assert_eq!(
        legacy
            .plugin_ui_assets("plugina")
            .expect_err("declarative must fail")
            .code(),
        "unavailable"
    );
    cleanup(&root);
    cleanup(&legacy_root);
}

#[test]
fn ui_target_rejects_declarative_plugins() {
    let (core, root) = core_with_two_plugins();
    let error = core
        .plugin_ui_target("plugina", "main")
        .expect_err("declarative plugin must fail");
    assert_eq!(error.code(), "unavailable");
    cleanup(&root);
}

#[test]
fn confined_ui_entry_blocks_escape() {
    assert!(is_confined_ui_entry("ui/dist/index.html"));
    assert!(!is_confined_ui_entry("../plugin.json"));
    assert!(!is_confined_ui_entry("ui/../../secret.html"));
    assert!(!is_confined_ui_entry("/etc/ui.html"));
    assert!(!is_confined_ui_entry("ui/dist/app.js"));
    assert!(!is_confined_ui_entry("ui\\dist\\index.html"));
    assert!(!is_confined_ui_entry("assets/index.html"));
}

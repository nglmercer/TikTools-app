use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;
use crate::paths::AppPaths;
use serde_json::json;

fn paths() -> AppPaths {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tiktools-rust-sqlite-{suffix}"));
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}

#[test]
fn initializes_compatible_schemas_and_round_trips_records() {
    let paths = paths();
    paths.ensure_directories().unwrap();
    let database = DatabaseManager::new(paths.clone());
    let config = database.load_points_config().unwrap();
    assert_eq!(config.currency_name, "Points");

    let graph = json!({
        "schemaVersion": 1,
        "id": "workflow-1",
        "name": "Demo",
        "enabled": false,
        "nodes": [],
        "edges": []
    });
    let saved = database.save_workflow(&graph).unwrap();
    assert_eq!(saved["id"], "workflow-1");
    assert_eq!(database.load_workflows().unwrap().len(), 1);

    let action = json!({"id":"action-1","name":"Action","enabled":true,"type":"code"});
    database.save_behavior("behavior_actions", &action).unwrap();
    let snapshot = database.load_behavior_snapshot().unwrap();
    assert_eq!(snapshot["actions"][0]["id"], "action-1");

    let _ = fs::remove_dir_all(paths.root);
}

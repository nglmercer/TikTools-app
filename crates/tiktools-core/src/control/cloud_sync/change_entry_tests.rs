use super::*;
use cloud_client::{
    WorkspaceEdit, WorkspaceEntry, WorkspaceKey, WorkspaceLocalEdit, WorkspacePending,
    WorkspacePendingStatus, WorkspaceRecord,
};
use serde_json::json;

fn record(revision: i64, value: serde_json::Value) -> WorkspaceRecord {
    WorkspaceRecord {
        collection: "plugin-selections".into(),
        key: "alpha".into(),
        revision,
        deleted: false,
        value: Some(value),
    }
}

fn entry() -> WorkspaceEntry {
    WorkspaceEntry {
        target: WorkspaceKey::new("plugin-selections", "alpha").unwrap(),
        remote: None,
        pending: None,
        next: None,
    }
}

#[test]
fn entry_without_pending_edit_reports_the_cached_cloud_record() {
    let mut item = entry();
    item.remote = Some(record(4, json!({"version": "1.0.0", "enabled": true})));
    let projected = v2_change_entry(&item);
    assert_eq!(projected.collection, "plugin-selections");
    assert_eq!(projected.key, "alpha");
    assert_eq!(projected.remote.state, "present");
    assert_eq!(projected.remote.revision, Some(4));
    assert_eq!(projected.remote.version.as_deref(), Some("1.0.0"));
    assert_eq!(projected.remote.enabled, Some(true));
    assert!(projected.pending.is_none());
    assert!(!projected.has_newer);
}

#[test]
fn absent_remote_and_pending_edit_keep_base_and_local_apart() {
    let mut item = entry();
    let base = record(2, json!({"version": "1.0.0", "enabled": true}));
    item.remote = Some(base.clone());
    item.pending = Some(WorkspacePending {
        base_revision: Some(2),
        edit: WorkspaceEdit::Put(json!({"version": "1.0.0", "enabled": false})),
        local_generation: 3,
        base_record: Some(base),
        status: WorkspacePendingStatus::Conflict,
        write_epoch: None,
    });
    item.next = Some(WorkspaceLocalEdit {
        edit: WorkspaceEdit::Delete,
        local_generation: 4,
    });
    let projected = v2_change_entry(&item);
    let pending = projected.pending.expect("pending change");
    assert_eq!(pending.status, "conflict");
    assert_eq!(pending.base.enabled, Some(true));
    assert_eq!(pending.local.enabled, Some(false));
    assert!(projected.has_newer);

    let mut fresh = entry();
    fresh.pending = Some(WorkspacePending {
        base_revision: None,
        edit: WorkspaceEdit::Delete,
        local_generation: 1,
        base_record: None,
        status: WorkspacePendingStatus::Queued,
        write_epoch: None,
    });
    let projected = v2_change_entry(&fresh);
    assert_eq!(projected.remote.state, "absent");
    let pending = projected.pending.unwrap();
    assert_eq!(pending.status, "queued");
    assert_eq!(pending.base.state, "absent");
    assert_eq!(pending.local.state, "deleted");
}
fn settings_record(revision: i64, settings: serde_json::Value) -> WorkspaceRecord {
    WorkspaceRecord {
        collection: "plugin-settings".into(),
        key: "alpha".into(),
        revision,
        deleted: false,
        value: Some(json!({"schemaVersion": 1, "pluginId": "alpha", "settings": settings})),
    }
}

fn settings_entry(
    base: Option<WorkspaceRecord>,
    edit: WorkspaceEdit,
    status: WorkspacePendingStatus,
) -> WorkspaceEntry {
    WorkspaceEntry {
        target: WorkspaceKey::new("plugin-settings", "alpha").unwrap(),
        remote: base.clone(),
        pending: Some(WorkspacePending {
            base_revision: base.as_ref().map(|record| record.revision),
            edit,
            local_generation: 2,
            base_record: base,
            status,
            write_epoch: None,
        }),
        next: Some(WorkspaceLocalEdit {
            edit: WorkspaceEdit::Put(json!({
                "schemaVersion": 1,
                "pluginId": "alpha",
                "settings": {"apiKey": "NEXT-SECRET-VALUE"}
            })),
            local_generation: 3,
        }),
    }
}

#[test]
fn plugin_settings_expose_only_changed_key_names_never_values() {
    let base = settings_record(
        5,
        json!({"apiKey": "OLD-SECRET-VALUE", "volume": 3, "gone": {"nested": "REMOVED-VALUE"}, "same": "SAME-VALUE"}),
    );
    let edit = WorkspaceEdit::Put(json!({
        "schemaVersion": 1,
        "pluginId": "alpha",
        "settings": {"apiKey": "NEW-SECRET-VALUE", "volume": 3, "same": "SAME-VALUE", "added": ["ADDED-VALUE"]}
    }));
    let item = settings_entry(Some(base), edit, WorkspacePendingStatus::Conflict);
    let projected = v2_change_entry(&item);
    let pending = projected.pending.as_ref().expect("pending change");
    assert_eq!(
        pending.changed_keys.as_deref(),
        Some(&["added".to_owned(), "apiKey".to_owned(), "gone".to_owned()][..])
    );
    assert_eq!(pending.status, "conflict");
    assert_eq!(pending.base.state, "present");
    assert_eq!(pending.base.revision, Some(5));
    assert_eq!(pending.local.state, "present");
    assert_eq!(projected.remote.revision, Some(5));
    assert!(projected.has_newer);
    let text = serde_json::to_string(&projected).unwrap();
    for secret in [
        "OLD-SECRET-VALUE",
        "NEW-SECRET-VALUE",
        "NEXT-SECRET-VALUE",
        "REMOVED-VALUE",
        "SAME-VALUE",
        "ADDED-VALUE",
        "nested",
    ] {
        assert!(!text.contains(secret), "{secret} leaked into {text}");
    }
    assert!(text.contains("apiKey"));
}

#[test]
fn plugin_settings_creation_and_deletion_list_every_key_without_values() {
    let created = settings_entry(
        None,
        WorkspaceEdit::Put(json!({
            "schemaVersion": 1,
            "pluginId": "alpha",
            "settings": {"token": "CREATED-SECRET", "mode": "fast"}
        })),
        WorkspacePendingStatus::Queued,
    );
    let projected = v2_change_entry(&created);
    let pending = projected.pending.as_ref().unwrap();
    assert_eq!(pending.base.state, "absent");
    assert_eq!(
        pending.changed_keys.as_deref(),
        Some(&["mode".to_owned(), "token".to_owned()][..])
    );
    assert!(!serde_json::to_string(&projected)
        .unwrap()
        .contains("CREATED-SECRET"));

    let deleted = settings_entry(
        Some(settings_record(2, json!({"token": "DELETED-SECRET"}))),
        WorkspaceEdit::Delete,
        WorkspacePendingStatus::Queued,
    );
    let projected = v2_change_entry(&deleted);
    let pending = projected.pending.as_ref().unwrap();
    assert_eq!(pending.local.state, "deleted");
    assert_eq!(
        pending.changed_keys.as_deref(),
        Some(&["token".to_owned()][..])
    );
    assert!(!serde_json::to_string(&projected)
        .unwrap()
        .contains("DELETED-SECRET"));
}

#[test]
fn selections_keep_their_values_and_have_no_changed_keys() {
    let mut item = entry();
    item.pending = Some(WorkspacePending {
        base_revision: None,
        edit: WorkspaceEdit::Put(json!({"version": "2.0.0", "enabled": true})),
        local_generation: 1,
        base_record: None,
        status: WorkspacePendingStatus::Queued,
        write_epoch: None,
    });
    let pending = v2_change_entry(&item).pending.unwrap();
    assert_eq!(pending.changed_keys, None);
    assert_eq!(pending.local.version.as_deref(), Some("2.0.0"));
    assert!(pending.local.value.is_some());
}

#[test]
fn unknown_collections_are_reduced_to_metadata() {
    let mut item = entry();
    item.target = WorkspaceKey::new("profiles", "p1").unwrap();
    item.remote = Some(WorkspaceRecord {
        collection: "profiles".into(),
        key: "p1".into(),
        revision: 9,
        deleted: false,
        value: Some(json!({"name": "PROFILE-NAME"})),
    });
    let projected = v2_change_entry(&item);
    assert_eq!(projected.remote.revision, Some(9));
    assert!(!serde_json::to_string(&projected)
        .unwrap()
        .contains("PROFILE-NAME"));
}

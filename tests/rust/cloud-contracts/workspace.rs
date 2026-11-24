use cloud_contracts::{WorkspaceChanges, WorkspaceRecord, WorkspaceSnapshot};

#[test]
fn snapshots_and_changes_refuse_duplicates_and_invalid_revisions() {
    let record = WorkspaceRecord {
        collection: "preferences".into(),
        key: "desktop".into(),
        revision: 1,
        deleted: false,
        value: Some(serde_json::json!({"schemaVersion":1,"theme":"dark"})),
    };
    let mut snapshot = WorkspaceSnapshot {
        schema_version: 2,
        cursor: 1,
        records: vec![record.clone()],
    };
    assert!(snapshot.valid());
    snapshot.records.push(record.clone());
    assert!(!snapshot.valid());
    let mut page = WorkspaceChanges {
        changes: vec![record],
        cursor: 1,
        has_more: false,
    };
    assert!(page.valid_after(0));
    assert!(!page.valid_after(1));
    page.cursor = 2;
    assert!(!page.valid_after(0));
}

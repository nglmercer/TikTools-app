//! Allowlisted UI projections: plugin-setting values remain host-only.
use super::{V2ChangeEntry, V2ConflictValue, V2PendingChange};

/// Collections whose records are credential-free by schema and may be shown.
/// `variables` is credential-free by contract (secrets are excluded in the
/// host before staging), so its values are projected like preferences.
const VALUE_VISIBLE_COLLECTIONS: &[&str] = &["plugin-selections", "preferences", "variables"];

/// Keeps the state and revision of a value and drops everything else.
fn v2_metadata_only(mut value: V2ConflictValue) -> V2ConflictValue {
    value.value = None;
    value.version = None;
    value.enabled = None;
    value
}

fn settings_map(value: &serde_json::Value) -> Option<&serde_json::Map<String, serde_json::Value>> {
    value.get("settings").and_then(serde_json::Value::as_object)
}

/// Sorted top-level names of the plugin settings that differ between the base
/// record and the pending edit. Only names are returned, never values.
fn changed_setting_keys(
    base: Option<&cloud_client::WorkspaceRecord>,
    edit: &cloud_client::WorkspaceEdit,
) -> Vec<String> {
    let before = base
        .filter(|record| !record.deleted)
        .and_then(|record| record.value.as_ref())
        .and_then(settings_map);
    let after = match edit {
        cloud_client::WorkspaceEdit::Put(value) => settings_map(value),
        cloud_client::WorkspaceEdit::Delete => None,
    };
    let mut names = std::collections::BTreeSet::new();
    for map in [before, after].into_iter().flatten() {
        names.extend(map.keys().cloned());
    }
    names
        .into_iter()
        .filter(|name| before.and_then(|map| map.get(name)) != after.and_then(|map| map.get(name)))
        .collect()
}

pub(crate) fn v2_change_entry(entry: &cloud_client::WorkspaceEntry) -> V2ChangeEntry {
    use cloud_client::WorkspacePendingStatus;
    let collection = entry.target.collection.as_str();
    let visible = VALUE_VISIBLE_COLLECTIONS.contains(&collection);
    let project = |value: V2ConflictValue| {
        if visible {
            value
        } else {
            v2_metadata_only(value)
        }
    };
    V2ChangeEntry {
        collection: entry.target.collection.clone(),
        key: entry.target.key.clone(),
        remote: project(v2_value_from_record(entry.remote.as_ref())),
        pending: entry.pending.as_ref().map(|pending| V2PendingChange {
            status: match pending.status {
                WorkspacePendingStatus::Queued => "queued",
                WorkspacePendingStatus::Unconfirmed => "unconfirmed",
                WorkspacePendingStatus::Conflict => "conflict",
            }
            .into(),
            base: project(v2_value_from_record(pending.base_record.as_ref())),
            local: project(v2_value_from_edit(&pending.edit)),
            changed_keys: (collection == "plugin-settings")
                .then(|| changed_setting_keys(pending.base_record.as_ref(), &pending.edit)),
        }),
        has_newer: entry.next.is_some(),
    }
}

pub(super) fn v2_value_from_edit(edit: &cloud_client::WorkspaceEdit) -> V2ConflictValue {
    match edit {
        cloud_client::WorkspaceEdit::Delete => V2ConflictValue {
            state: "deleted".into(),
            revision: None,
            value: None,
            version: None,
            enabled: None,
        },
        cloud_client::WorkspaceEdit::Put(value) => selection_projection(value),
    }
}

pub(super) fn v2_value_from_record(
    record: Option<&cloud_client::WorkspaceRecord>,
) -> V2ConflictValue {
    match record {
        None => V2ConflictValue {
            state: "absent".into(),
            revision: None,
            value: None,
            version: None,
            enabled: None,
        },
        Some(record) if record.deleted => V2ConflictValue {
            state: "deleted".into(),
            revision: Some(record.revision),
            value: None,
            version: None,
            enabled: None,
        },
        Some(record) => {
            let mut projected =
                record
                    .value
                    .as_ref()
                    .map(selection_projection)
                    .unwrap_or(V2ConflictValue {
                        state: "present".into(),
                        revision: Some(record.revision),
                        value: None,
                        version: None,
                        enabled: None,
                    });
            projected.revision = Some(record.revision);
            projected
        }
    }
}

fn selection_projection(value: &serde_json::Value) -> V2ConflictValue {
    let version = value
        .get("version")
        .and_then(|item| item.as_str())
        .map(str::to_owned);
    let enabled = value.get("enabled").and_then(|item| item.as_bool());
    V2ConflictValue {
        state: "present".into(),
        revision: None,
        value: Some(value.clone()),
        version,
        enabled,
    }
}

//! Cloud sync for the automation library, record by record.
//!
//! Every action and event is one record with its own digest. A sync pass compares
//! the local library, the Cloud snapshot and the digests stored at the last sync:
//!
//! - a record changed on one side only takes that side's version;
//! - a record changed on both sides with different content is a conflict: the UI
//!   version wins when this host runs as the UI, the remote version otherwise;
//! - records the other side did not touch are kept, so one side's change never
//!   erases an unrelated change made on the other side.
//!
//! The merged library is published as one snapshot, and local changes are written
//! through the versioned record operations. Only the sync base (digests, revision,
//! cursor) is stored under [`LIBRARY_SYNC_BASE_KEY`].
//!
//! The merge is a pure function tested without I/O. Only
//! [`AppCore::automation_library_sync_now`] performs I/O.
//!
//! Limitation: profiles and templates (the catalog) are not merged yet. A pass
//! publishes the local catalog.

#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
use super::owner_library::{owner_library_digest, OwnerLibraryCatalog, OwnerLibraryPack};
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
use super::AutomationKind;
use super::OperationError;
use crate::AppCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
use std::collections::BTreeSet;

/// App state key holding the JSON-encoded [`LibrarySyncBase`].
pub const LIBRARY_SYNC_BASE_KEY: &str = "sync.automationLibrary.base";
/// Domain event emitted after a library change has been synced.
pub const AUTOMATION_LIBRARY_CHANGED_EVENT: &str = "automation-library-changed";

/// How a sync pass moved the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibrarySyncAction {
    /// Nothing changed on either side.
    Nothing,
    /// Local changes were published; nothing came from the Cloud.
    PushLocal,
    /// Cloud changes were written locally (possibly with local changes published too).
    ApplyRemote,
    /// Conflicting records were resolved in favour of the UI version.
    PushLocalWinsConflict,
    /// Conflicting records were resolved in favour of the Cloud version; the local edit was discarded.
    ApplyRemoteWinsConflict,
}

/// Value returned by one sync pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySyncResult {
    pub action: LibrarySyncAction,
    pub revision: String,
    pub conflicts: usize,
    /// Why the pass was refused, when it was. Nothing is written or published in that
    /// case, and the sync base does not advance, so the next pass tries again.
    pub rejection: Option<String>,
}

/// Durable sync base: the state both sides agreed on at the last sync.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibrarySyncBase {
    /// Local write counter of the automation library when this base was taken. A
    /// match with the current counter and the Cloud token skips the merge entirely.
    #[serde(default)]
    pub local_version: Option<i64>,
    /// Cloud record token the base was taken from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Cloud cursor used to read subsequent changes.
    #[serde(default)]
    pub cursor: i64,
    /// Digest of every record as both sides held it at the last sync, keyed
    /// `action:<id>`, `event:<id>`, `profile:<id>` or `template:<id>`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub records: BTreeMap<String, String>,
    /// Content of every profile at the last sync. A three-way merge of profile
    /// membership needs what each side started from, not only its digest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, Value>,
}

/// Feature-independent view of the Cloud library record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibrarySyncRecord {
    #[serde(default)]
    pub schema_version: u32,
    pub origin: String,
    pub revision: String,
    pub snapshot: Value,
}

#[cfg(feature = "cloud-auth")]
impl From<cloud_client::LibraryRecord> for LibrarySyncRecord {
    fn from(record: cloud_client::LibraryRecord) -> Self {
        Self {
            schema_version: record.schema_version,
            origin: record.origin,
            revision: record.revision,
            snapshot: record.snapshot,
        }
    }
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
fn cloud_error(error: cloud_client::SyncError) -> OperationError {
    use cloud_client::SyncError;
    let message = error.to_string();
    match error {
        SyncError::Configuration | SyncError::Protocol => OperationError::invalid(message),
        SyncError::Unauthorized | SyncError::WrongAccount => OperationError::unavailable(message),
        SyncError::Conflict | SyncError::CursorExpired => OperationError::conflict(message),
        SyncError::RateLimited { .. } | SyncError::TooManyPages => OperationError::timeout(message),
        _ => OperationError::unavailable(message),
    }
}

/// Record key shared by the base, the merge and the published snapshot.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn record_key(kind: AutomationKind, id: &str) -> String {
    let family = match kind {
        AutomationKind::Action => "action",
        AutomationKind::Event => "event",
    };
    format!("{family}:{id}")
}

/// Splits a record key back into its kind and id.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
fn split_key(key: &str) -> Option<(AutomationKind, &str)> {
    if let Some(id) = key.strip_prefix("action:") {
        Some((AutomationKind::Action, id))
    } else {
        key.strip_prefix("event:")
            .map(|id| (AutomationKind::Event, id))
    }
}

/// Records of a library definitions object, keyed by [`record_key`]. The
/// definitions must list both actions and events; anything else is refused,
/// because reading a malformed snapshot as empty would delete local records.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn record_map(definitions: &Value) -> Result<BTreeMap<String, Value>, OperationError> {
    let mut records = BTreeMap::new();
    for (kind, list) in [
        (AutomationKind::Action, "actions"),
        (AutomationKind::Event, "events"),
    ] {
        let items = definitions
            .get(list)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                OperationError::invalid("library definitions must list actions and events")
            })?;
        for item in items {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| OperationError::invalid("library record has no id"))?;
            if records.insert(record_key(kind, id), item.clone()).is_some() {
                return Err(OperationError::invalid(
                    "library contains a duplicate record id",
                ));
            }
        }
    }
    Ok(records)
}

/// Remote records of a Cloud snapshot.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn remote_records(snapshot: &Value) -> Result<BTreeMap<String, Value>, OperationError> {
    let definitions = snapshot.get("definitions").ok_or_else(|| {
        OperationError::invalid("remote automation library snapshot has no definitions")
    })?;
    record_map(definitions)
}

/// Base key of the active profile id. Profiles are `profile:<id>`, templates `template:<id>`.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
const ACTIVE_KEY: &str = "active";

/// Whether a merge key is an automation record (an action or an event).
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn is_record_key(key: &str) -> bool {
    key.starts_with("action:") || key.starts_with("event:")
}

/// Catalog part of a sync: profiles, templates and the active profile id.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn is_catalog_key(key: &str) -> bool {
    !is_record_key(key)
}

/// Catalog state keyed for the merge: one entry per profile and template, and the active id.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn catalog_map(catalog: &OwnerLibraryCatalog) -> Result<BTreeMap<String, Value>, OperationError> {
    let mut map = BTreeMap::new();
    for pack in &catalog.packs {
        let value = serde_json::to_value(pack)
            .map_err(|_| OperationError::internal("automation library profile is unavailable"))?;
        if map.insert(format!("profile:{}", pack.id), value).is_some() {
            return Err(OperationError::invalid(
                "catalog contains a duplicate profile id",
            ));
        }
    }
    for template in &catalog.templates {
        let id = template
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| OperationError::invalid("catalog template has no id"))?;
        if map
            .insert(format!("template:{id}"), template.clone())
            .is_some()
        {
            return Err(OperationError::invalid(
                "catalog contains a duplicate template id",
            ));
        }
    }
    map.insert(
        ACTIVE_KEY.to_owned(),
        Value::String(catalog.active_id.clone()),
    );
    Ok(map)
}

/// The merged catalog back in its own shape. A profile that the merge removed while the
/// active id still points at it falls back to the default profile.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn merged_catalog(merged: &BTreeMap<String, Value>) -> Result<OwnerLibraryCatalog, OperationError> {
    let mut packs = Vec::new();
    let mut templates = Vec::new();
    let mut active = "default".to_owned();
    for (key, value) in merged {
        if key.starts_with("profile:") {
            packs.push(
                serde_json::from_value::<OwnerLibraryPack>(value.clone())
                    .map_err(|_| OperationError::invalid("library profile is malformed"))?,
            );
        } else if key.starts_with("template:") {
            templates.push(value.clone());
        } else if key == ACTIVE_KEY {
            active = value
                .as_str()
                .ok_or_else(|| OperationError::invalid("active library profile is malformed"))?
                .to_owned();
        }
    }
    if active != "default" && !packs.iter().any(|pack| pack.id == active) {
        active = "default".to_owned();
    }
    Ok(OwnerLibraryCatalog {
        packs,
        active_id: active,
        templates,
    })
}

/// Catalog of a Cloud snapshot, keyed for the merge. A snapshot without a catalog is refused.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn remote_catalog_map(snapshot: &Value) -> Result<BTreeMap<String, Value>, OperationError> {
    let catalog = snapshot.get("catalog").ok_or_else(|| {
        OperationError::invalid("remote automation library snapshot has no catalog")
    })?;
    let catalog: OwnerLibraryCatalog = serde_json::from_value(catalog.clone())
        .map_err(|_| OperationError::invalid("remote automation library catalog is malformed"))?;
    catalog_map(&catalog)
}

/// Everything a Cloud snapshot contributes to the merge: records and catalog.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn remote_state(snapshot: &Value) -> Result<BTreeMap<String, Value>, OperationError> {
    let mut state = remote_records(snapshot)?;
    state.extend(remote_catalog_map(snapshot)?);
    Ok(state)
}

/// The outcome of merging the two sides against the base, decided without I/O.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
#[derive(Debug, Default, PartialEq, Eq)]
struct LibraryMerge {
    /// The merged state of every record that survives. Removed records are absent.
    merged: BTreeMap<String, Value>,
    /// Records whose local state must change: `Some` to create or update, `None` to remove.
    local_changes: BTreeMap<String, Option<Value>>,
    /// Whether the merged library differs from the Cloud record and must be published.
    publish: bool,
    /// Records changed on both sides with different content.
    conflicts: usize,
}

/// Merges one profile changed on both sides. Name and description are scalars: a side
/// that did not change them takes the other side's value, and two different changes
/// are a conflict (the UI wins on the UI). Membership lists are merged as sets against
/// the base, so each side's additions survive and each side's removals apply. Returns
/// the merged profile and the number of scalar conflicts.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn merge_profile(
    base: Option<&Value>,
    local: &Value,
    remote: &Value,
    this_is_ui: bool,
) -> (Value, usize) {
    let mut conflicts = 0;
    let mut out = local.clone();
    let Some(object) = out.as_object_mut() else {
        return (local.clone(), 0);
    };
    for field in ["name", "description"] {
        let (b, l, r) = (
            base.and_then(|v| v.get(field)),
            local.get(field),
            remote.get(field),
        );
        let value = if l == r {
            l
        } else if l == b {
            r
        } else if r == b {
            l
        } else {
            conflicts += 1;
            if this_is_ui {
                l
            } else {
                r
            }
        };
        object.insert(field.to_owned(), value.cloned().unwrap_or(Value::Null));
    }
    for field in ["eventIds", "actionIds"] {
        let ids = |v: Option<&Value>| -> Vec<String> {
            v.and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|id| id.as_str().map(str::to_owned))
                .collect()
        };
        let merged = merge_id_sets(
            &ids(base.and_then(|v| v.get(field))),
            &ids(local.get(field)),
            &ids(remote.get(field)),
        );
        object.insert(
            field.to_owned(),
            Value::Array(merged.into_iter().map(Value::String).collect()),
        );
    }
    (out, conflicts)
}

/// Three-way merge of a membership list. An id one side removed is gone; an id one
/// side added is kept; the local order comes first, then ids only the remote added.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn merge_id_sets(base: &[String], local: &[String], remote: &[String]) -> Vec<String> {
    let base: BTreeSet<&str> = base.iter().map(String::as_str).collect();
    let local_set: BTreeSet<&str> = local.iter().map(String::as_str).collect();
    let remote_set: BTreeSet<&str> = remote.iter().map(String::as_str).collect();
    let removed = |side: &BTreeSet<&str>| -> BTreeSet<&str> {
        base.iter()
            .copied()
            .filter(|id| !side.contains(id))
            .collect()
    };
    let mut gone = removed(&local_set);
    gone.extend(removed(&remote_set));
    let mut out: Vec<String> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for id in local {
        if !gone.contains(id.as_str()) && seen.insert(id.as_str()) {
            out.push(id.clone());
        }
    }
    for id in remote {
        let remote_added = !base.contains(id.as_str()) && !local_set.contains(id.as_str());
        if remote_added && !gone.contains(id.as_str()) && seen.insert(id.as_str()) {
            out.push(id.clone());
        }
    }
    out
}

/// Merges two sides against their common base, record by record.
#[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
fn merge_library(
    base: &BTreeMap<String, String>,
    base_profiles: &BTreeMap<String, Value>,
    local: &BTreeMap<String, Value>,
    remote: &BTreeMap<String, Value>,
    this_is_ui: bool,
) -> LibraryMerge {
    let keys: BTreeSet<&String> = base
        .keys()
        .chain(local.keys())
        .chain(remote.keys())
        .collect();
    let mut merge = LibraryMerge::default();
    for key in keys {
        let local_value = local.get(key);
        let remote_value = remote.get(key);
        let local_digest = local_value.map(owner_library_digest);
        let remote_digest = remote_value.map(owner_library_digest);
        let chosen: Option<Value> = if local_digest == remote_digest {
            local_value.cloned()
        } else {
            let base_digest = base.get(key);
            let local_changed = local_digest.as_ref() != base_digest;
            let remote_changed = remote_digest.as_ref() != base_digest;
            match (local_value, remote_value) {
                (Some(l), Some(r))
                    if local_changed && remote_changed && key.starts_with("profile:") =>
                {
                    let (value, conflicts) =
                        merge_profile(base_profiles.get(key), l, r, this_is_ui);
                    merge.conflicts += conflicts;
                    Some(value)
                }
                _ if local_changed && remote_changed => {
                    merge.conflicts += 1;
                    if this_is_ui {
                        local_value.cloned()
                    } else {
                        remote_value.cloned()
                    }
                }
                _ if local_changed => local_value.cloned(),
                _ => remote_value.cloned(),
            }
        };
        let chosen_digest = chosen.as_ref().map(owner_library_digest);
        if chosen_digest != local_digest {
            merge.local_changes.insert(key.clone(), chosen.clone());
        }
        if chosen_digest != remote_digest {
            merge.publish = true;
        }
        if let Some(value) = chosen {
            merge.merged.insert(key.clone(), value);
        }
    }
    merge
}

/// The snapshot published to the Cloud: the owner-library shape, with the merged records.
#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
fn published_snapshot(
    host_id: &str,
    host_instance_id: &str,
    revision: &str,
    catalog: Value,
    merged: &BTreeMap<String, Value>,
) -> Value {
    let mut actions = Vec::new();
    let mut events = Vec::new();
    // Only records go in the definitions. Catalog keys travel in `catalog`.
    for (key, value) in merged {
        if key.starts_with("action:") {
            actions.push(value.clone());
        } else if key.starts_with("event:") {
            events.push(value.clone());
        }
    }
    serde_json::json!({
        "hostId": host_id,
        "hostInstanceId": host_instance_id,
        "revision": revision,
        "catalog": catalog,
        "definitions": {"actions": actions, "events": events},
    })
}

impl AppCore {
    /// Reads the JSON-encoded sync base from the shared app state store.
    /// Corrupt or absent state starts from an empty base instead of failing.
    pub fn library_sync_base(&self) -> Result<LibrarySyncBase, OperationError> {
        let stored = self.app_state_get(Some(&[LIBRARY_SYNC_BASE_KEY.to_owned()]))?;
        match stored.get(LIBRARY_SYNC_BASE_KEY) {
            Some(raw) => Ok(serde_json::from_str(raw).unwrap_or_else(|error| {
                tracing::warn!(%error, "discarding corrupt automation library sync base");
                LibrarySyncBase::default()
            })),
            None => Ok(LibrarySyncBase::default()),
        }
    }

    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    fn set_library_sync_base(&self, base: &LibrarySyncBase) -> Result<(), OperationError> {
        let value = serde_json::to_string(base)
            .map_err(|_| OperationError::internal("automation library sync base is unavailable"))?;
        self.app_state_set(LIBRARY_SYNC_BASE_KEY, &value)?;
        Ok(())
    }

    /// The origin attributed to local pushes. Defaults to `"agent"`; the
    /// desktop calls this with `"ui"` once at startup.
    pub fn set_sync_origin(&self, origin: &str) {
        let value = if origin == "ui" { "ui" } else { "agent" };
        let mut current = self.sync_origin.lock().unwrap_or_else(|e| e.into_inner());
        *current = value.to_owned();
    }

    pub fn sync_origin(&self) -> &'static str {
        let current = self.sync_origin.lock().unwrap_or_else(|e| e.into_inner());
        if current.as_str() == "ui" {
            "ui"
        } else {
            "agent"
        }
    }

    /// Publishes the library-change domain event with the new revision.
    #[cfg(any(all(feature = "cloud-auth", feature = "persistence"), test))]
    fn publish_automation_library_changed(&self, revision: &str) {
        self.events
            .publish_domain(crate::events::DomainEvent::WorkflowChanged {
                kind: AUTOMATION_LIBRARY_CHANGED_EVENT.to_owned(),
                id: "library".to_owned(),
                change: revision.to_owned(),
            });
    }

    /// Reads the local write counter of the automation library.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    fn automation_library_version(&self) -> Result<i64, OperationError> {
        self.db
            .automation_library_version()
            .map_err(|error| OperationError::internal(error.to_string()))
    }

    /// Writes the local side of a merge through the versioned record operations.
    /// Removals go first, then actions, then events, so no event is written before
    /// the action it may reference.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    fn write_local_changes(
        &self,
        changes: &BTreeMap<String, Option<Value>>,
        local: &BTreeMap<String, Value>,
    ) -> Result<(), OperationError> {
        let removals = changes.iter().filter(|(_, value)| value.is_none());
        for (key, _) in removals
            .clone()
            .filter(|(key, _)| key.starts_with("event:"))
        {
            let (kind, id) = split_key(key).ok_or_else(|| OperationError::internal("bad key"))?;
            self.automation_delete_versioned(kind, id, None)?;
        }
        for (key, _) in removals.filter(|(key, _)| key.starts_with("action:")) {
            let (kind, id) = split_key(key).ok_or_else(|| OperationError::internal("bad key"))?;
            self.automation_delete_versioned(kind, id, None)?;
        }
        let upserts = changes
            .iter()
            .filter_map(|(key, value)| value.as_ref().map(|value| (key, value)));
        for ordered in [true, false] {
            for (key, value) in upserts.clone() {
                if key.starts_with("action:") != ordered {
                    continue;
                }
                let (kind, id) =
                    split_key(key).ok_or_else(|| OperationError::internal("bad key"))?;
                if local.contains_key(key) {
                    self.automation_update_versioned(kind, id, value.clone(), None)?;
                } else {
                    self.automation_create_versioned(kind, value.clone(), None)?;
                }
            }
        }
        Ok(())
    }

    /// One sync pass: fetch the Cloud record, merge it with the local library against
    /// the stored base, write the Cloud side locally, and publish the merged library
    /// when it differs from the Cloud record.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    pub async fn automation_library_sync_now(&self) -> Result<LibrarySyncResult, OperationError> {
        let this_is_ui = self.sync_origin() == "ui";
        let origin = self.sync_origin();
        let base = self.library_sync_base()?;
        let local_version = self.automation_library_version()?;
        let session = self
            .cloud
            .auth()
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("Cloud sign-in is required"))?;
        let (remote, server_token) = match session.library_get().await.map_err(cloud_error)? {
            Some((record, token)) => (Some(LibrarySyncRecord::from(record)), Some(token)),
            None => (None, None),
        };
        let cursor = session.library_head().await.map_err(cloud_error)?;
        let refused = |message: String| LibrarySyncResult {
            action: LibrarySyncAction::ApplyRemote,
            revision: server_token.clone().unwrap_or_default(),
            conflicts: 1,
            rejection: Some(message),
        };

        // Nothing moved on either side since the base.
        if remote.is_some()
            && base.local_version == Some(local_version)
            && server_token == base.revision
        {
            self.set_library_sync_base(&LibrarySyncBase {
                cursor,
                ..base.clone()
            })?;
            return Ok(LibrarySyncResult {
                action: LibrarySyncAction::Nothing,
                revision: base.revision.clone().unwrap_or_default(),
                conflicts: 0,
                rejection: None,
            });
        }

        let remote_map = match remote.as_ref() {
            Some(record) => match remote_state(&record.snapshot) {
                Ok(state) => state,
                Err(error) => {
                    return Ok(refused(format!("{}: {}", error.code, error.message)));
                }
            },
            None => BTreeMap::new(),
        };
        for (key, value) in &remote_map {
            if is_record_key(key) {
                if let Err(error) = super::owner_definition::validate_source(value) {
                    return Ok(refused(format!("{}: {}", error.code, error.message)));
                }
            }
        }

        let host = self.owner_library_snapshot()?;
        let (catalog, stored_revision, definitions) = self
            .db
            .automation_library_snapshot()
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let mut local_map = record_map(&definitions.value)?;
        local_map.extend(catalog_map(&catalog)?);
        // With no Cloud record there is nothing to be removed from: the base is treated
        // as empty, so local records are kept and published instead of being deleted.
        let base_records = if remote.is_some() {
            base.records.clone()
        } else {
            BTreeMap::new()
        };
        let base_profiles = if remote.is_some() {
            base.profiles.clone()
        } else {
            BTreeMap::new()
        };
        let merge = merge_library(
            &base_records,
            &base_profiles,
            &local_map,
            &remote_map,
            this_is_ui,
        );

        // Everything is checked before the first write.
        let merged_catalog = match merged_catalog(&merge.merged) {
            Ok(merged) => merged,
            Err(error) => return Ok(refused(format!("{}: {}", error.code, error.message))),
        };
        if let Err(error) = self.db.check_automation_catalog(&merged_catalog) {
            return Ok(refused(format!("invalid_params: merged catalog: {error}")));
        }
        let record_changes: BTreeMap<String, Option<Value>> = merge
            .local_changes
            .iter()
            .filter(|(key, _)| is_record_key(key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let catalog_changed = merge.local_changes.keys().any(|key| is_catalog_key(key));

        if !record_changes.is_empty() {
            if let Err(error) = self.write_local_changes(&record_changes, &local_map) {
                return Ok(refused(format!("{}: {}", error.code, error.message)));
            }
        }
        if catalog_changed {
            let expected = match stored_revision.parse::<i64>() {
                Ok(revision) => revision,
                Err(_) => {
                    return Ok(refused(
                        "internal: catalog revision is unavailable".to_owned(),
                    ))
                }
            };
            if let Err(error) = self
                .db
                .replace_automation_catalog(&merged_catalog, expected)
            {
                return Ok(refused(format!("internal: catalog write failed: {error}")));
            }
        }

        let mut revision = server_token.clone().unwrap_or_default();
        let mut token_after = server_token.clone();
        if merge.publish {
            let catalog_value = serde_json::to_value(&merged_catalog).map_err(|_| {
                OperationError::internal("automation library catalog is unavailable")
            })?;
            let snapshot = published_snapshot(
                &host.host_id,
                &host.host_instance_id,
                &stored_revision,
                catalog_value,
                &merge.merged,
            );
            let record = cloud_client::LibraryRecord {
                schema_version: 1,
                origin: origin.to_owned(),
                revision: local_version.to_string(),
                snapshot,
                updated_at: None,
            };
            let token = session
                .library_put(&record, server_token.as_deref())
                .await
                .map_err(cloud_error)?;
            revision = token.clone();
            token_after = Some(token);
        }

        // The base holds what both sides now agree on. The counter is the one read
        // before the pass, so a write made during it is seen by the next pass.
        self.set_library_sync_base(&LibrarySyncBase {
            local_version: Some(local_version),
            revision: token_after,
            cursor,
            records: merge
                .merged
                .iter()
                .map(|(key, value)| (key.clone(), owner_library_digest(value)))
                .collect(),
            profiles: merge
                .merged
                .iter()
                .filter(|(key, _)| key.starts_with("profile:"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        })?;

        let changed = !merge.local_changes.is_empty() || merge.publish;
        let action = if merge.conflicts > 0 {
            if this_is_ui {
                LibrarySyncAction::PushLocalWinsConflict
            } else {
                LibrarySyncAction::ApplyRemoteWinsConflict
            }
        } else if !merge.local_changes.is_empty() {
            LibrarySyncAction::ApplyRemote
        } else if merge.publish {
            LibrarySyncAction::PushLocal
        } else {
            LibrarySyncAction::Nothing
        };
        if changed {
            self.publish_automation_library_changed(&revision);
        }
        Ok(LibrarySyncResult {
            action,
            revision,
            conflicts: merge.conflicts,
            rejection: None,
        })
    }

    /// Replaces a Cloud library record that cannot be read (one written by an older
    /// publish, for example) with this device's library. It is refused while the
    /// record is readable: a readable record is merged by `automation_library_sync_now`.
    /// The replacement is fenced on the record token it read.
    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    pub async fn automation_library_sync_replace_unreadable(
        &self,
    ) -> Result<LibrarySyncResult, OperationError> {
        let origin = self.sync_origin();
        let local_version = self.automation_library_version()?;
        let session = self
            .cloud
            .auth()
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("Cloud sign-in is required"))?;
        let (remote, server_token) = match session.library_get().await.map_err(cloud_error)? {
            Some((record, token)) => (Some(LibrarySyncRecord::from(record)), Some(token)),
            None => (None, None),
        };
        let cursor = session.library_head().await.map_err(cloud_error)?;
        if let Some(record) = remote.as_ref() {
            if remote_state(&record.snapshot).is_ok() {
                return Err(OperationError::invalid(
                    "the Cloud library record is readable; use automation.library.sync.now",
                ));
            }
        }
        let host = self.owner_library_snapshot()?;
        let (catalog, stored_revision, definitions) = self
            .db
            .automation_library_snapshot()
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let mut local_map = record_map(&definitions.value)?;
        local_map.extend(catalog_map(&catalog)?);
        let catalog_value = serde_json::to_value(&catalog)
            .map_err(|_| OperationError::internal("automation library catalog is unavailable"))?;
        let snapshot = published_snapshot(
            &host.host_id,
            &host.host_instance_id,
            &stored_revision,
            catalog_value,
            &local_map,
        );
        let record = cloud_client::LibraryRecord {
            schema_version: 1,
            origin: origin.to_owned(),
            revision: local_version.to_string(),
            snapshot,
            updated_at: None,
        };
        let token = session
            .library_put(&record, server_token.as_deref())
            .await
            .map_err(cloud_error)?;
        self.set_library_sync_base(&LibrarySyncBase {
            local_version: Some(local_version),
            revision: Some(token.clone()),
            cursor,
            records: local_map
                .iter()
                .map(|(key, value)| (key.clone(), owner_library_digest(value)))
                .collect(),
            profiles: local_map
                .iter()
                .filter(|(key, _)| key.starts_with("profile:"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        })?;
        self.publish_automation_library_changed(&token);
        Ok(LibrarySyncResult {
            action: LibrarySyncAction::PushLocal,
            revision: token,
            conflicts: 0,
            rejection: None,
        })
    }

    /// Offline builds keep the same call shape but cannot sync.
    #[cfg(not(all(feature = "cloud-auth", feature = "persistence")))]
    pub async fn automation_library_sync_now(&self) -> Result<LibrarySyncResult, OperationError> {
        Err(OperationError::capability_unavailable(
            "automation library sync requires cloud-auth",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{paths::AppPaths, HostEmitter, NoopMediaHost};
    use serde_json::json;
    use std::sync::Arc;

    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }

    fn core() -> Arc<AppCore> {
        let root = std::env::temp_dir().join(format!(
            "tt-automation-library-sync-{}-{}",
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
        Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ))
    }

    /// An event record; `name` is the only content that varies between versions.
    fn event(id: &str, name: &str) -> Value {
        json!({"id": id, "name": name, "enabled": false, "trigger": "tiktok.chat",
            "actionIds": [], "filters": [], "cooldownMs": 0, "cooldownScope": "global",
            "runMode": "all", "schemaVersion": 1})
    }

    /// Records keyed the way the sync keys them, from `(id, version)` pairs.
    fn side(records: &[(&str, &str)]) -> BTreeMap<String, Value> {
        records
            .iter()
            .map(|(id, name)| (format!("event:{id}"), event(id, name)))
            .collect()
    }

    /// The base digests of one side's records.
    fn base_of(records: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
        records
            .iter()
            .map(|(key, value)| (key.clone(), owner_library_digest(value)))
            .collect()
    }

    #[test]
    fn changes_on_both_sides_to_different_records_are_both_kept() {
        // The regression behind the lost agent change: each side edits a different record.
        let base = side(&[("a", "v1"), ("b", "v1")]);
        let local = side(&[("a", "local-edit"), ("b", "v1")]);
        let remote = side(&[("a", "v1"), ("b", "remote-edit"), ("c", "remote-new")]);
        let merge = merge_library(&base_of(&base), &BTreeMap::new(), &local, &remote, true);
        assert_eq!(merge.conflicts, 0);
        assert!(merge.publish);
        assert_eq!(merge.merged["event:a"]["name"], "local-edit");
        assert_eq!(merge.merged["event:b"]["name"], "remote-edit");
        assert_eq!(merge.merged["event:c"]["name"], "remote-new");
        // Local keeps only what it does not have yet; the local edit is published.
        assert_eq!(
            merge.local_changes.keys().collect::<Vec<_>>(),
            vec!["event:b", "event:c"]
        );
    }

    #[test]
    fn same_record_conflict_goes_to_the_ui_version_on_the_ui_and_the_remote_version_on_agents() {
        let base = side(&[("a", "v1")]);
        let local = side(&[("a", "ui-edit")]);
        let remote = side(&[("a", "agent-edit")]);

        let ui = merge_library(&base_of(&base), &BTreeMap::new(), &local, &remote, true);
        assert_eq!(ui.conflicts, 1);
        assert_eq!(ui.merged["event:a"]["name"], "ui-edit");
        assert!(ui.local_changes.is_empty(), "the UI keeps its own version");
        assert!(ui.publish, "the Cloud gets the UI version");

        let agent = merge_library(&base_of(&base), &BTreeMap::new(), &local, &remote, false);
        assert_eq!(agent.conflicts, 1);
        assert_eq!(agent.merged["event:a"]["name"], "agent-edit");
        assert_eq!(
            agent.local_changes["event:a"],
            Some(remote["event:a"].clone())
        );
        assert!(
            !agent.publish,
            "the Cloud already holds the winning version"
        );
    }

    #[test]
    fn a_local_removal_is_published_and_a_remote_removal_is_applied_locally() {
        let base = side(&[("gone-local", "v1"), ("gone-remote", "v1")]);
        let local = side(&[("gone-remote", "v1")]);
        let remote = side(&[("gone-local", "v1")]);
        let merge = merge_library(&base_of(&base), &BTreeMap::new(), &local, &remote, false);
        assert!(!merge.merged.contains_key("event:gone-local"));
        assert!(!merge.merged.contains_key("event:gone-remote"));
        // The local removal is already true locally: it is published, not written.
        assert!(!merge.local_changes.contains_key("event:gone-local"));
        // The remote removal is applied locally; nothing is left to publish for it.
        assert_eq!(merge.local_changes["event:gone-remote"], None);
        assert!(merge.publish);
    }

    #[test]
    fn identical_sides_need_nothing() {
        let records = side(&[("a", "v1")]);
        let merge = merge_library(
            &base_of(&records),
            &BTreeMap::new(),
            &records,
            &records,
            true,
        );
        assert_eq!(
            merge,
            LibraryMerge {
                merged: records.clone(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn the_first_sync_is_a_union_of_both_sides() {
        let local = side(&[("mine", "v1")]);
        let remote = side(&[("theirs", "v1")]);
        let merge = merge_library(&BTreeMap::new(), &BTreeMap::new(), &local, &remote, true);
        assert_eq!(merge.merged.len(), 2);
        assert_eq!(
            merge.local_changes.keys().collect::<Vec<_>>(),
            vec!["event:theirs"]
        );
        assert!(merge.publish);
    }

    #[test]
    fn a_remote_snapshot_without_its_lists_is_refused_not_read_as_empty() {
        assert!(remote_records(&json!({"unexpected": true})).is_err());
        assert!(remote_records(&json!({"definitions": {"actions": []}})).is_err());
        assert!(remote_records(&json!({"definitions": {"actions": [], "events": []}})).is_ok());
    }

    #[test]
    fn applying_a_remote_change_does_not_look_like_a_local_edit_on_the_next_pass() {
        let remote = side(&[("a", "v2")]);
        let base_after_apply = base_of(&remote);
        let merge = merge_library(&base_after_apply, &BTreeMap::new(), &remote, &remote, false);
        assert_eq!(
            merge,
            LibraryMerge {
                merged: remote.clone(),
                ..Default::default()
            }
        );
    }

    fn profile(id: &str, name: &str) -> Value {
        json!({"id": id, "name": name, "description": "", "eventIds": [], "actionIds": []})
    }

    fn template(id: &str, title: &str) -> Value {
        json!({"id": id, "title": title})
    }

    #[test]
    fn a_profile_edit_on_one_side_and_a_new_template_on_the_other_both_survive() {
        let base_catalog = OwnerLibraryCatalog {
            packs: vec![serde_json::from_value(profile("p1", "Shows")).unwrap()],
            active_id: "default".to_owned(),
            templates: Vec::new(),
        };
        let base = catalog_map(&base_catalog).unwrap();
        let local = catalog_map(&OwnerLibraryCatalog {
            templates: vec![template("t-local", "Local template")],
            ..base_catalog.clone()
        })
        .unwrap();
        let remote = catalog_map(&OwnerLibraryCatalog {
            packs: vec![serde_json::from_value(profile("p1", "Renamed remotely")).unwrap()],
            ..base_catalog.clone()
        })
        .unwrap();
        let merge = merge_library(&base_of_map(&base), &BTreeMap::new(), &local, &remote, true);
        assert_eq!(merge.conflicts, 0);
        let merged = merged_catalog(&merge.merged).unwrap();
        assert_eq!(merged.packs[0].name, "Renamed remotely");
        assert_eq!(merged.templates.len(), 1, "the local template is kept");
        assert_eq!(merged.templates[0]["id"], "t-local");
        assert!(merge.publish);
    }

    #[test]
    fn a_missing_active_profile_falls_back_to_the_default_profile() {
        // The active id names a profile that is not in the merged state.
        let mut merged = BTreeMap::new();
        merged.insert(ACTIVE_KEY.to_owned(), Value::String("p-removed".to_owned()));
        assert_eq!(merged_catalog(&merged).unwrap().active_id, "default");
    }

    #[test]
    fn a_catalog_with_duplicate_template_ids_is_refused() {
        let catalog = OwnerLibraryCatalog {
            packs: Vec::new(),
            active_id: "default".to_owned(),
            templates: vec![template("same", "a"), template("same", "b")],
        };
        assert!(catalog_map(&catalog).is_err());
    }

    fn base_of_map(map: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
        map.iter()
            .map(|(key, value)| (key.clone(), owner_library_digest(value)))
            .collect()
    }

    #[test]
    fn both_sides_adding_members_to_one_profile_keep_both_additions() {
        let base = json!({"id": "p1", "name": "Shows", "description": "", "eventIds": ["a"], "actionIds": []});
        let local = json!({"id": "p1", "name": "Shows", "description": "", "eventIds": ["a", "ui-new"], "actionIds": []});
        let remote = json!({"id": "p1", "name": "Shows", "description": "", "eventIds": ["a", "agent-new"], "actionIds": []});
        let (merged, conflicts) = merge_profile(Some(&base), &local, &remote, true);
        assert_eq!(conflicts, 0, "membership additions are not a conflict");
        assert_eq!(merged["eventIds"], json!(["a", "ui-new", "agent-new"]));
    }

    #[test]
    fn a_removal_on_one_side_applies_and_an_addition_on_the_other_survives() {
        let base = vec!["a".to_owned(), "b".to_owned()];
        let local = vec!["a".to_owned()];
        let remote = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(merge_id_sets(&base, &local, &remote), vec!["a", "c"]);
    }

    #[test]
    fn two_different_profile_names_are_a_conflict_that_follows_the_side_rule() {
        let base = json!({"id": "p1", "name": "Shows", "description": "", "eventIds": [], "actionIds": []});
        let local = json!({"id": "p1", "name": "UI name", "description": "", "eventIds": [], "actionIds": []});
        let remote = json!({"id": "p1", "name": "Agent name", "description": "", "eventIds": [], "actionIds": []});
        let (ui, ui_conflicts) = merge_profile(Some(&base), &local, &remote, true);
        assert_eq!((ui["name"].clone(), ui_conflicts), (json!("UI name"), 1));
        let (agent, agent_conflicts) = merge_profile(Some(&base), &local, &remote, false);
        assert_eq!(
            (agent["name"].clone(), agent_conflicts),
            (json!("Agent name"), 1)
        );
    }

    #[test]
    fn applied_change_publishes_library_event_with_new_revision() {
        let core = core();
        let mut events = core.events.subscribe_projection();
        core.publish_automation_library_changed("42");
        let event = events.try_recv().expect("library event was published");
        let envelope = event.to_envelope().expect("event serializes");
        assert_eq!(envelope.data["kind"], AUTOMATION_LIBRARY_CHANGED_EVENT);
        assert_eq!(envelope.data["change"], "42");
    }
}

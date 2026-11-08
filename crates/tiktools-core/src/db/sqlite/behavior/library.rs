//! Library business writes use the same transaction as definitions and their
//! trusted provenance. No app-state or second-database write occurs here.
use super::super::automation_library::{
    catalog_on, source_budget, validate_catalog, write_catalog_on, LIBRARY_METHOD,
    RESULT_VERSIONS_BYTES, SNAPSHOT_BYTES,
};
use super::*;
use crate::{
    control::{
        owner_library_definition_digest, owner_library_digest, owner_library_versions_digest,
        validate_owner_library_submit, AutomationKind, OwnerLibraryCatalog, OwnerLibraryEntry,
        OwnerLibraryMutation, OwnerLibraryOperation, OwnerLibraryPack, OwnerLibraryReceipt,
        OwnerLibrarySubmit, OWNER_LIBRARY_MAX_BYTES,
    },
    db::CommandTransaction,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredLibraryResult {
    receipt: OwnerLibraryReceipt,
    operation: OwnerLibraryOperation,
    target: String,
    expected_revision: String,
    definition_digest: String,
}

fn unwrap_result(
    mut result: Versioned<Value>,
    input: &OwnerLibrarySubmit,
) -> Result<Versioned<Value>, DatabaseError> {
    if serde_json::to_vec(&result.value)?.len() > 8 * 1024 {
        return Err(invalid());
    }
    let saved: StoredLibraryResult = serde_json::from_value(result.value)?;
    if saved.operation != input.mutation.operation()
        || saved.target != input.mutation.target()
        || saved.expected_revision != input.expected_revision
        || saved.definition_digest != input.definition_digest
    {
        return Err(invalid());
    }
    result.value = serde_json::to_value(saved.receipt)?;
    Ok(result)
}

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("invalid automation library transition".into())
}

/// A coherent source universe, with raw byte admission before JSON decoding.
pub(in crate::db::sqlite) fn snapshot_on(
    db: &Connection,
) -> Result<Versioned<Value>, DatabaseError> {
    provenance::validate_runtime_catalog(db)?;
    // Validate authority values and retained orphan identities as well as the
    // bidirectional version relation before admitting a coherent universe.
    provenance::rows(db)?;
    let mut raw_bytes = 0i64;
    for table in ["behavior_actions", "behavior_events"] {
        let bytes:i64 = db.query_row(&format!("SELECT coalesce(sum(coalesce(length(CAST(payload_json AS BLOB)),0)+length(CAST(id AS BLOB))+length(CAST(name AS BLOB))+64),0) FROM {table}"),[],|row|row.get(0))?;
        raw_bytes = raw_bytes
            .checked_add(bytes)
            .ok_or(DatabaseError::SnapshotTooLarge)?;
        if raw_bytes < 0 || raw_bytes as u64 > SNAPSHOT_BYTES as u64 {
            return Err(DatabaseError::SnapshotTooLarge);
        }
    }
    let plugin_bytes: i64 = db.query_row(
        "SELECT coalesce(sum(length(CAST(id AS BLOB))+48),0) FROM behavior_plugins",
        [],
        |row| row.get(0),
    )?;
    if plugin_bytes < 0 || raw_bytes.saturating_add(plugin_bytes) as u64 > SNAPSHOT_BYTES as u64 {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let mut snapshot = DatabaseManager::load_behavior_snapshot_from(db)?;
    for key in ["actions", "events"] {
        let rows = snapshot
            .value
            .get_mut(key)
            .and_then(Value::as_array_mut)
            .ok_or_else(invalid)?;
        for row in rows.iter() {
            source_budget(row, OWNER_LIBRARY_MAX_BYTES)?;
            if !row
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(crate::control::owner_library::library_record_id)
            {
                return Err(invalid());
            }
        }
        rows.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    }
    snapshot
        .resource_versions
        .sort_by(|left, right| (&left.scope, &left.id).cmp(&(&right.scope, &right.id)));
    if serde_json::to_vec(&snapshot.value)?
        .len()
        .saturating_add(serde_json::to_vec(&snapshot.resource_versions)?.len())
        > SNAPSHOT_BYTES
    {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    Ok(snapshot)
}
fn row_version<'a>(
    snapshot: &'a Versioned<Value>,
    kind: AutomationKind,
    id: &str,
) -> Option<&'a ResourceVersion> {
    snapshot
        .resource_versions
        .iter()
        .find(|version| version.scope == kind.family().scope() && version.id == id)
}
fn insert_versions(
    affected: &mut BTreeMap<(String, String), ResourceVersion>,
    versions: Vec<ResourceVersion>,
) {
    for version in versions {
        affected.insert((version.scope.clone(), version.id.clone()), version);
    }
}
fn rows(snapshot: &Versioned<Value>, kind: AutomationKind) -> Result<&[Value], DatabaseError> {
    snapshot
        .value
        .get(match kind {
            AutomationKind::Action => "actions",
            AutomationKind::Event => "events",
        })
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(invalid)
}
fn upsert_templates(
    catalog: &mut OwnerLibraryCatalog,
    templates: &[Value],
) -> Result<(), DatabaseError> {
    let mut seen = BTreeSet::new();
    for template in templates {
        source_budget(template, 256 * 1024)?;
        let parsed =
            crate::services::templates::parse_rule_template(template).map_err(|_| invalid())?;
        let id = template
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        if parsed.id != id || !seen.insert(id) {
            return Err(invalid());
        }
        catalog
            .templates
            .retain(|value| value.get("id").and_then(Value::as_str) != Some(id));
        catalog.templates.push(template.clone());
    }
    Ok(())
}
fn create_entries(
    tx: &Transaction<'_>,
    entries: &[&OwnerLibraryEntry],
    affected: &mut BTreeMap<(String, String), ResourceVersion>,
) -> Result<(Vec<String>, Vec<String>), DatabaseError> {
    let mut actions = Vec::new();
    let mut events = Vec::new();
    for entry in entries {
        for action in &entry.actions {
            let result = save_record(
                tx,
                "behavior_actions",
                action,
                Some("0"),
                BehaviorSave::Create,
            )?;
            actions.push(action["id"].as_str().ok_or_else(invalid)?.to_owned());
            insert_versions(affected, result.resource_versions);
        }
        let result = save_record(
            tx,
            "behavior_events",
            &entry.event,
            Some("0"),
            BehaviorSave::Create,
        )?;
        events.push(entry.event["id"].as_str().ok_or_else(invalid)?.to_owned());
        insert_versions(affected, result.resource_versions);
    }
    Ok((actions, events))
}
fn named_profile(catalog: &OwnerLibraryCatalog, id: &str) -> Result<Option<usize>, DatabaseError> {
    if id == "default" {
        Ok(None)
    } else {
        catalog
            .packs
            .iter()
            .position(|pack| pack.id == id)
            .map(Some)
            .ok_or(DatabaseError::NotFound)
    }
}
fn create_into_profile(
    tx: &Transaction<'_>,
    catalog: &mut OwnerLibraryCatalog,
    entries: &[&OwnerLibraryEntry],
    profile_id: &str,
    affected: &mut BTreeMap<(String, String), ResourceVersion>,
) -> Result<(), DatabaseError> {
    let profile = named_profile(catalog, profile_id)?;
    let (actions, events) = create_entries(tx, entries, affected)?;
    clear_fresh_memberships(catalog, &actions, &events);
    if let Some(index) = profile {
        let pack = &mut catalog.packs[index];
        for id in actions {
            if !pack.action_ids.contains(&id) {
                pack.action_ids.push(id);
            }
        }
        for id in events {
            if !pack.event_ids.contains(&id) {
                pack.event_ids.push(id);
            }
        }
    }
    Ok(())
}
/// Explicit fresh creation owns the membership decision, including Default.
/// CAS at zero has already excluded every live or retained source identity.
fn clear_fresh_memberships(
    catalog: &mut OwnerLibraryCatalog,
    actions: &[String],
    events: &[String],
) {
    let actions: BTreeSet<&str> = actions.iter().map(String::as_str).collect();
    let events: BTreeSet<&str> = events.iter().map(String::as_str).collect();
    for pack in &mut catalog.packs {
        pack.action_ids.retain(|id| !actions.contains(id.as_str()));
        pack.event_ids.retain(|id| !events.contains(id.as_str()));
    }
}
fn switch_profile(
    tx: &Transaction<'_>,
    catalog: &mut OwnerLibraryCatalog,
    snapshot: &Versioned<Value>,
    id: &str,
    affected: &mut BTreeMap<(String, String), ResourceVersion>,
) -> Result<(), DatabaseError> {
    let profile = named_profile(catalog, id)?;
    let selected = |kind: AutomationKind| -> BTreeSet<String> {
        if let Some(index) = profile {
            (if kind == AutomationKind::Action {
                &catalog.packs[index].action_ids
            } else {
                &catalog.packs[index].event_ids
            })
            .iter()
            .cloned()
            .collect()
        } else {
            let owned: BTreeSet<&str> = catalog
                .packs
                .iter()
                .flat_map(|pack| {
                    if kind == AutomationKind::Action {
                        pack.action_ids.iter()
                    } else {
                        pack.event_ids.iter()
                    }
                })
                .map(String::as_str)
                .collect();
            rows(snapshot, kind)
                .unwrap_or_default()
                .iter()
                .filter_map(|value| value["id"].as_str())
                .filter(|id| !owned.contains(id))
                .map(str::to_owned)
                .collect()
        }
    };
    let actions = selected(AutomationKind::Action);
    let events = selected(AutomationKind::Event);
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        for row in rows(snapshot, kind)? {
            let id = row["id"].as_str().ok_or_else(invalid)?;
            let enabled = if kind == AutomationKind::Action {
                actions.contains(id)
            } else {
                events.contains(id)
            };
            let mut output = row.clone();
            output["enabled"] = enabled.into();
            source_budget(&output, 256 * 1024)?;
            let before = row_version(snapshot, kind, id).ok_or_else(invalid)?;
            let result = set_enabled(tx, kind.table(), id, enabled, Some(&before.revision))?;
            insert_versions(affected, result.resource_versions);
        }
    }
    let live_actions: BTreeSet<_> = rows(snapshot, AutomationKind::Action)?
        .iter()
        .filter_map(|value| value["id"].as_str())
        .collect();
    let live_events: BTreeSet<_> = rows(snapshot, AutomationKind::Event)?
        .iter()
        .filter_map(|value| value["id"].as_str())
        .collect();
    for pack in &mut catalog.packs {
        pack.action_ids
            .retain(|id| live_actions.contains(id.as_str()));
        pack.event_ids
            .retain(|id| live_events.contains(id.as_str()));
    }
    catalog.active_id = id.into();
    Ok(())
}
fn delete_profile(
    tx: &Transaction<'_>,
    catalog: &mut OwnerLibraryCatalog,
    snapshot: &Versioned<Value>,
    id: &str,
    affected: &mut BTreeMap<(String, String), ResourceVersion>,
) -> Result<(), DatabaseError> {
    let index = named_profile(catalog, id)?.ok_or_else(invalid)?;
    let actions: BTreeSet<_> = catalog.packs[index].action_ids.iter().cloned().collect();
    let events: BTreeSet<_> = catalog.packs[index].event_ids.iter().cloned().collect();
    let live_actions: BTreeSet<&str> = actions
        .iter()
        .map(String::as_str)
        .filter(|id| row_version(snapshot, AutomationKind::Action, id).is_some())
        .collect();
    // Explicit event deletes happen once. They are excluded from later action
    // cascades, so a record cannot receive multiple revisions in this command.
    for id in &events {
        let Some(before) = row_version(snapshot, AutomationKind::Event, id) else {
            continue;
        };
        let result = delete_record(tx, "behavior_events", id, Some(&before.revision))?;
        insert_versions(affected, result.resource_versions);
    }
    // Collapse all action references in each surviving event at once.
    for row in rows(snapshot, AutomationKind::Event)? {
        let id = row["id"].as_str().ok_or_else(invalid)?;
        if events.contains(id) {
            continue;
        }
        let mut value = row.clone();
        let Some(references) = value.get_mut("actionIds").and_then(Value::as_array_mut) else {
            continue;
        };
        let before_len = references.len();
        references.retain(|value| !value.as_str().is_some_and(|id| live_actions.contains(id)));
        if before_len == references.len() {
            continue;
        }
        let before = versions::check(tx, "behavior_events", id, None)?;
        let object = value.as_object().ok_or_else(invalid)?;
        let name = required_value_string(object, "name")?;
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let at = now();
        tx.execute("UPDATE behavior_events SET name=?2,enabled=?3,payload_json=?4,updated_at=?5 WHERE id=?1",params![id,name,bool_int(enabled),serde_json::to_string(&value)?,at])?;
        let version = versions::advance(
            tx,
            "behavior_events",
            id,
            before,
            false,
            "references-updated",
            at,
        )?;
        insert_versions(affected, vec![version]);
    }
    for id in &actions {
        if row_version(snapshot, AutomationKind::Action, id).is_none() {
            continue;
        }
        let before = versions::check(tx, "behavior_actions", id, None)?;
        let removed = tx.execute("DELETE FROM behavior_actions WHERE id=?1", [id])? > 0;
        let version = if removed {
            versions::advance(tx, "behavior_actions", id, before, true, "deleted", now())?
        } else {
            versions::version("behavior_actions", id, before)?
        };
        insert_versions(affected, vec![version]);
    }
    catalog.packs.remove(index);
    for pack in &mut catalog.packs {
        pack.action_ids.retain(|id| !actions.contains(id));
        pack.event_ids.retain(|id| !events.contains(id));
    }
    if catalog.active_id == id {
        catalog.active_id = "default".into();
    }
    Ok(())
}
fn apply(
    tx: &Transaction<'_>,
    catalog: &mut OwnerLibraryCatalog,
    snapshot: &Versioned<Value>,
    mutation: &OwnerLibraryMutation,
) -> Result<Vec<ResourceVersion>, DatabaseError> {
    let mut affected = BTreeMap::new();
    match mutation {
        OwnerLibraryMutation::CreateProfile {
            id,
            name,
            description,
        } => {
            if catalog.packs.iter().any(|pack| pack.id == *id) {
                return Err(invalid());
            }
            catalog.packs.push(OwnerLibraryPack {
                id: id.clone(),
                name: name.clone(),
                description: description.clone(),
                event_ids: Vec::new(),
                action_ids: Vec::new(),
            });
        }
        OwnerLibraryMutation::SwitchProfile { id } => {
            switch_profile(tx, catalog, snapshot, id, &mut affected)?
        }
        OwnerLibraryMutation::DeleteProfile { id } => {
            delete_profile(tx, catalog, snapshot, id, &mut affected)?
        }
        OwnerLibraryMutation::ApplyTemplate {
            entry,
            profile_id,
            additional_entries,
            templates,
        } => {
            let entries: Vec<_> = std::iter::once(entry)
                .chain(additional_entries.iter().flatten())
                .collect();
            create_into_profile(tx, catalog, &entries, profile_id, &mut affected)?;
            if let Some(templates) = templates {
                upsert_templates(catalog, templates)?;
            }
        }
        OwnerLibraryMutation::ImportProfile {
            profile,
            entries,
            templates,
        } => {
            let entries: Vec<_> = entries.iter().collect();
            let (actions, events) = create_entries(tx, &entries, &mut affected)?;
            clear_fresh_memberships(catalog, &actions, &events);
            catalog.packs.retain(|pack| pack.id != profile.id);
            catalog.packs.push(OwnerLibraryPack {
                id: profile.id.clone(),
                name: profile.name.clone(),
                description: profile.description.clone(),
                action_ids: actions,
                event_ids: events,
            });
            catalog.active_id = profile.id.clone();
            if let Some(templates) = templates {
                upsert_templates(catalog, templates)?;
            }
        }
        OwnerLibraryMutation::AssignRecord {
            kind,
            id,
            profile_id,
        } => {
            let profile = named_profile(catalog, profile_id)?;
            if row_version(snapshot, *kind, id).is_none() {
                return Err(DatabaseError::NotFound);
            }
            let already = catalog.packs.iter().any(|pack| {
                (if *kind == AutomationKind::Action {
                    &pack.action_ids
                } else {
                    &pack.event_ids
                })
                .contains(id)
            });
            if let Some(index) = profile.filter(|_| !already) {
                (if *kind == AutomationKind::Action {
                    &mut catalog.packs[index].action_ids
                } else {
                    &mut catalog.packs[index].event_ids
                })
                .push(id.clone());
            }
        }
        OwnerLibraryMutation::ImportTemplates { templates } => {
            upsert_templates(catalog, templates)?
        }
        OwnerLibraryMutation::DeleteTemplate { id } => {
            catalog
                .templates
                .retain(|template| template.get("id").and_then(Value::as_str) != Some(id.as_str()));
        }
    }
    validate_catalog(catalog)?;
    Ok(affected.into_values().collect())
}

impl DatabaseManager {
    #[cfg(test)]
    pub(crate) fn execute_automation_library_command(
        &self,
        input: &OwnerLibrarySubmit,
        command: &CommandTransaction,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        self.execute_automation_library_command_with_budget(input, command, SNAPSHOT_BYTES)
    }
    pub(crate) fn execute_automation_library_command_with_budget(
        &self,
        input: &OwnerLibrarySubmit,
        command: &CommandTransaction,
        snapshot_budget: usize,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        validate_owner_library_submit(input).map_err(|_| invalid())?;
        command.request.validate().map_err(|_| invalid())?;
        if command.principal_id != "local-owner"
            || command.session_id != "local-owner"
            || command.authorization.is_some()
            || command.request.expected_revision != input.expected_revision
            || command.request.host_instance_id != input.host_instance_id
        {
            return Err(invalid());
        }
        self.ensure_automation_library()?;
        let hash = crate::control::owner_library_command_hash(input);
        self.behavior_transaction(|tx| {
            super::super::commands::prune(tx)?;
            if let Some(result)=super::super::commands::replay(tx,command,&hash)? {return Ok((unwrap_result(result,input)?,false));}
            let (mut catalog,before)=catalog_on(tx)?;
            if before.to_string()!=input.expected_revision {
                return Err(DatabaseError::RevisionConflict(ResourceVersion{kind:crate::collaboration::ResourceKind::Automation,scope:"library".into(),id:"catalog".into(),revision:before.to_string()}));
            }
            let snapshot=snapshot_on(tx)?;
            if owner_library_definition_digest(&snapshot.value,&snapshot.resource_versions)!=input.definition_digest {return Err(DatabaseError::LibraryDefinitionConflict);}
            let affected=apply(tx,&mut catalog,&snapshot,&input.mutation)?;
            let primary=write_catalog_on(tx,&catalog,before)?;
            let mut final_versions=vec![primary.clone()];
            final_versions.extend(affected);
            if serde_json::to_vec(&final_versions)?.len()>RESULT_VERSIONS_BYTES {return Err(DatabaseError::SnapshotTooLarge);}
            let final_snapshot=snapshot_on(tx)?;
            if serde_json::to_vec(&json!({"catalog":catalog,"revision":primary.revision,"definitions":final_snapshot.value,"resourceVersions":final_snapshot.resource_versions}))?.len()>snapshot_budget.min(SNAPSHOT_BYTES) {return Err(DatabaseError::SnapshotTooLarge);}
            let receipt=OwnerLibraryReceipt{revision:primary.revision,catalog_digest:owner_library_digest(&json!(catalog)),definition_digest:owner_library_definition_digest(&final_snapshot.value,&final_snapshot.resource_versions),versions_digest:owner_library_versions_digest(&final_versions),affected_resources:u32::try_from(final_versions.len()).map_err(|_|DatabaseError::SnapshotTooLarge)?};
            let saved=StoredLibraryResult {receipt,operation:input.mutation.operation(),target:input.mutation.target().into(),expected_revision:input.expected_revision.clone(),definition_digest:input.definition_digest.clone()};
            let saved=serde_json::to_value(saved)?;
            if serde_json::to_vec(&saved)?.len()>8*1024 {return Err(DatabaseError::SnapshotTooLarge);}
            let result=super::super::commands::finish(tx,command,&hash,LIBRARY_METHOD,&before.to_string(),Versioned::new(saved,final_versions))?;
            Ok((unwrap_result(result,input)?,true))
        })
    }
}

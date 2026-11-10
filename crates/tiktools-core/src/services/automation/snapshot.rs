//! Behavior snapshot index and plugin trigger ownership.

use super::*;

impl AutomationService {
    pub fn replace_snapshot(&self, snapshot: &Value) {
        self.replace_snapshot_versioned(snapshot, &[]);
    }
    pub(crate) fn replace_snapshot_versioned(
        &self,
        snapshot: &Value,
        versions: &[crate::collaboration::ResourceVersion],
    ) {
        self.replace_snapshot_internal(snapshot, versions, None, |index| {
            #[cfg(feature = "persistence")]
            approval::clear(index);
            #[cfg(not(feature = "persistence"))]
            let _ = index;
        });
    }
    #[cfg(all(feature = "persistence", test))]
    pub(crate) fn replace_snapshot_trusted(
        &self,
        snapshot: &Value,
        versions: &[crate::collaboration::ResourceVersion],
        authorities: RuntimeAuthorities,
    ) {
        self.replace_snapshot_internal(snapshot, versions, Some(authorities), approval::clear);
    }
    /// The approval list and definitions must come from the same trusted read
    /// transaction. Ordinary snapshot reloads deliberately erase approvals.
    #[cfg(feature = "persistence")]
    pub(crate) fn replace_snapshot_trusted_with_approvals(
        &self,
        snapshot: &Value,
        versions: &[crate::collaboration::ResourceVersion],
        authorities: RuntimeAuthorities,
        approvals: Vec<crate::db::BehaviorApprovalSnapshot>,
    ) {
        self.replace_snapshot_internal(snapshot, versions, Some(authorities), |index| {
            approval::install(index, approvals);
        });
    }
    fn replace_snapshot_internal(
        &self,
        snapshot: &Value,
        versions: &[crate::collaboration::ResourceVersion],
        authorities: Option<RuntimeAuthorities>,
        install: impl FnOnce(&mut BehaviorIndex),
    ) {
        let actions = records_by_id(snapshot.get("actions"));
        let events = records_by_id(snapshot.get("events"));
        let (owners, plugins) = plugin_trigger_ownership(snapshot);
        let mut index = recover_rwlock_write(&self.index, "automation index");
        if authorities.as_ref().is_some_and(|authorities| {
            !authority::coherent(&actions, &events, versions, authorities)
        }) {
            index.actions.clear();
            index.events.clear();
            index.catalog_epoch = None;
            index.authorities = None;
            index.authority_available = false;
            #[cfg(feature = "persistence")]
            approval::clear(&mut index);
            return;
        }
        if index.trigger_owners != owners
            || index.enabled_plugins != plugins
            || index.catalog_epoch.is_none()
        {
            index.catalog_epoch = index.next();
        }
        let mut sequence = index.sequence;
        let actions = index_records(
            &index.actions,
            actions,
            versions,
            "action",
            &mut sequence,
            None,
            (&index.authorities, &authorities),
        );
        let events = index_records(
            &index.events,
            events,
            versions,
            "event",
            &mut sequence,
            Some((&index.actions, &actions)),
            (&index.authorities, &authorities),
        );
        index.sequence = sequence;
        index.actions = actions;
        index.events = events;
        index.trigger_owners = owners;
        index.enabled_plugins = plugins;
        index.authorities = authorities;
        index.authority_available = true;
        install(&mut index);
    }
    #[cfg(not(feature = "persistence"))]
    pub(crate) fn records_snapshot(&self) -> Value {
        let index = recover_rwlock_read(&self.index, "automation index");
        serde_json::json!({"actions":index.actions.values().map(|r|&r.definition).collect::<Vec<_>>(),"events":index.events.values().map(|r|&r.definition).collect::<Vec<_>>(),"plugins":[],"actionTypes":[],"translations":{}})
    }
    pub fn upsert_action(&self, action: Value) {
        if let Some(id) = action.get("id").and_then(Value::as_str) {
            let id = id.to_owned();
            let mut index = recover_rwlock_write(&self.index, "automation index");
            #[cfg(feature = "persistence")]
            approval::clear(&mut index);
            let generation = index.next();
            if let Some(authorities) = &mut index.authorities {
                let authority = authorities
                    .entry(("action".into(), id.clone()))
                    .or_insert_with(RuntimeAuthority::inline_owner);
                if authority.is_owner() {
                    *authority = RuntimeAuthority::inline_owner();
                }
            }
            index.actions.insert(
                id.clone(),
                IndexedRecord {
                    definition: action,
                    persisted_revision: None,
                    generation,
                },
            );
            invalidate_dependents(&mut index, &id);
        }
    }
    pub fn remove_action(&self, id: &str) {
        let mut index = recover_rwlock_write(&self.index, "automation index");
        #[cfg(feature = "persistence")]
        approval::clear(&mut index);
        if index.actions.remove(id).is_none() {
            return;
        }
        let affected: Vec<_> = index
            .events
            .iter()
            .filter(|(_, r)| {
                r.definition
                    .get("actionIds")
                    .and_then(Value::as_array)
                    .is_some_and(|ids| ids.iter().any(|v| v.as_str() == Some(id)))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for event in affected {
            let generation = index.next();
            if let Some(record) = index.events.get_mut(&event) {
                if let Some(ids) = record
                    .definition
                    .get_mut("actionIds")
                    .and_then(Value::as_array_mut)
                {
                    ids.retain(|v| v.as_str() != Some(id));
                }
                record.generation = generation;
            }
        }
    }
    pub fn upsert_event(&self, event: Value) {
        if let Some(id) = event.get("id").and_then(Value::as_str) {
            let id = id.to_owned();
            let mut index = recover_rwlock_write(&self.index, "automation index");
            #[cfg(feature = "persistence")]
            approval::clear(&mut index);
            let generation = index.next();
            if let Some(authorities) = &mut index.authorities {
                let authority = authorities
                    .entry(("event".into(), id.clone()))
                    .or_insert_with(RuntimeAuthority::inline_owner);
                if authority.is_owner() {
                    *authority = RuntimeAuthority::inline_owner();
                }
            }
            index.events.insert(
                id,
                IndexedRecord {
                    definition: event,
                    persisted_revision: None,
                    generation,
                },
            );
        }
    }
    pub fn remove_event(&self, id: &str) {
        let mut index = recover_rwlock_write(&self.index, "automation index");
        index.events.remove(id);
        #[cfg(feature = "persistence")]
        approval::clear(&mut index);
        drop(index);
        recover_mutex(&self.cooldowns, "automation cooldowns").retain(|key, _| key.event_id != id);
    }
}
type ActionDependencies<'a> = (
    &'a BTreeMap<String, IndexedRecord>,
    &'a BTreeMap<String, IndexedRecord>,
);

fn index_records(
    previous: &BTreeMap<String, IndexedRecord>,
    records: BTreeMap<String, Value>,
    versions: &[crate::collaboration::ResourceVersion],
    scope: &str,
    sequence: &mut u64,
    dependencies: Option<ActionDependencies<'_>>,
    authorities: (&Option<RuntimeAuthorities>, &Option<RuntimeAuthorities>),
) -> BTreeMap<String, IndexedRecord> {
    let versions: BTreeMap<_, _> = versions
        .iter()
        .filter(|v| v.kind == crate::collaboration::ResourceKind::Automation && v.scope == scope)
        .map(|v| (v.id.as_str(), v.revision.as_str()))
        .collect();
    records
        .into_iter()
        .map(|(id, definition)| {
            let persisted_revision = versions.get(id.as_str()).map(|v| (*v).to_owned());
            let generation = match previous.get(&id) {
                Some(old)
                    if old.definition == definition
                        && old.persisted_revision == persisted_revision
                        && !authority::authority_changed(
                            authorities.0,
                            authorities.1,
                            scope,
                            &id,
                        )
                        && !dependencies.is_some_and(|(before, after)| {
                            definition
                                .get("actionIds")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .any(|id| {
                                    before.get(id).map(|r| r.generation)
                                        != after.get(id).map(|r| r.generation)
                                        || authority::authority_changed(
                                            authorities.0,
                                            authorities.1,
                                            "action",
                                            id,
                                        )
                                })
                        }) =>
                {
                    old.generation
                }
                _ => {
                    let next = sequence.checked_add(1);
                    if let Some(next) = next {
                        *sequence = next;
                    }
                    next
                }
            };
            (
                id,
                IndexedRecord {
                    definition,
                    persisted_revision,
                    generation,
                },
            )
        })
        .collect()
}

/// Plugin-owned triggers from a behavior snapshot: trigger type -> plugin id,
/// plus the set of plugins that are installed, enabled, and available.
fn plugin_trigger_ownership(
    snapshot: &Value,
) -> (BTreeMap<String, String>, std::collections::BTreeSet<String>) {
    let mut trigger_owners = BTreeMap::new();
    if let Some(entries) = snapshot.get("eventTypes").and_then(Value::as_array) {
        for entry in entries {
            let event_type = entry.get("type").and_then(Value::as_str);
            let plugin_id = entry
                .get("source")
                .and_then(|source| source.get("pluginId"))
                .and_then(Value::as_str);
            if let (Some(event_type), Some(plugin_id)) = (event_type, plugin_id) {
                trigger_owners.insert(event_type.to_owned(), plugin_id.to_owned());
            }
        }
    }
    let mut enabled_plugins = std::collections::BTreeSet::new();
    if let Some(plugins) = snapshot.get("plugins").and_then(Value::as_array) {
        for plugin in plugins {
            let id = plugin
                .get("descriptor")
                .and_then(|descriptor| descriptor.get("id"))
                .and_then(Value::as_str)
                .or_else(|| plugin.get("id").and_then(Value::as_str));
            let active = plugin.get("installed").and_then(Value::as_bool) == Some(true)
                && plugin.get("enabled").and_then(Value::as_bool) == Some(true)
                && plugin.get("available").and_then(Value::as_bool) != Some(false);
            if let Some(id) = id {
                if active {
                    enabled_plugins.insert(id.to_owned());
                }
            }
        }
    }
    (trigger_owners, enabled_plugins)
}

/// Built-in triggers always match; plugin triggers only match while their
/// owning plugin is active, so disabling a plugin pauses its events.
pub(crate) fn trigger_allowed(
    trigger: Option<&str>,
    trigger_owners: &BTreeMap<String, String>,
    enabled_plugins: &std::collections::BTreeSet<String>,
) -> bool {
    let Some(trigger) = trigger else {
        return false;
    };
    match trigger_owners.get(trigger) {
        None => true,
        Some(plugin_id) => enabled_plugins.contains(plugin_id),
    }
}

fn records_by_id(value: Option<&Value>) -> BTreeMap<String, Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_owned(), value.clone()))
        })
        .collect()
}

fn invalidate_dependents(index: &mut BehaviorIndex, action: &str) {
    let affected: Vec<_> = index
        .events
        .iter()
        .filter(|(_, r)| {
            r.definition
                .get("actionIds")
                .and_then(Value::as_array)
                .is_some_and(|ids| ids.iter().any(|v| v.as_str() == Some(action)))
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in affected {
        let generation = index.next();
        if let Some(r) = index.events.get_mut(&id) {
            r.generation = generation;
        }
    }
}

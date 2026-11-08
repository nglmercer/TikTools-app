//! Trusted sidecar projection. These values never come from editable JSON.
use super::*;

/// Exact host-owned identity behind a runtime record. An inline Owner has no
/// persisted revision/generation; it is never represented as saved approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeAuthority {
    kind: RuntimeAuthorityKind,
    generation: Option<u64>,
    revision: Option<String>,
    deleted: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAuthorityKind {
    Owner,
    #[cfg(feature = "persistence")]
    Delegated {
        principal_id: String,
        session_id: String,
    },
}
impl RuntimeAuthority {
    pub(super) fn inline_owner() -> Self {
        Self {
            kind: RuntimeAuthorityKind::Owner,
            generation: None,
            revision: None,
            deleted: false,
        }
    }
    pub(crate) fn is_owner(&self) -> bool {
        matches!(self.kind, RuntimeAuthorityKind::Owner)
    }
    pub(crate) fn subject(&self) -> Option<(&str, &str)> {
        #[cfg(feature = "persistence")]
        if let RuntimeAuthorityKind::Delegated {
            principal_id,
            session_id,
        } = &self.kind
        {
            return Some((principal_id, session_id));
        }
        None
    }
    pub(crate) fn provenance_generation(&self) -> Option<u64> {
        self.generation
    }
    pub(crate) fn resource_revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }
    pub(crate) fn is_deleted(&self) -> bool {
        self.deleted
    }
}
#[cfg(feature = "persistence")]
impl From<crate::db::BehaviorRuntimeAuthority> for RuntimeAuthority {
    fn from(value: crate::db::BehaviorRuntimeAuthority) -> Self {
        Self {
            kind: match value.authority {
                crate::db::BehaviorAuthority::Owner => RuntimeAuthorityKind::Owner,
                crate::db::BehaviorAuthority::Delegated {
                    principal_id,
                    session_id,
                } => RuntimeAuthorityKind::Delegated {
                    principal_id,
                    session_id,
                },
            },
            generation: Some(value.generation as u64),
            revision: Some(value.revision.to_string()),
            deleted: value.deleted,
        }
    }
}
pub(crate) type RuntimeAuthorities = BTreeMap<(String, String), RuntimeAuthority>;

pub(super) fn authority_changed(
    before: &Option<RuntimeAuthorities>,
    after: &Option<RuntimeAuthorities>,
    family: &str,
    id: &str,
) -> bool {
    match (before, after) {
        (None, None) => false,
        (Some(before), Some(after)) => {
            before.get(&(family.into(), id.into())) != after.get(&(family.into(), id.into()))
        }
        _ => true,
    }
}

/// A trusted installation must preserve the body/version/provenance relation.
/// Tombstones have retained authority but never a live body in this snapshot.
pub(super) fn coherent(
    actions: &BTreeMap<String, Value>,
    events: &BTreeMap<String, Value>,
    versions: &[crate::collaboration::ResourceVersion],
    authorities: &RuntimeAuthorities,
) -> bool {
    let mut live_versions = BTreeMap::new();
    for version in versions {
        if version.kind != crate::collaboration::ResourceKind::Automation {
            continue;
        }
        if !matches!(version.scope.as_str(), "action" | "event")
            || live_versions
                .insert(
                    (version.scope.as_str(), version.id.as_str()),
                    version.revision.as_str(),
                )
                .is_some()
        {
            return false;
        }
    }
    for (family, records) in [("action", actions), ("event", events)] {
        for id in records.keys() {
            let Some(authority) = authorities.get(&(family.into(), id.clone())) else {
                return false;
            };
            if authority.is_deleted()
                || authority.resource_revision()
                    != live_versions.get(&(family, id.as_str())).copied()
            {
                return false;
            }
        }
    }
    if live_versions.len() != actions.len() + events.len() {
        return false;
    }
    authorities.iter().all(|((family, id), authority)| {
        let record = match family.as_str() {
            "action" => actions.get(id),
            "event" => events.get(id),
            _ => return false,
        };
        let revision_valid = authority.resource_revision().is_some_and(|revision| {
            revision
                .parse::<i64>()
                .ok()
                .is_some_and(|n| n > 0 && n.to_string() == revision)
        });
        let subject_valid = authority.subject().is_none_or(|(principal, session)| {
            [principal, session].into_iter().all(|id| {
                !id.is_empty()
                    && id.len() <= 256
                    && id.trim() == id
                    && !id.contains('*')
                    && !id.chars().any(char::is_control)
            })
        });
        revision_valid
            && authority
                .provenance_generation()
                .is_some_and(|n| n > 0 && n <= i64::MAX as u64)
            && subject_valid
            && authority.is_deleted() == record.is_none()
    })
}

pub(super) fn owner_allowed(index: &BehaviorIndex, family: &str, id: Option<&str>) -> bool {
    if !index.authority_available {
        return false;
    }
    let Some(id) = id else {
        return true;
    };
    let Some(authorities) = &index.authorities else {
        return true;
    };
    if let Some(owner) = authorities.get(&(family.into(), id.into())) {
        return owner.is_owner();
    }
    // A stored record without provenance is corruption. A new explicit owner
    // inline draft has no stored identity and remains an owner operation.
    match family {
        "event" => !index.events.contains_key(id),
        "action" => !index.actions.contains_key(id),
        _ => false,
    }
}
pub(super) fn owner_record_allowed(index: &BehaviorIndex, record: &Value) -> bool {
    owner_allowed(index, "event", record.get("id").and_then(Value::as_str))
        && record
            .get("actionIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .all(|id| owner_allowed(index, "action", Some(id)))
}
impl AutomationService {
    /// Capture only host-installed sidecar data, including retained delegated
    /// tombstones. None is not Owner authority: it can also mean quarantine or
    /// an identity that has never existed. Editable JSON is not consulted.
    #[cfg(feature = "persistence")]
    pub(crate) fn runtime_authority(
        &self,
        family: crate::collaboration::AutomationFamily,
        id: &str,
    ) -> Option<RuntimeAuthority> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !index.authority_available {
            return None;
        }
        index
            .authorities
            .as_ref()?
            .get(&(family.scope().into(), id.into()))
            .cloned()
    }
    #[cfg(all(test, feature = "persistence"))]
    pub(crate) fn owner_event_allowed(&self, record: &Value) -> bool {
        owner_record_allowed(
            &recover_rwlock_read(&self.index, "automation index"),
            record,
        )
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn owner_hotkey_snapshot(&self) -> Value {
        let index = recover_rwlock_read(&self.index, "automation index");
        serde_json::json!({"events":index.events.values().filter(|r|owner_record_allowed(&index,&r.definition)).map(|r|&r.definition).collect::<Vec<_>>()})
    }
    pub(crate) fn quarantine(&self) {
        let mut index = recover_rwlock_write(&self.index, "automation index");
        #[cfg(feature = "persistence")]
        approval::clear(&mut index);
        index.actions.clear();
        index.events.clear();
        index.catalog_epoch = None;
        index.authorities = None;
        index.authority_available = false;
    }
    pub(crate) fn owner_action_allowed(&self, action: &Value, origin: Option<&Value>) -> bool {
        let index = recover_rwlock_read(&self.index, "automation index");
        owner_allowed(&index, "action", action.get("id").and_then(Value::as_str))
            && origin.is_none_or(|v| owner_record_allowed(&index, v))
    }
}

#[cfg(all(test, feature = "persistence"))]
mod tests;

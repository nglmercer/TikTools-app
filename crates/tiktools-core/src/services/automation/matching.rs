//! One atomic index binds matching to the exact event and action definitions.
use super::authority::owner_record_allowed;
use super::filters::{event_record_matches, number_u64};
use super::snapshot::trigger_allowed;
use super::*;

impl AutomationService {
    pub fn matching_events(&self, event: &Value) -> Vec<Value> {
        self.matching_records(event)
            .into_iter()
            .map(|r| r.definition)
            .collect()
    }
    pub(crate) fn matching_records(&self, event: &Value) -> Vec<MatchedRecord> {
        self.matching_records_for_input(event, false)
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn matching_records_scoped(&self, event: &Value) -> Vec<MatchedRecord> {
        self.matching_records_for_input(
            &crate::services::automation_approval_policy::project_event(event),
            true,
        )
    }
    fn matching_records_for_input(&self, event: &Value, scoped: bool) -> Vec<MatchedRecord> {
        let index = recover_rwlock_read(&self.index, "automation index");
        let Some(catalog_epoch) = index.catalog_epoch else {
            return Vec::new();
        };
        index
            .events
            .values()
            .filter(|r| {
                r.generation.is_some()
                    && r.definition.get("enabled").and_then(Value::as_bool) == Some(true)
                    && r.definition.get("trigger").and_then(Value::as_str)
                        == event.get("type").and_then(Value::as_str)
                    && trigger_allowed(
                        r.definition.get("trigger").and_then(Value::as_str),
                        &index.trigger_owners,
                        &index.enabled_plugins,
                    )
                    && record_authorized(&index, &r.definition)
                    && input_source_allowed(&index, &r.definition, scoped)
                    && (!scoped || public_filter_paths(&r.definition))
                    && record_event_matches(&index, &r.definition, event)
            })
            .map(|r| MatchedRecord {
                definition: r.definition.clone(),
                generation: r.generation,
                catalog_epoch,
                actions: select_actions(&index, &r.definition),
                #[cfg(feature = "persistence")]
                approval: record_approval(&index, &r.definition),
            })
            .collect()
    }
    pub(crate) fn manual_record(&self, record: &Value) -> Option<MatchedRecord> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !record_authorized(&index, record) {
            return None;
        }
        let generation = if owner_record_allowed(&index, record) {
            None
        } else {
            Some(index.events.get(record.get("id")?.as_str()?)?.generation?)
        };
        Some(MatchedRecord {
            definition: record.clone(),
            generation,
            catalog_epoch: index.catalog_epoch?,
            actions: select_actions(&index, record),
            #[cfg(feature = "persistence")]
            approval: record_approval(&index, record),
        })
    }
    pub fn event_record_matches(&self, record: &Value, event: &Value) -> bool {
        event_record_matches(record, event)
    }
    /// Approved filters see the same public scalar projection as renderers.
    /// Unknown/private paths resolve to absence and cannot influence outcomes.
    pub(crate) fn runtime_event_record_matches(&self, record: &Value, event: &Value) -> bool {
        self.scoped_event(record, event)
            .is_some_and(|event| event_record_matches(record, &event))
    }
    pub(super) fn scoped_event<'a>(
        &self,
        record: &Value,
        event: &'a Value,
    ) -> Option<std::borrow::Cow<'a, Value>> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !record_authorized(&index, record) {
            return None;
        }
        if owner_record_allowed(&index, record) {
            return Some(std::borrow::Cow::Borrowed(event));
        }
        #[cfg(feature = "persistence")]
        return Some(std::borrow::Cow::Owned(
            crate::services::automation_approval_policy::project_event(event),
        ));
        #[cfg(not(feature = "persistence"))]
        None
    }
    /// Compatibility body check. Actual queued execution additionally uses an
    /// unforgeable host-side generation token, including delete/recreate ABA.
    pub fn is_record_live(&self, record: &Value) -> bool {
        let index = recover_rwlock_read(&self.index, "automation index");
        record
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| index.events.get(id))
            .is_some_and(|r| {
                r.generation.is_some() && r.definition == *record && record_allowed(&index, record)
            })
    }
    pub(crate) fn is_match_live(&self, record: &MatchedRecord) -> bool {
        let index = recover_rwlock_read(&self.index, "automation index");
        if index.catalog_epoch != Some(record.catalog_epoch)
            || !record_authorized(&index, &record.definition)
            || !record_approval_current(&index, record)
        {
            return false;
        }
        if let Some(generation) = record.generation {
            if !record
                .definition
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| index.events.get(id))
                .is_some_and(|r| {
                    r.generation == Some(generation) && record_allowed(&index, &r.definition)
                })
            {
                return false;
            }
        }
        record.actions.iter().all(|a| {
            a.definition
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| index.actions.get(id))
                .is_some_and(|r| {
                    r.generation == Some(a.generation)
                        && r.definition.get("enabled").and_then(Value::as_bool) != Some(false)
                })
        })
    }
    pub(crate) fn is_step_live(&self, record: &MatchedRecord, action: &MatchedAction) -> bool {
        let index = recover_rwlock_read(&self.index, "automation index");
        if index.catalog_epoch != Some(record.catalog_epoch)
            || !record_authorized(&index, &record.definition)
            || !record_approval_current(&index, record)
        {
            return false;
        }
        if let Some(generation) = record.generation {
            if !record
                .definition
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| index.events.get(id))
                .is_some_and(|r| {
                    r.generation == Some(generation) && record_allowed(&index, &r.definition)
                })
            {
                return false;
            }
        }
        if !record.actions.iter().any(|selected| {
            selected.generation == action.generation && selected.definition == action.definition
        }) {
            return false;
        }
        action
            .definition
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| index.actions.get(id))
            .is_some_and(|r| {
                r.generation == Some(action.generation)
                    && r.definition.get("enabled").and_then(Value::as_bool) != Some(false)
            })
    }
    pub fn actions_for_event(&self, record: &Value) -> Vec<Value> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !record_authorized(&index, record) {
            return Vec::new();
        }
        select_actions(&index, record)
            .into_iter()
            .map(|a| a.definition)
            .collect()
    }
    pub fn emit_depth(&self, event: &Value) -> u64 {
        let index = recover_rwlock_read(&self.index, "automation index");
        let tracked = event
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| t == "plugin.emit" || index.trigger_owners.contains_key(t));
        if !tracked {
            return 0;
        }
        event
            .get("data")
            .and_then(|v| v.get("depth"))
            .and_then(number_u64)
            .unwrap_or_default()
    }
}

fn input_source_allowed(index: &BehaviorIndex, record: &Value, scoped: bool) -> bool {
    #[cfg(feature = "persistence")]
    return scoped
        || record_approval(index, record).is_none_or(|approval| !approval.requires_scoped_input());
    #[cfg(not(feature = "persistence"))]
    {
        let _ = (index, record);
        !scoped
    }
}

/// Reject private/unknown filter paths before evaluating even negative or
/// absence predicates. Scoped input cannot cause an Owner rule to inspect
/// arbitrary fields by selecting a permissive operator.
pub(crate) fn public_filter_paths(record: &Value) -> bool {
    let Some(filters) = record.get("filters") else {
        return true;
    };
    let Some(filters) = filters.as_array() else {
        return false;
    };
    filters.len() <= 256
        && filters.iter().all(|filter| {
            let Some(raw) = filter.get("path").and_then(Value::as_str) else {
                return false;
            };
            if raw.len() > 256 {
                return false;
            }
            let raw = raw
                .trim()
                .trim_start_matches("{{")
                .trim_end_matches("}}")
                .trim();
            let path = raw.strip_prefix("event.").unwrap_or(raw);
            let path = path.strip_prefix("event").unwrap_or(path);
            let parts = path
                .trim_matches('.')
                .split('.')
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            !parts.is_empty()
                && !parts[0].starts_with("event")
                && crate::collaboration::PUBLIC_EFFECT_RENDER_FIELDS
                    .contains(&format!("event.{}", parts.join(".")).as_str())
        })
}
fn record_allowed(index: &BehaviorIndex, record: &Value) -> bool {
    record_authorized(index, record)
        && record.get("enabled").and_then(Value::as_bool) == Some(true)
        && trigger_allowed(
            record.get("trigger").and_then(Value::as_str),
            &index.trigger_owners,
            &index.enabled_plugins,
        )
}
fn record_approval_current(index: &BehaviorIndex, record: &MatchedRecord) -> bool {
    #[cfg(feature = "persistence")]
    return record
        .approval
        .as_ref()
        .is_none_or(|candidate| super::approval::candidate_current(index, candidate));
    #[cfg(not(feature = "persistence"))]
    {
        let _ = (index, record);
        true
    }
}
fn record_authorized(index: &BehaviorIndex, record: &Value) -> bool {
    if owner_record_allowed(index, record) {
        return true;
    }
    #[cfg(feature = "persistence")]
    return super::approval::candidate_for_primary(
        index,
        crate::collaboration::AutomationFamily::Event,
        record,
    )
    .is_some();
    #[cfg(not(feature = "persistence"))]
    false
}
#[cfg(feature = "persistence")]
fn record_approval(
    index: &BehaviorIndex,
    record: &Value,
) -> Option<super::approval::ApprovalCandidate> {
    if owner_record_allowed(index, record) {
        return None;
    }
    super::approval::candidate_for_primary(
        index,
        crate::collaboration::AutomationFamily::Event,
        record,
    )
}
fn record_event_matches(index: &BehaviorIndex, record: &Value, event: &Value) -> bool {
    if owner_record_allowed(index, record) {
        return event_record_matches(record, event);
    }
    #[cfg(feature = "persistence")]
    return event_record_matches(
        record,
        &crate::services::automation_approval_policy::project_event(event),
    );
    #[cfg(not(feature = "persistence"))]
    false
}
fn select_actions(index: &BehaviorIndex, record: &Value) -> Vec<MatchedAction> {
    let mut selected: Vec<_> = record
        .get("actionIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|id| index.actions.get(id))
        .filter(|r| r.definition.get("enabled").and_then(Value::as_bool) != Some(false))
        .filter_map(|r| {
            Some(MatchedAction {
                definition: r.definition.clone(),
                generation: r.generation?,
            })
        })
        .collect();
    if record.get("runMode").and_then(Value::as_str) == Some("random") && selected.len() > 1 {
        let i = fastrand::usize(..selected.len());
        selected = vec![selected.swap_remove(i)];
    }
    selected
}

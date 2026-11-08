//! Owner-compatible versioned record writes. Remote execution authority is a
//! separate gate and remains unavailable until manifests are implemented.
use super::*;
use crate::collaboration::{ResourceKind, Versioned};

impl AppCore {
    pub fn automation_list_versioned(
        &self,
        kind: Option<AutomationKind>,
    ) -> Result<Versioned<AutomationRecords>, OperationError> {
        let mut snapshot = self.load_merged_behavior_versioned()?;
        let events = if kind != Some(AutomationKind::Action) {
            snapshot
                .value
                .get("events")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let actions = if kind != Some(AutomationKind::Event) {
            snapshot
                .value
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        snapshot.resource_versions.retain(|v| {
            v.kind == ResourceKind::Automation && kind.is_none_or(|k| v.scope == k.label())
        });
        Ok(snapshot.map(|_| (events, actions)))
    }
    pub fn automation_get_versioned(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> Result<Versioned<Option<Value>>, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        #[cfg(feature = "persistence")]
        {
            self.db
                .behavior_get_versioned(kind.table(), &id)
                .map_err(automation_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            Ok(Versioned::new(
                self.behavior_records(kind)
                    .into_iter()
                    .find(|v| v.get("id").and_then(Value::as_str) == Some(&id)),
                Vec::new(),
            ))
        }
    }
    pub fn automation_create_versioned(
        &self,
        kind: AutomationKind,
        mut record: Value,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, OperationError> {
        let object = record
            .as_object_mut()
            .ok_or_else(|| OperationError::invalid("automation record must be an object"))?;
        if object
            .get("id")
            .and_then(Value::as_str)
            .is_none_or(|s| s.trim().is_empty())
        {
            object.insert("id".into(), fresh_record_id(kind.label()).into());
        }
        if object
            .get("name")
            .and_then(Value::as_str)
            .is_none_or(|s| s.trim().is_empty())
        {
            object.insert(
                "name".into(),
                match kind {
                    AutomationKind::Event => "Untitled event",
                    AutomationKind::Action => "Untitled action",
                }
                .into(),
            );
        }
        self.write_automation(kind, record, expected, Write::Create, "created")
    }
    pub fn automation_update_versioned(
        &self,
        kind: AutomationKind,
        id: &str,
        mut record: Value,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        record
            .as_object_mut()
            .ok_or_else(|| OperationError::invalid("automation record must be an object"))?
            .insert("id".into(), id.into());
        self.write_automation(kind, record, expected, Write::Update, "updated")
    }
    pub fn automation_save_versioned(
        &self,
        kind: AutomationKind,
        record: &Value,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, OperationError> {
        self.write_automation(kind, record.clone(), expected, Write::Upsert, "saved")
    }
    fn write_automation(
        &self,
        kind: AutomationKind,
        mut record: Value,
        expected: Option<&str>,
        mode: Write,
        change: &str,
    ) -> Result<Versioned<Value>, OperationError> {
        let object = record
            .as_object_mut()
            .ok_or_else(|| OperationError::invalid("automation record must be an object"))?;
        let id = clean_record_id(
            object.get("id").and_then(Value::as_str).unwrap_or_default(),
            kind.label(),
        )?;
        if !object
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
        {
            return Err(OperationError::invalid("automation record needs a name"));
        }
        object.insert("id".into(), id.clone().into());
        let _guard = self.automation_gate()?;
        self.ensure_automation_writable(expected)?;
        #[cfg(feature = "persistence")]
        let result = self
            .db
            .save_behavior_versioned(
                kind.table(),
                &record,
                expected,
                match mode {
                    Write::Upsert => crate::db::BehaviorSave::Upsert,
                    Write::Create => crate::db::BehaviorSave::Create,
                    Write::Update => crate::db::BehaviorSave::Update,
                },
            )
            .map_err(automation_error)?;
        #[cfg(not(feature = "persistence"))]
        let result = {
            let present = self
                .behavior_records(kind)
                .iter()
                .any(|v| v.get("id").and_then(Value::as_str) == Some(&id));
            if matches!(mode, Write::Create) && present {
                return Err(OperationError::conflict("automation record already exists"));
            }
            if matches!(mode, Write::Update) && !present {
                return Err(OperationError::not_found(
                    "automation record does not exist",
                ));
            }
            match kind {
                AutomationKind::Event => self.automation.upsert_event(record.clone()),
                AutomationKind::Action => self.automation.upsert_action(record.clone()),
            };
            Versioned::new(record, Vec::new())
        };
        #[cfg(feature = "persistence")]
        self.refresh_automation_snapshot_locked()?;
        self.automation_changed(kind, &id, change);
        Ok(result)
    }
    pub fn automation_delete_versioned(
        &self,
        kind: AutomationKind,
        id: &str,
        expected: Option<&str>,
    ) -> Result<Versioned<bool>, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        let _guard = self.automation_gate()?;
        self.ensure_automation_writable(expected)?;
        #[cfg(feature = "persistence")]
        let result = self
            .db
            .delete_behavior_versioned(kind.table(), &id, expected)
            .map_err(automation_error)?;
        #[cfg(not(feature = "persistence"))]
        let result = {
            match kind {
                AutomationKind::Event => self.automation.remove_event(&id),
                AutomationKind::Action => self.automation.remove_action(&id),
            };
            Versioned::new(true, Vec::new())
        };
        #[cfg(feature = "persistence")]
        self.refresh_automation_snapshot_locked()?;
        if result.value {
            self.automation_changed(kind, &id, "deleted");
        }
        Ok(result)
    }
    pub fn automation_set_enabled_versioned(
        &self,
        kind: AutomationKind,
        id: &str,
        enabled: bool,
        expected: Option<&str>,
    ) -> Result<Versioned<Value>, OperationError> {
        let id = clean_record_id(id, kind.label())?;
        let _guard = self.automation_gate()?;
        self.ensure_automation_writable(expected)?;
        #[cfg(feature = "persistence")]
        let result = self
            .db
            .set_behavior_enabled_versioned(kind.table(), &id, enabled, expected)
            .map_err(automation_error)?;
        #[cfg(not(feature = "persistence"))]
        let result = {
            let mut record = self.automation_get(kind, &id)?;
            record["enabled"] = enabled.into();
            match kind {
                AutomationKind::Event => self.automation.upsert_event(record.clone()),
                AutomationKind::Action => self.automation.upsert_action(record.clone()),
            };
            Versioned::new(record, Vec::new())
        };
        #[cfg(feature = "persistence")]
        self.refresh_automation_snapshot_locked()?;
        self.automation_changed(kind, &id, if enabled { "enabled" } else { "disabled" });
        Ok(result)
    }
    pub fn automation_set_enablement_versioned(
        &self,
        updates: &[(AutomationKind, String, bool)],
    ) -> Result<Versioned<AutomationSetEnablementResult>, OperationError> {
        if updates.is_empty() {
            return Ok(Versioned::new(
                AutomationSetEnablementResult {
                    updated: 0,
                    missing: Vec::new(),
                },
                Vec::new(),
            ));
        }
        let updates: Vec<_> = updates
            .iter()
            .map(|(kind, id, enabled)| {
                clean_record_id(id, kind.label()).map(|id| (*kind, id, *enabled))
            })
            .collect::<Result<_, _>>()?;
        let _guard = self.automation_gate()?;
        self.ensure_automation_writable(None)?;
        #[cfg(feature = "persistence")]
        let (missing, versions) = {
            let refs: Vec<_> = updates
                .iter()
                .map(|(k, id, e)| (k.table(), id.as_str(), *e))
                .collect();
            let result = self
                .db
                .set_behavior_enablement(&refs)
                .map_err(automation_error)?;
            self.refresh_automation_snapshot_locked()?;
            (
                result
                    .value
                    .into_iter()
                    .map(|(kind, id)| AutomationEnablementMiss { kind, id })
                    .collect::<Vec<_>>(),
                result.resource_versions,
            )
        };
        #[cfg(not(feature = "persistence"))]
        let (missing, versions) = {
            let mut missing = Vec::new();
            for (kind, id, enabled) in &updates {
                match self.automation_get(*kind, id) {
                    Ok(mut record) => {
                        record["enabled"] = (*enabled).into();
                        match kind {
                            AutomationKind::Event => self.automation.upsert_event(record),
                            AutomationKind::Action => self.automation.upsert_action(record),
                        }
                    }
                    Err(_) => missing.push(AutomationEnablementMiss {
                        kind: kind.label().into(),
                        id: id.clone(),
                    }),
                }
            }
            (missing, Vec::new())
        };
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: "behavior".into(),
            id: String::new(),
            change: "enablement".into(),
        });
        Ok(Versioned::new(
            AutomationSetEnablementResult {
                updated: updates.len() - missing.len(),
                missing,
            },
            versions,
        ))
    }
    pub(crate) fn automation_gate(&self) -> Result<AutomationGuard<'_>, OperationError> {
        let lock = self
            .automation_mutations
            .lock()
            .map_err(|_| OperationError::internal("automation coordination is unavailable"))?;
        let epoch = self
            .automation_write_epoch
            .load(std::sync::atomic::Ordering::Acquire);
        if !epoch.is_multiple_of(2) || epoch >= u64::MAX - 1 {
            return Err(OperationError::internal("automation epoch is unavailable"));
        }
        // The mutex excludes other writers; AcqRel also orders subsequent
        // definition writes after publication of the odd in-progress epoch.
        self.automation_write_epoch
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Ok(AutomationGuard {
            core: self,
            _lock: lock,
        })
    }
    pub(super) fn ensure_automation_writable(
        &self,
        expected: Option<&str>,
    ) -> Result<(), OperationError> {
        if self.is_shutdown() {
            return Err(OperationError::capability_unavailable(
                "host is shutting down",
            ));
        }
        #[cfg(not(feature = "persistence"))]
        if expected.is_some() {
            return Err(OperationError::capability_unavailable(
                "automation versions require persistence",
            ));
        }
        let _ = expected;
        Ok(())
    }
    pub(super) fn automation_changed(&self, kind: AutomationKind, id: &str, change: &str) {
        self.events.publish_domain(DomainEvent::WorkflowChanged {
            kind: kind.label().into(),
            id: id.into(),
            change: change.into(),
        });
    }
}
pub(crate) struct AutomationGuard<'a> {
    core: &'a AppCore,
    _lock: std::sync::MutexGuard<'a, ()>,
}
impl Drop for AutomationGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.core
                .automation_write_epoch
                .store(u64::MAX, std::sync::atomic::Ordering::Release);
        } else {
            self.core
                .automation_write_epoch
                .fetch_add(1, std::sync::atomic::Ordering::Release);
        }
        self.core.automation_write_settled.notify_waiters();
    }
}
enum Write {
    Upsert,
    Create,
    Update,
}
#[cfg(feature = "persistence")]
pub(super) fn automation_error(error: crate::db::DatabaseError) -> OperationError {
    match error {
        crate::db::DatabaseError::RevisionConflict(v) => OperationError {
            code: "revision_conflict",
            message: "automation revision changed".into(),
            data: Some(json!({"resourceVersions":[v]})),
        },
        crate::db::DatabaseError::NotFound => {
            OperationError::not_found("automation record does not exist")
        }
        crate::db::DatabaseError::Invalid(_) => {
            OperationError::invalid("invalid automation record or version")
        }
        other => OperationError::internal(other.to_string()),
    }
}

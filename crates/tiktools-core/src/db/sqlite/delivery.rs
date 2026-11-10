//! Bounded metadata-only delivery. No path here invokes a business command.
use super::{util::now, DatabaseError, DatabaseManager};
use crate::collaboration::{
    hash_secret, AuditRecord, CommandReceipt, CommandResource, CommandState, CoordinationEvent,
    ResourceKind, ResourceVersion,
};
use rusqlite::{params, Connection, OptionalExtension};

const BATCH_PER_QUEUE: usize = 50;
// Automatic LIVE awards need more throughput than human workflow edits. Each
// database and each command queue keeps an independent bounded allowance.
const POINTS_RESOURCES_PER_BATCH: usize = 256;
const MAX_DELIVERY_BATCH: usize = BATCH_PER_QUEUE * 3 + POINTS_RESOURCES_PER_BATCH;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeliveryToken {
    Resource(i64),
    Command(String),
    PointsResource(i64),
    PointsCommand(String),
}
#[derive(Debug, Clone)]
pub(crate) struct PendingDelivery {
    pub token: DeliveryToken,
    pub audit: AuditRecord,
    pub audit_required: bool,
    pub event: CoordinationEvent,
}
struct ResourceRow {
    cursor: i64,
    kind: String,
    scope: String,
    id: String,
    revision: i64,
    change: String,
    timestamp: i64,
    command_event: Option<String>,
}
fn kind(value: String) -> Result<ResourceKind, DatabaseError> {
    Ok(serde_json::from_value(serde_json::Value::String(value))?)
}
fn audit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditRecord> {
    let resource_kind: String = row.get(5)?;
    let kind =
        serde_json::from_value(serde_json::Value::String(resource_kind)).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let event_id: String = row.get(0)?;
    Ok(AuditRecord {
        correlation_id: event_id.clone(),
        event_id,
        timestamp: row.get(1)?,
        principal_id: row.get(2)?,
        session_id: row.get(3)?,
        method: row.get(4)?,
        resource: CommandResource {
            kind,
            scope: row.get(6)?,
            id: row.get(7)?,
        },
        command_id: Some(row.get(8)?),
        before_revision: row.get(9)?,
        after_revision: row.get(10)?,
        result_code: row.get(11)?,
    })
}
const COMMAND_COLUMNS:&str="c.event_id,c.committed_at,c.principal_id,c.session_id,c.method,c.resource_kind,c.resource_scope,c.resource_id,c.command_id,c.before_revision,c.after_revision,c.outcome_code";
fn command_audit(db: &Connection, id: &str) -> Result<AuditRecord, DatabaseError> {
    db.query_row(
        &format!("SELECT {COMMAND_COLUMNS} FROM control_command_outbox c WHERE c.event_id=?1"),
        [id],
        audit_from_row,
    )
    .optional()?
    .ok_or_else(|| DatabaseError::Invalid("command audit reservation is missing".into()))
}
fn legacy_audit(
    row: &ResourceRow,
    resource: &ResourceVersion,
    event_id: &str,
) -> Result<AuditRecord, DatabaseError> {
    let method = match (resource.kind, row.change.as_str()) {
        (ResourceKind::Workflow, "saved") => "workflows.save",
        (ResourceKind::Workflow, "enabled") => "workflows.enable",
        (ResourceKind::Workflow, "disabled") => "workflows.disable",
        (ResourceKind::Workflow, "deleted") => "workflows.delete",
        (ResourceKind::Competition, "paused") => "competitions.pause",
        (ResourceKind::Competition, "cancelled") => "competitions.cancel",
        (ResourceKind::Competition, "created") => "competitions.create",
        (ResourceKind::Competition, "updated") => "competitions.update",
        (ResourceKind::Competition, "deleted") => "competitions.delete",
        (ResourceKind::Competition, "state-changed") => "competitions.state.change",
        (ResourceKind::Viewer, "awarded") => "points.award",
        (ResourceKind::Viewer, "adjusted" | "automation-adjusted") => "points.adjust",
        (ResourceKind::Viewer, "reset") => "points.reset",
        (ResourceKind::Automation, "saved") => "automation.save",
        (ResourceKind::Automation, "adopted") => "automation.adopt",
        (ResourceKind::Automation, "enabled") => "automation.enable",
        (ResourceKind::Automation, "disabled") => "automation.disable",
        (ResourceKind::Automation, "deleted") => "automation.delete",
        (ResourceKind::Automation, "references-updated") => "automation.references.update",
        (ResourceKind::Automation, "library-updated")
            if resource.scope == "library" && resource.id == "catalog" =>
        {
            "automation.library.apply"
        }
        _ => {
            return Err(DatabaseError::Invalid(
                "unclassified resource invalidation".into(),
            ))
        }
    };
    let before = row
        .revision
        .checked_sub(1)
        .filter(|version| *version >= 0)
        .ok_or_else(|| DatabaseError::Invalid("invalid outbox revision".into()))?;
    let actor = if (resource.kind == ResourceKind::Viewer
        && matches!(row.change.as_str(), "awarded" | "automation-adjusted"))
        || (resource.kind == ResourceKind::Competition && row.change == "state-changed")
    {
        "host-runtime"
    } else {
        "local-owner"
    };
    Ok(AuditRecord {
        event_id: event_id.into(),
        timestamp: row.timestamp,
        principal_id: actor.into(),
        session_id: actor.into(),
        method: method.into(),
        resource: CommandResource {
            kind: resource.kind,
            scope: resource.scope.clone(),
            id: resource.id.clone(),
        },
        command_id: None,
        before_revision: before.to_string(),
        after_revision: resource.revision.clone(),
        result_code: "succeeded".into(),
        correlation_id: event_id.into(),
    })
}

impl DatabaseManager {
    /// Each queue gets capacity even while the other is continuously busy.
    pub(crate) fn pending_coordination(&self) -> Result<Vec<PendingDelivery>, DatabaseError> {
        let mut pending = self.pending_coordination_from(false)?;
        pending.extend(self.pending_coordination_from(true)?);
        Ok(pending)
    }

    fn pending_coordination_from(
        &self,
        points: bool,
    ) -> Result<Vec<PendingDelivery>, DatabaseError> {
        let mut db = self.open(&if points {
            self.points_path()
        } else {
            self.automation_path()
        })?;
        let tx = db.transaction()?;
        let resource_limit = if points {
            POINTS_RESOURCES_PER_BATCH
        } else {
            BATCH_PER_QUEUE
        };
        let resources = {
            let mut stmt=tx.prepare("SELECT r.cursor,r.kind,r.scope,r.resource_id,r.revision,r.change,r.committed_at,l.event_id FROM control_resource_outbox r LEFT JOIN control_outbox_delivery d ON d.resource_cursor=r.cursor LEFT JOIN control_command_resources l ON l.resource_cursor=r.cursor WHERE d.resource_cursor IS NULL ORDER BY r.cursor LIMIT ?1")?;
            let rows = stmt
                .query_map([resource_limit as i64], |row| {
                    Ok(ResourceRow {
                        cursor: row.get(0)?,
                        kind: row.get(1)?,
                        scope: row.get(2)?,
                        id: row.get(3)?,
                        revision: row.get(4)?,
                        change: row.get(5)?,
                        timestamp: row.get(6)?,
                        command_event: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let mut pending = Vec::with_capacity(resource_limit + BATCH_PER_QUEUE);
        for row in resources {
            let resource = ResourceVersion {
                kind: kind(row.kind.clone())?,
                scope: row.scope.clone(),
                id: row.id.clone(),
                revision: row.revision.to_string(),
            };
            let event_id = hash_secret(&serde_json::to_string(&(
                "resource",
                if points { "points" } else { "automation" },
                &row.kind,
                &row.scope,
                &row.id,
                row.revision,
            ))?);
            let audit = match &row.command_event {
                Some(id) => command_audit(&tx, id)?,
                None => legacy_audit(&row, &resource, &event_id)?,
            };
            let audit_required = row.command_event.is_some()
                || !points
                || !matches!(row.change.as_str(), "awarded" | "automation-adjusted");
            let event = CoordinationEvent::Invalidated {
                event_id,
                resource,
                principal_id: audit.principal_id.clone(),
                command_id: audit.command_id.clone(),
                change: row.change,
                committed_at: row.timestamp,
            };
            pending.push(PendingDelivery {
                token: if points {
                    DeliveryToken::PointsResource(row.cursor)
                } else {
                    DeliveryToken::Resource(row.cursor)
                },
                audit,
                audit_required,
                event,
            });
        }
        {
            let mut stmt=tx.prepare(&format!("SELECT {COMMAND_COLUMNS},s.state FROM control_command_outbox c JOIN control_commands s ON s.principal_id=c.principal_id AND s.command_id=c.command_id WHERE c.dispatched_at IS NULL ORDER BY c.committed_at,c.event_id LIMIT ?1"))?;
            for row in stmt.query_map([BATCH_PER_QUEUE as i64], |row| {
                Ok((audit_from_row(row)?, row.get::<_, String>(12)?))
            })? {
                let (audit, state) = row?;
                let state: CommandState = serde_json::from_value(serde_json::Value::String(state))?;
                let resource = ResourceVersion {
                    kind: audit.resource.kind,
                    scope: audit.resource.scope.clone(),
                    id: audit.resource.id.clone(),
                    revision: audit.after_revision.clone(),
                };
                let receipt = CommandReceipt {
                    command_id: audit
                        .command_id
                        .clone()
                        .ok_or_else(|| DatabaseError::Invalid("command ID is missing".into()))?,
                    state,
                    resource: Some(audit.resource.clone()),
                    committed_at: Some(audit.timestamp),
                };
                let event = CoordinationEvent::CommandChanged {
                    event_id: audit.event_id.clone(),
                    resource,
                    principal_id: audit.principal_id.clone(),
                    command: receipt,
                    committed_at: audit.timestamp,
                };
                pending.push(PendingDelivery {
                    token: if points {
                        DeliveryToken::PointsCommand(audit.event_id.clone())
                    } else {
                        DeliveryToken::Command(audit.event_id.clone())
                    },
                    audit,
                    audit_required: true,
                    event,
                });
            }
        }
        tx.commit()?;
        Ok(pending)
    }
    pub(crate) fn acknowledge_coordination(
        &self,
        tokens: &[DeliveryToken],
    ) -> Result<(), DatabaseError> {
        if tokens.len() > MAX_DELIVERY_BATCH {
            return Err(DatabaseError::Invalid(
                "delivery acknowledgement exceeds batch limit".into(),
            ));
        }
        // Independent business databases cannot share a SQLite commit. A
        // failure after one acknowledgement simply replays the other source's
        // stable event IDs; it never repeats a balance or workflow mutation.
        self.acknowledge_coordination_in(tokens, false)?;
        self.acknowledge_coordination_in(tokens, true)
    }

    fn acknowledge_coordination_in(
        &self,
        tokens: &[DeliveryToken],
        points: bool,
    ) -> Result<(), DatabaseError> {
        let belongs = |token: &DeliveryToken| {
            matches!(
                token,
                DeliveryToken::PointsResource(_) | DeliveryToken::PointsCommand(_)
            ) == points
        };
        if !tokens.iter().any(belongs) {
            return Ok(());
        }
        let mut db = self.open(&if points {
            self.points_path()
        } else {
            self.automation_path()
        })?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let timestamp = now();
        for token in tokens.iter().filter(|token| belongs(token)) {
            match token {
                DeliveryToken::Resource(cursor) | DeliveryToken::PointsResource(cursor) => {
                    tx.execute("INSERT OR IGNORE INTO control_outbox_delivery(resource_cursor,delivered_at) SELECT cursor,?2 FROM control_resource_outbox WHERE cursor=?1",params![cursor,timestamp])?;
                }
                DeliveryToken::Command(id) | DeliveryToken::PointsCommand(id) => {
                    tx.execute("UPDATE control_command_outbox SET dispatched_at=coalesce(dispatched_at,?2) WHERE event_id=?1",params![id,timestamp])?;
                }
            }
        }
        // Once both sinks have acknowledged, source metadata is disposable.
        // Keeping only pending rows prevents every idle poll scanning history.
        // Audit retention and command-ID deduplication live in separate tables.
        tx.execute("DELETE FROM control_resource_outbox WHERE cursor IN (SELECT resource_cursor FROM control_outbox_delivery) AND NOT EXISTS(SELECT 1 FROM control_command_resources l JOIN control_command_outbox c ON c.event_id=l.event_id WHERE l.resource_cursor=control_resource_outbox.cursor AND c.dispatched_at IS NULL)",[])?;
        tx.execute("DELETE FROM control_command_outbox WHERE dispatched_at IS NOT NULL AND NOT EXISTS(SELECT 1 FROM control_command_resources l LEFT JOIN control_outbox_delivery d ON d.resource_cursor=l.resource_cursor WHERE l.event_id=control_command_outbox.event_id AND d.resource_cursor IS NULL)",[])?;
        tx.commit()?;
        Ok(())
    }
}

//! Closed status projections. Arbitrary behavior payload JSON never leaves the
//! owner through these reads, and is never even materialized to build them.
use super::*;
use crate::collaboration::{
    AutomationFamily, AutomationSummary, Grant, ResourceKind, ResourceVersion,
};

fn table(family: AutomationFamily) -> &'static str {
    match family {
        AutomationFamily::Action => "behavior_actions",
        AutomationFamily::Event => "behavior_events",
    }
}
fn select(family: AutomationFamily) -> String {
    format!(
        "SELECT CASE WHEN length(CAST(b.id AS BLOB))<=128 THEN b.id ELSE NULL END,
        substr(b.name,1,160),b.enabled,v.revision FROM {} b JOIN control_resource_versions v
        ON v.kind='automation' AND v.scope=?1 AND v.resource_id=b.id AND v.deleted=0",
        table(family)
    )
}
fn summary(
    row: &rusqlite::Row<'_>,
    family: AutomationFamily,
) -> Result<(Value, ResourceVersion), DatabaseError> {
    let id: Option<String> = row.get(0)?;
    let id = id.ok_or(DatabaseError::SnapshotTooLarge)?;
    let revision: i64 = row.get(3)?;
    if revision <= 0
        || id.trim() != id
        || id.is_empty()
        || id.contains('*')
        || id.chars().any(char::is_control)
    {
        return Err(DatabaseError::Invalid(
            "invalid automation projection identity".into(),
        ));
    }
    let value = AutomationSummary {
        kind: family,
        id: id.clone(),
        name: row.get(1)?,
        enabled: row.get(2)?,
        definition_redacted: true,
    };
    Ok((
        serde_json::to_value(value)?,
        ResourceVersion {
            kind: ResourceKind::Automation,
            scope: family.scope().into(),
            id,
            revision: revision.to_string(),
        },
    ))
}
fn append(
    records: &mut Vec<(Value, ResourceVersion)>,
    used: &mut usize,
    row: (Value, ResourceVersion),
    page: usize,
    total: usize,
) -> Result<(), DatabaseError> {
    let bytes = serde_json::to_vec(&row.0)?.len() + serde_json::to_vec(&row.1)?.len() + 2;
    *used = used
        .checked_add(bytes)
        .ok_or(DatabaseError::SnapshotTooLarge)?;
    if bytes > page || *used > total.min(crate::collaboration::SNAPSHOT_BYTES) {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    records.push(row);
    Ok(())
}

impl DatabaseManager {
    pub(crate) fn load_behavior_summaries(
        &self,
        grant: &Grant,
        family: Option<AutomationFamily>,
        page: usize,
        total: usize,
    ) -> Result<Vec<(Value, ResourceVersion)>, DatabaseError> {
        grant
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid automation grant".into()))?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let mut records = Vec::new();
        let mut used = 0;
        for kind in [AutomationFamily::Event, AutomationFamily::Action] {
            if family.is_some_and(|f| f != kind) {
                continue;
            }
            if let Some(ids) = grant.resources.get(&ResourceKind::Automation) {
                // Restricted reads perform indexed lookups only. Never load a
                // hidden record's label/payload while filtering the full table.
                let mut stmt = tx.prepare(&format!("{} WHERE b.id=?2", select(kind)))?;
                for qualified in ids {
                    let Some((scope, id)) = qualified.split_once(':') else {
                        continue;
                    };
                    if scope != kind.scope() || !grant.allows_automation(kind, id) {
                        continue;
                    }
                    let mut rows = stmt.query(params![kind.scope(), id])?;
                    if let Some(row) = rows.next()? {
                        append(&mut records, &mut used, summary(row, kind)?, page, total)?;
                    }
                }
            } else {
                let mut stmt = tx.prepare(&format!("{} ORDER BY b.id", select(kind)))?;
                let mut rows = stmt.query([kind.scope()])?;
                while let Some(row) = rows.next()? {
                    append(&mut records, &mut used, summary(row, kind)?, page, total)?;
                }
            }
        }
        tx.commit()?;
        Ok(records)
    }
    pub(crate) fn behavior_summary(
        &self,
        family: AutomationFamily,
        id: &str,
        bytes: usize,
    ) -> Result<Versioned<Option<Value>>, DatabaseError> {
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let result = {
            let mut stmt = tx.prepare(&format!("{} WHERE b.id=?2", select(family)))?;
            let mut rows = stmt.query(params![family.scope(), id])?;
            match rows.next()? {
                Some(row) => {
                    let (value, version) = summary(row, family)?;
                    Versioned::new(Some(value), vec![version])
                }
                None => Versioned::new(
                    None,
                    vec![versions::version(
                        table(family),
                        id,
                        versions::current(&tx, table(family), id)?,
                    )?],
                ),
            }
        };
        if serde_json::to_vec(&result)?.len() > bytes {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        tx.commit()?;
        Ok(result)
    }
}

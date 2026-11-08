//! Approved automation commits its private execution identity with the balance.
//! The trusted admission callback holds saved authorization through `commit`.
use super::*;
use crate::{
    collaboration::{CommandState, ResourceKind, ResourceVersion, Versioned},
    services::{effects::ApprovedPointsKey, PointAward},
};
use rusqlite::{OptionalExtension, TransactionBehavior};

#[cfg(test)]
mod tests;

// Public principals are generated UUIDs. Runtime identities occupy their own
// reserved namespace, never an author session, host process or caller RPC ID.
const RUNTIME_NAMESPACE: &str = "__approved-points-runtime-v1";
const METHOD: &str = "automation.points.adjust";
// Keep a bounded lifetime identity ledger. Expired outcomes remain tombstones;
// capacity rejection cannot turn old messages back into fresh adjustments.
const MAX_RUNTIME_IDS: u32 = 750_000;
const RESULT_BYTES: i64 = 4_096;
const VERSIONS_BYTES: i64 = 1_024;

type RuntimeCommand<'a> = super::super::commands::CommandTransaction<RuntimeRequest<'a>>;
struct RuntimeRequest<'a>(&'a str);
impl super::super::commands::CommandIdentity for RuntimeRequest<'_> {
    fn command_id(&self) -> &str {
        self.0
    }
    fn host_instance_id(&self) -> &str {
        RUNTIME_NAMESPACE
    }
}

fn runtime_command(key: &ApprovedPointsKey) -> RuntimeCommand<'_> {
    RuntimeCommand {
        principal_id: RUNTIME_NAMESPACE.into(),
        session_id: RUNTIME_NAMESPACE.into(),
        authorization: None,
        request: RuntimeRequest(key.id()),
        current_instance_id: RUNTIME_NAMESPACE.into(),
        max_ids_per_principal: MAX_RUNTIME_IDS,
    }
}

/// Unlike ordinary command recovery this result is private and never returned
/// to a caller or installed into the cache. Only the original effective delta
/// is used for current grant admission on a retained invocation.
fn retained_effective_delta(
    db: &Connection,
    id: &str,
    unique_id: &str,
    delta: f64,
    result: &Versioned<Value>,
) -> Result<f64, DatabaseError> {
    let metadata_valid: bool = db.query_row(
        "SELECT method=?3 AND resource_kind='viewer' AND resource_scope='host' AND resource_id=?4 AND session_id=?1 AND host_instance_id=?1 AND state='succeeded' FROM control_commands WHERE principal_id=?1 AND command_id=?2",
        params![RUNTIME_NAMESPACE,id,METHOD,unique_id], |row| row.get(0),
    )?;
    let Some(value) = result.value.as_object() else {
        return Err(DatabaseError::PointsStateInvalid);
    };
    let Some(award) = value.get("award").and_then(Value::as_object) else {
        return Err(DatabaseError::PointsStateInvalid);
    };
    let award_keys = ["uniqueId", "delta", "totalPoints", "level", "currencyName"];
    let effective = value
        .get("effectiveDelta")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or(DatabaseError::PointsStateInvalid)?;
    if !metadata_valid
        || value.len() != 2
        || award.len() != award_keys.len()
        || award_keys.iter().any(|key| !award.contains_key(*key))
        || award.get("uniqueId").and_then(Value::as_str) != Some(unique_id)
        || award.get("delta").and_then(Value::as_f64) != Some(delta)
        || !award
            .get("totalPoints")
            .and_then(Value::as_f64)
            .is_some_and(|value| value.is_finite() && value >= 0.0)
        || !award
            .get("level")
            .and_then(Value::as_u64)
            .is_some_and(|value| value >= 1 && value <= u64::from(u32::MAX))
        || !award.get("currencyName").is_some_and(Value::is_string)
        || result.resource_versions.len() != 1
        || !result.resource_versions.first().is_some_and(|version| {
            version.kind == ResourceKind::Viewer
                && version.scope == "host"
                && version.id == unique_id
                && version
                    .revision
                    .parse::<i64>()
                    .is_ok_and(|revision| revision > 0 && revision.to_string() == version.revision)
        })
        || !result.command.as_ref().is_some_and(|command| {
            command.command_id == id && command.state == CommandState::Succeeded
        })
        || result.host_instance_id.as_deref() != Some(RUNTIME_NAMESPACE)
    {
        return Err(DatabaseError::PointsStateInvalid);
    }
    Ok(effective)
}

impl DatabaseManager {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn adjust_approved_points(
        &self,
        key: &ApprovedPointsKey,
        unique_id: &str,
        delta: f64,
        config: &PointsConfig,
        timeout: std::time::Duration,
        admit: impl FnOnce(
            f64,
            &mut dyn FnMut() -> Result<(), DatabaseError>,
        ) -> Result<(), DatabaseError>,
    ) -> Result<(Option<PointAward>, Option<ViewerState>), DatabaseError> {
        if !crate::services::valid_viewer_id(unique_id)
            || !delta.is_finite()
            || delta.abs() > 1_000_000_000.0
            || !crate::services::effects::valid_digest(key.id())
            || !crate::services::effects::valid_digest(key.fingerprint())
        {
            return Err(DatabaseError::Invalid(
                "invalid approved points identity or adjustment".into(),
            ));
        }
        // Production callers hold coordination then the automation writer
        // gate, just like public coordinators. Honor the shared ledger cap
        // across both business databases, not only this transaction's store.
        self.check_command_admission(
            super::super::commands::CommandDatabase::Points,
            RUNTIME_NAMESPACE,
            key.id(),
            MAX_RUNTIME_IDS,
        )?;
        let command = runtime_command(key);
        // Defensively bind the actual sink arguments as well as the trusted
        // saved-action fingerprint. They never become the execution identity.
        let hash = super::super::commands::additive_hash(
            METHOD,
            &serde_json::json!({
                "bindingFingerprint":key.fingerprint(),"uniqueId":unique_id,"delta":delta,
            }),
        )?;
        let mut db = self.open(&self.points_path())?;
        db.busy_timeout(timeout)?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Bound every stored string before generic command recovery copies
        // metadata or parses JSON into Rust. Exact resource identity is checked
        // after the semantic hash, so a changed rendered target still reports
        // CommandIdReused rather than masquerading as storage corruption.
        let preflight: Option<(i64, i64, bool)> = tx
            .query_row(
                "SELECT coalesce(length(CAST(result_json AS BLOB)),0),coalesce(length(CAST(versions_json AS BLOB)),0),
                 coalesce(CASE WHEN length(CAST(payload_hash AS BLOB))=64 AND length(payload_hash)=64
                   THEN payload_hash NOT GLOB '*[^0-9a-f]*' ELSE 0 END
                   AND method=?3 AND resource_kind='viewer' AND resource_scope='host'
                   AND length(CAST(resource_id AS BLOB)) BETWEEN 1 AND 128
                   AND session_id=?1 AND host_instance_id=?1 AND state='succeeded',0)
                 FROM control_commands WHERE principal_id=?1 AND command_id=?2",
                params![RUNTIME_NAMESPACE,key.id(),METHOD], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
            )
            .optional()?;
        if preflight.is_some_and(|(result, versions, metadata_valid)| {
            !metadata_valid
                || !(0..=RESULT_BYTES).contains(&result)
                || !(0..=VERSIONS_BYTES).contains(&versions)
        }) {
            return Err(DatabaseError::PointsStateInvalid);
        }
        if let Some(retained) = super::super::commands::replay(&tx, &command, &hash)? {
            let effective = retained_effective_delta(&tx, key.id(), unique_id, delta, &retained)?;
            let mut tx = Some(tx);
            let mut committed = false;
            admit(effective, &mut || {
                let tx = tx.take().ok_or_else(|| {
                    DatabaseError::Invalid("approval admission already consumed".into())
                })?;
                tx.commit()?;
                committed = true;
                Ok(())
            })?;
            if !committed {
                return Err(DatabaseError::Invalid(
                    "approval admission did not commit".into(),
                ));
            }
            return Ok((None, None));
        }
        let mut viewer =
            commands::bounded_viewer(&tx, unique_id, 16 * 1024)?.ok_or(DatabaseError::NotFound)?;
        let before = commands::revision(&tx, unique_id)?;
        // Match existing core.points semantics: existing viewers only, with no
        // subscriber bonus. Also admit the actual rounded/clamped delta.
        let total = (((viewer.points + delta).max(0.0)) * 100.0).round() / 100.0;
        let effective = total - viewer.points;
        if !total.is_finite() || !effective.is_finite() {
            return Err(DatabaseError::Invalid(
                "invalid approved points arithmetic".into(),
            ));
        }
        viewer.points = total;
        viewer.level =
            ((total / config.points_per_level.max(10.0)).floor() as u32).saturating_add(1);
        viewer.last_seen = now();
        let award = PointAward {
            unique_id: unique_id.into(),
            delta,
            total_points: total,
            level: viewer.level,
            currency_name: config.currency_name.clone(),
        };
        let stored = serde_json::json!({"effectiveDelta":effective,"award":award});
        if serde_json::to_vec(&stored)?.len() > RESULT_BYTES as usize {
            return Err(DatabaseError::PointsStateInvalid);
        }
        let op = PointsWriteOp::Adjust {
            seq: 0,
            unique_id: unique_id.into(),
            points: total,
            level: viewer.level,
        };
        let mut tx = Some(tx);
        let mut committed = false;
        admit(effective, &mut || {
            let tx = tx.take().ok_or_else(|| {
                DatabaseError::Invalid("approval admission already consumed".into())
            })?;
            apply_points_op_on(&tx, &op)?;
            versions::Reservations::new(&tx)?.reserve(&tx, &op)?;
            let result = Versioned::new(
                stored.clone(),
                vec![ResourceVersion {
                    kind: ResourceKind::Viewer,
                    scope: "host".into(),
                    id: unique_id.into(),
                    revision: commands::revision(&tx, unique_id)?.to_string(),
                }],
            );
            super::super::commands::prune(&tx)?;
            super::super::commands::finish(
                &tx,
                &command,
                &hash,
                METHOD,
                &before.to_string(),
                result,
            )?;
            #[cfg(test)]
            if std::env::var("TIKTOOLS_APPROVED_POINTS_CRASH_STAGE").as_deref()
                == Ok("before_commit")
            {
                std::process::exit(98);
            }
            tx.commit()?;
            committed = true;
            #[cfg(test)]
            if std::env::var("TIKTOOLS_APPROVED_POINTS_CRASH_STAGE").as_deref()
                == Ok("after_commit")
            {
                std::process::exit(99);
            }
            Ok(())
        })?;
        if !committed {
            return Err(DatabaseError::Invalid(
                "approval admission did not commit".into(),
            ));
        }
        Ok((Some(award), Some(viewer)))
    }
}

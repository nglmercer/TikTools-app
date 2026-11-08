//! Owner source and retained approval identity share a single read snapshot.
//! The returned binding is private evidence for comparison, never a permit.
use super::*;
use crate::control::{OwnerApprovalStatus, OwnerApprovalStatusKind};

pub(crate) struct BehaviorApprovalReviewSnapshot {
    pub status: OwnerApprovalStatus,
    pub binding: Value,
}

impl DatabaseManager {
    pub(crate) fn behavior_approval_review_versioned(
        &self,
        table: &str,
        id: &str,
    ) -> Result<(Versioned<Value>, Option<BehaviorApprovalReviewSnapshot>), DatabaseError> {
        let scope = versions::scope(table)?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction()?;
        let source = review::snapshot(&tx, table, id)?;
        let mut query = tx.prepare("SELECT state,generation,policy_version,manifest_digest,length(CAST(binding_json AS BLOB)) FROM behavior_execution_approvals WHERE scope=?1 AND resource_id=?2")?;
        let mut rows = query.query(params![scope, id])?;
        let approval = if let Some(row) = rows.next()? {
            let invalid =
                || DatabaseError::Invalid("retained automation approval is inconsistent".into());
            for (column, limit) in [(0, 11usize), (3, 64)] {
                match row.get_ref(column)? {
                    rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= limit => {}
                    _ => return Err(invalid()),
                }
            }
            let state = match row.get::<_, String>(0)?.as_str() {
                "active" => OwnerApprovalStatusKind::Active,
                "invalidated" => OwnerApprovalStatusKind::Invalidated,
                "revoked" => OwnerApprovalStatusKind::Revoked,
                _ => return Err(invalid()),
            };
            let generation: i64 = row.get(1)?;
            let policy_version: u32 = row.get(2)?;
            let manifest_digest: String = row.get(3)?;
            let bytes: i64 = row.get(4)?;
            if generation <= 0 || policy_version == 0 || !(1..=4 * 1024 * 1024).contains(&bytes) {
                return Err(invalid());
            }
            let encoded: String = tx.query_row("SELECT binding_json FROM behavior_execution_approvals WHERE scope=?1 AND resource_id=?2", params![scope,id], |row| row.get(0))?;
            let binding: Value = serde_json::from_str(&encoded)?;
            approvals::canonical(&binding, 0)?;
            if !binding.as_object().is_some_and(|object| {
                object.len() == 2
                    && object.contains_key("manifest")
                    && object.get("bindings").is_some_and(Value::is_object)
            }) {
                return Err(invalid());
            }
            let manifest = &binding["manifest"];
            let (stored_policy, stored_digest, _) = approvals::validate_manifest(manifest)?;
            if stored_policy != policy_version
                || stored_digest != manifest_digest
                || manifest["primary"]["kind"].as_str() != Some(scope)
                || manifest["primary"]["id"].as_str() != Some(id)
            {
                return Err(invalid());
            }
            if state == OwnerApprovalStatusKind::Active {
                approvals::validate_reverse_dependencies(&tx, scope, id, manifest)?;
                approvals::validate_review(manifest, &source)?;
            }
            Some(BehaviorApprovalReviewSnapshot {
                status: OwnerApprovalStatus {
                    state,
                    generation: Some(generation.to_string()),
                    policy_version: Some(policy_version),
                    manifest_digest: Some(manifest_digest),
                    current: false,
                },
                binding,
            })
        } else {
            None
        };
        drop(rows);
        drop(query);
        tx.commit()?;
        Ok((source, approval))
    }
}

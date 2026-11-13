//! Persistent execution-subject lookup. Call on a blocking pool.
use super::*;
use crate::{ExecutionAuthorization, ExecutionAuthorizations, ExecutionIdentity};
use std::collections::BTreeSet;

const SUBJECTS: usize = 513; // One primary definition and all 512 references.
const GRANT_BYTES: usize = 512 * 1024 - 2048;
const BATCH_BYTES: usize = 4 * 1024 * 1024;

fn canonical(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|value| value.to_string() == id)
}

impl CollaborationService {
    /// Reconstruct a saved subject from trusted host provenance, without a
    /// bearer token, socket or activity touch. This is not a Control API route.
    pub fn execution_authorization(
        self: &Arc<Self>,
        principal_id: &str,
        session_id: &str,
    ) -> Result<ExecutionAuthorization, Error> {
        let batch = self.execution_authorizations(&[(principal_id.into(), session_id.into())])?;
        Ok(ExecutionAuthorization { batch })
    }

    /// At most 513 supplied subjects, deduplicated in first-occurrence order.
    /// Every nonempty subject must be current; one invalid binding denies all.
    /// Empty input is an explicit empty set and conveys no remote authority.
    pub fn execution_authorizations(
        self: &Arc<Self>,
        identities: &[(String, String)],
    ) -> Result<ExecutionAuthorizations, Error> {
        if identities.len() > SUBJECTS {
            return Err(Error::TooLarge);
        }
        let mut seen = BTreeSet::new();
        let mut selected = Vec::new();
        for (principal, session) in identities {
            if !canonical(principal) || !canonical(session) {
                return Err(Error::Unauthorized);
            }
            if seen.insert((principal.as_str(), session.as_str())) {
                selected.push((principal.clone(), session.clone()));
            }
        }
        let authorization = ExecutionAuthorizations {
            service: self.clone(),
            identities: selected,
        };
        authorization.with_current(|_| ())?;
        Ok(authorization)
    }

    pub(crate) fn with_execution_identities<R>(
        &self,
        subjects: &[(String, String)],
        apply: impl FnOnce(&[ExecutionIdentity]) -> R,
    ) -> Result<R, Error> {
        // A trusted all-Owner manifest has no remote subjects. An empty list
        // must remain empty even when collaboration itself is disabled.
        if subjects.is_empty() {
            return Ok(apply(&[]));
        }
        if subjects.len() > SUBJECTS {
            return Err(Error::TooLarge);
        }
        let db = self.db()?;
        enabled(&db)?;
        let configuration_revision = config_revision(&db)?.to_string();
        let at = now();
        let mut bytes = 0usize;
        let mut identities = Vec::with_capacity(subjects.len());
        for (principal, session) in subjects {
            let raw:Option<(Option<String>,i64,String,i64,i64)>=db.query_row(
                "SELECT CASE WHEN length(CAST(p.grant_json AS BLOB))<=?4 THEN p.grant_json ELSE NULL END,p.revision,s.kind,s.absolute_expires,s.last_activity FROM sessions s JOIN principals p ON p.id=s.principal_id LEFT JOIN cloud_peer_bindings b ON b.session_id=s.id WHERE s.id=?1 AND s.principal_id=?2 AND s.kind=p.kind AND s.revoked=0 AND p.revoked=0 AND ((b.session_id IS NOT NULL AND b.valid_until>0) OR (b.session_id IS NULL AND s.absolute_expires>?3 AND s.last_activity>?5 AND s.last_activity>=0))",
                params![session,principal,at,GRANT_BYTES as i64,at.saturating_sub(IDLE_TTL)],
                |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
            ).optional()?;
            let (raw, revision, kind, absolute, last_activity) = raw.ok_or(Error::Unauthorized)?;
            let raw = raw.ok_or(Error::TooLarge)?;
            bytes = bytes
                .checked_add(raw.len())
                .filter(|b| *b <= BATCH_BYTES)
                .ok_or(Error::TooLarge)?;
            if revision <= 0 {
                return Err(Error::Unauthorized);
            }
            let grant: Grant = serde_json::from_str(&raw)?;
            grant.validate()?;
            let peer_lease = peers::lease::for_session(&db, session)?;
            let (absolute, idle) = if let Some(lease) = &peer_lease {
                let until = now().saturating_add(lease.remaining()?);
                (until, until)
            } else {
                (
                    absolute,
                    last_activity
                        .checked_add(IDLE_TTL)
                        .ok_or(Error::Unauthorized)?,
                )
            };
            identities.push(ExecutionIdentity {
                host_id: self.host_id.clone(),
                principal_id: principal.clone(),
                session_id: session.clone(),
                client_kind: parse_kind(&kind)?,
                grant,
                grant_revision: revision.to_string(),
                configuration_revision: configuration_revision.clone(),
                absolute_expires_at: absolute,
                idle_expires_at: idle,
                peer_lease,
            });
        }
        // Earlier subjects may have expired while a large batch was parsed.
        for identity in &identities {
            identity.check_deadline()?;
        }
        Ok(apply(&identities))
    }
}

#[cfg(test)]
mod tests;

//! Retained operator provenance and current-subject transaction admission.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};

impl DatabaseManager {
    /// An unretained stop caller must never cause an effect under someone
    /// else's authority, including when a generation is at its subject limit.
    pub(crate) fn competition_retains_operator(
        &self,
        id: &str,
        generation: u64,
        principal: &str,
        session: &str,
    ) -> Result<bool, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        if generation <= 0 {
            return Ok(false);
        }
        let db = self.open(&self.automation_path())?;
        Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM competition_execution_subjects s JOIN competition_execution_generations g ON g.competition_id=s.competition_id AND g.generation=s.generation WHERE s.competition_id=?1 AND s.generation=?2 AND s.principal_id=?3 AND s.session_id=?4 AND g.origin='delegated')", params![id,generation,principal,session], |row| row.get(0))?)
    }

    /// Missing or malformed provenance is never treated as local authority.
    pub(crate) fn competition_execution_is_owner(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<bool, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid competition generation".into()))?;
        if generation <= 0 {
            return Err(DatabaseError::Invalid(
                "invalid competition generation".into(),
            ));
        }
        let connection = self.open(&self.automation_path())?;
        let origin: Option<(String, bool)> = connection.query_row(
            "SELECT origin,EXISTS(SELECT 1 FROM competition_execution_subjects s WHERE s.competition_id=g.competition_id AND s.generation=g.generation) FROM competition_execution_generations g WHERE competition_id=?1 AND generation=?2",
            params![id,generation], |row| Ok((row.get(0)?,row.get(1)?)),
        ).optional()?;
        match origin {
            Some((ref kind, false)) if kind == "owner" => Ok(true),
            Some((ref kind, true)) if kind == "delegated" => Ok(false),
            _ => Err(DatabaseError::Invalid(
                "competition execution authority missing".into(),
            )),
        }
    }

    /// Resolve every retained operator under one business writer and current
    /// collaboration lock, keeping both until commit. The closure may write
    /// only through the supplied transaction; it must not await, dispatch an
    /// external effect or reenter either DB. This is identity/scope admission,
    /// not permission to run an effect after the transaction.
    #[allow(dead_code)]
    pub(crate) fn with_competition_delegation<R>(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        id: &str,
        generation: u64,
        apply: impl FnOnce(
            &rusqlite::Transaction<'_>,
            &[crate::collaboration::ExecutionIdentity],
        ) -> Result<R, DatabaseError>,
    ) -> Result<R, DatabaseError> {
        self.with_competition_delegation_and_subjects(
            service,
            id,
            generation,
            &[],
            |tx, identities, _| apply(tx, identities),
        )
    }

    /// Additional subjects must come from trusted saved automation definitions.
    /// All identities are kept current together until the transaction commits.
    pub(super) fn with_competition_delegation_and_subjects<R>(
        &self,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        id: &str,
        generation: u64,
        additional: &[(String, String)],
        apply: impl FnOnce(
            &rusqlite::Transaction<'_>,
            &[crate::collaboration::ExecutionIdentity],
            usize,
        ) -> Result<R, DatabaseError>,
    ) -> Result<R, DatabaseError> {
        use crate::collaboration::{Capability, Error, ResourceKind};
        let denied = || DatabaseError::CommandAuthorization(Error::Forbidden);
        let generation = i64::try_from(generation)
            .ok()
            .filter(|g| *g > 0)
            .ok_or_else(denied)?;
        let mut db = self.open(&self.automation_path())?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let origin: Option<String> = tx.query_row(
            "SELECT origin FROM competition_execution_generations WHERE competition_id=?1 AND generation=?2",
            params![id, generation], |r| r.get(0),
        ).optional()?;
        if origin.as_deref() != Some("delegated") {
            return Err(denied());
        }
        let mut statement = tx.prepare("SELECT principal_id,session_id,grant_revision FROM competition_execution_subjects WHERE competition_id=?1 AND generation=?2 ORDER BY principal_id,session_id LIMIT 17")?;
        let entries = statement
            .query_map(params![id, generation], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        if entries.is_empty()
            || entries.len() > 16
            || entries.iter().any(|(_, _, revision)| {
                revision
                    .parse::<i64>()
                    .ok()
                    .is_none_or(|n| n <= 0 || n.to_string() != *revision)
            })
        {
            return Err(denied());
        }
        let mut subjects = entries
            .iter()
            .map(|(principal, session, _)| (principal.clone(), session.clone()))
            .collect::<Vec<_>>();
        for subject in additional {
            if !subjects.contains(subject) {
                subjects.push(subject.clone());
            }
        }
        let authorization = service
            .execution_authorizations(&subjects)
            .map_err(DatabaseError::CommandAuthorization)?;
        let result = authorization
            .with_current(|identities| {
                if identities.len() != subjects.len()
                    || identities.iter().zip(&entries).any(
                        |(identity, (principal, session, revision))| {
                            let current = identity.grant_revision().parse::<i64>().ok();
                            let original = revision.parse::<i64>().ok();
                            identity.principal_id() != principal
                                || identity.session_id() != session
                                || current < original
                                || !identity
                                    .grant()
                                    .capabilities
                                    .contains(&Capability::CompetitionsRead)
                                || !identity
                                    .grant()
                                    .capabilities
                                    .contains(&Capability::CompetitionsManage)
                                || identity.grant().resources.contains_key(&ResourceKind::Live)
                                || !identity
                                    .grant()
                                    .allows_resource(ResourceKind::Competition, id)
                                || identity.check_deadline().is_err()
                        },
                    )
                {
                    return Err(denied());
                }
                let result = apply(&tx, identities, entries.len())?;
                for identity in identities {
                    identity
                        .check_deadline()
                        .map_err(DatabaseError::CommandAuthorization)?;
                }
                tx.commit()?;
                Ok(result)
            })
            .map_err(DatabaseError::CommandAuthorization)??;
        Ok(result)
    }
}

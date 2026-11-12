//! Revision-bound delegated lifecycle commands and their retained outcomes.
use super::CompetitionActivation;
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{Definition, Lifecycle, LifecycleState};

impl DatabaseManager {
    /// Caller must hold the ranking and competition mutation barriers and
    /// verify that the ranking is unpaused. No event or effect is dispatched.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_competition_activation(
        &self,
        id: &str,
        generation: u64,
        operation: CompetitionActivation,
        command: &super::super::CommandTransaction,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        owner: &str,
        ranking_paused: bool,
    ) -> Result<(crate::collaboration::Versioned<serde_json::Value>, bool), DatabaseError> {
        use crate::collaboration::{Capability, Error, ResourceKind, ResourceVersion, Versioned};
        let denied = || DatabaseError::CommandAuthorization(Error::Forbidden);
        command
            .request
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid command metadata".into()))?;
        let auth = command.authorization.as_ref().ok_or_else(denied)?;
        let expected_generation = i64::try_from(generation).map_err(|_| denied())?;
        if owner.is_empty() {
            return Err(denied());
        }
        let hash = super::super::commands::payload_hash(
            operation.method(),
            &serde_json::json!({"id":id,"generation":generation.to_string()}),
            &command.request.expected_revision,
        )?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let caller = service
            .execution_authorizations(&[(command.principal_id.clone(), command.session_id.clone())])
            .map_err(DatabaseError::CommandAuthorization)?;
        let check_caller = |identities: &[crate::collaboration::ExecutionIdentity]| {
            let current = auth
                .snapshot()
                .map_err(DatabaseError::CommandAuthorization)?;
            let subject = identities.first().ok_or_else(denied)?;
            if identities.len() != 1
                || current.principal_id != command.principal_id
                || current.session_id != command.session_id
                || current.grant_revision != subject.grant_revision()
                || current.grant != *subject.grant()
                || !current
                    .grant
                    .capabilities
                    .contains(&Capability::CompetitionsRead)
                || !current
                    .grant
                    .capabilities
                    .contains(&Capability::CompetitionsManage)
                || current.grant.resources.contains_key(&ResourceKind::Live)
                || !current.grant.allows_resource(ResourceKind::Competition, id)
            {
                return Err(denied());
            }
            subject
                .check_deadline()
                .map_err(DatabaseError::CommandAuthorization)
        };
        let retained = caller
            .with_current(|identities| {
                check_caller(identities)?;
                super::super::commands::prune(&tx)?;
                super::super::commands::replay(&tx, command, &hash)
            })
            .map_err(DatabaseError::CommandAuthorization)??;
        // A retained outcome belongs to the original command, even if another
        // moderator was later revoked. Only the caller's current scope matters.
        if let Some(result) = retained {
            return caller
                .with_current(|identities| {
                    check_caller(identities)?;
                    tx.commit()?;
                    Ok((result, false))
                })
                .map_err(DatabaseError::CommandAuthorization)?;
        }
        if ranking_paused {
            return Err(DatabaseError::Invalid(
                "competition ranking is paused".into(),
            ));
        }
        let row: Option<(String,String,i64)> = tx.query_row(
            "SELECT d.definition_json,s.state_json,r.revision FROM competition_definitions d JOIN competition_state s ON s.competition_id=d.id JOIN competition_control_revisions r ON r.competition_id=d.id AND r.deleted=0 WHERE d.id=?1",
            [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional()?;
        let (definition, state, revision) = row.ok_or(DatabaseError::NotFound)?;
        let version = |n: i64| ResourceVersion {
            kind: ResourceKind::Competition,
            scope: "host".into(),
            id: id.into(),
            revision: n.to_string(),
        };
        if revision <= 0 || revision.to_string() != command.request.expected_revision {
            return Err(DatabaseError::RevisionConflict(version(revision)));
        }
        let definition: Definition = serde_json::from_str(&definition)?;
        let mut state: LifecycleState = serde_json::from_str(&state)?;
        if state.generation != generation {
            return Err(DatabaseError::RevisionConflict(version(revision)));
        }
        let mut retained = std::collections::BTreeMap::<(String, String), i64>::new();
        if matches!(operation, CompetitionActivation::Resume) {
            let valid_owner:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)",params![id,expected_generation,owner],|r|r.get(0))?;
            if !valid_owner {
                return Err(denied());
            }
            let origin:Option<String>=tx.query_row("SELECT origin FROM competition_execution_generations WHERE competition_id=?1 AND generation=?2",params![id,expected_generation],|r|r.get(0)).optional()?;
            let mut statement=tx.prepare("SELECT principal_id,session_id,grant_revision FROM competition_execution_subjects WHERE competition_id=?1 AND generation=?2 LIMIT 17")?;
            let entries = statement
                .query_map(params![id, expected_generation], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            match origin.as_deref() {
                Some("owner") if entries.is_empty() => {}
                Some("delegated") if !entries.is_empty() => {}
                _ => return Err(denied()),
            }
            for (p, s, r) in entries {
                let revision = r
                    .parse::<i64>()
                    .ok()
                    .filter(|n| *n > 0 && n.to_string() == r)
                    .ok_or_else(denied)?;
                if retained.insert((p, s), revision).is_some() {
                    return Err(denied());
                }
            }
        }
        retained
            .entry((command.principal_id.clone(), command.session_id.clone()))
            .or_insert(0);
        if retained.len() > 16 {
            return Err(denied());
        }
        let subjects = retained.keys().cloned().collect::<Vec<_>>();
        let all = service
            .execution_authorizations(&subjects)
            .map_err(DatabaseError::CommandAuthorization)?;
        all.with_current(|identities| {
            let current=auth.snapshot().map_err(DatabaseError::CommandAuthorization)?;
            if current.principal_id!=command.principal_id || current.session_id!=command.session_id || identities.len()!=retained.len() { return Err(denied()); }
            for subject in identities {
                let prior=*retained.get(&(subject.principal_id().to_owned(),subject.session_id().to_owned())).ok_or_else(denied)?;
                let current_revision=subject.grant_revision().parse::<i64>().ok().filter(|n|*n>0).ok_or_else(denied)?;
                if current_revision<prior || !subject.grant().capabilities.contains(&Capability::CompetitionsRead)
                    || !subject.grant().capabilities.contains(&Capability::CompetitionsManage)
                    || subject.grant().resources.contains_key(&ResourceKind::Live)
                    || !subject.grant().allows_resource(ResourceKind::Competition,id) { return Err(denied()); }
                if subject.principal_id()==command.principal_id && subject.session_id()==command.session_id
                    && (subject.grant_revision()!=current.grant_revision || *subject.grant()!=current.grant) { return Err(denied()); }
                subject.check_deadline().map_err(DatabaseError::CommandAuthorization)?;
            }
            state.transition(operation.transition(),crate::now_millis(),definition.duration_ms).map_err(|_|DatabaseError::Invalid("invalid competition transition".into()))?;
            let next_generation=i64::try_from(state.generation).map_err(|_|denied())?;
            if matches!(operation,CompetitionActivation::Start) {
                tx.execute("INSERT INTO competition_execution_generations VALUES(?1,?2,'delegated')",params![id,next_generation])?;
                tx.execute("INSERT INTO competition_integrity_checkpoints(competition_id,generation,status) VALUES(?1,?2,'open')",params![id,next_generation])?;
                tx.execute("INSERT INTO competition_integrity_owners(competition_id,generation,owner) VALUES(?1,?2,?3)",params![id,next_generation,owner])?;
            } else {
                tx.execute("UPDATE competition_execution_generations SET origin='delegated' WHERE competition_id=?1 AND generation=?2",params![id,next_generation])?;
            }
            tx.execute("INSERT OR IGNORE INTO competition_execution_subjects(competition_id,generation,principal_id,session_id,grant_revision) VALUES(?1,?2,?3,?4,?5)",params![id,next_generation,command.principal_id,command.session_id,current.grant_revision])?;
            tx.execute("UPDATE competition_state SET state_json=?2 WHERE competition_id=?1",params![id,serde_json::to_string(&state)?])?;
            let next:i64=tx.query_row("SELECT revision FROM competition_control_revisions WHERE competition_id=?1 AND deleted=0",[id],|r|r.get(0))?;
            if revision.checked_add(1)!=Some(next) { return Err(DatabaseError::Invalid("competition revision inconsistent".into())); }
            let result=Versioned::new(serde_json::json!({"id":id,"lifecycle":state.lifecycle,"generation":state.generation.to_string()}),vec![version(next)]);
            let result=super::super::commands::finish(&tx,command,&hash,operation.method(),&command.request.expected_revision,result)?;
            for subject in identities { subject.check_deadline().map_err(DatabaseError::CommandAuthorization)?; }
            auth.snapshot().map_err(DatabaseError::CommandAuthorization)?;
            tx.commit()?;
            Ok((result,true))
        }).map_err(DatabaseError::CommandAuthorization)?
    }

    /// The lifecycle update, retained outcome and metadata outbox commit in
    /// one writer transaction under the current collaboration grant.
    pub(crate) fn execute_competition_stop(
        &self,
        id: &str,
        generation: u64,
        operation: crate::control::CompetitionRemoteTransition,
        command: &super::super::CommandTransaction,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
        owner: Option<&str>,
    ) -> Result<(crate::collaboration::Versioned<serde_json::Value>, bool), DatabaseError> {
        use crate::collaboration::{Capability, ResourceKind, ResourceVersion, Versioned};
        let auth = command
            .authorization
            .as_ref()
            .ok_or(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ))?;
        let subjects = service
            .execution_authorizations(&[(command.principal_id.clone(), command.session_id.clone())])
            .map_err(DatabaseError::CommandAuthorization)?;
        let hash = super::super::commands::payload_hash(
            operation.method(),
            &serde_json::json!({"id":id,"generation":generation.to_string()}),
            &command.request.expected_revision,
        )?;
        let mut connection = self.open(&self.automation_path())?;
        connection.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        subjects.with_current(|identities| {
            let identity = identities.first().ok_or(DatabaseError::CommandAuthorization(crate::collaboration::Error::Forbidden))?;
            let current = auth.snapshot().map_err(DatabaseError::CommandAuthorization)?;
            if current.principal_id != command.principal_id || current.session_id != command.session_id
                || current.grant_revision != identity.grant_revision() || current.grant != *identity.grant()
                || !current.grant.capabilities.contains(&Capability::CompetitionsRead)
                || !current.grant.capabilities.contains(&Capability::CompetitionsManage)
                || current.grant.resources.contains_key(&ResourceKind::Live)
                || !current.grant.allows_resource(ResourceKind::Competition, id) {
                return Err(DatabaseError::CommandAuthorization(crate::collaboration::Error::Forbidden));
            }
            super::super::commands::prune(&tx)?;
            if let Some(result) = super::super::commands::replay(&tx, command, &hash)? {
                tx.commit()?;
                return Ok((result, false));
            }
            let row: Option<(String, String, i64)> = tx.query_row(
                "SELECT d.definition_json,s.state_json,r.revision FROM competition_definitions d JOIN competition_state s ON s.competition_id=d.id JOIN competition_control_revisions r ON r.competition_id=d.id AND r.deleted=0 WHERE d.id=?1",
                [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional()?;
            let (definition, state, revision) = row.ok_or(DatabaseError::NotFound)?;
            let version = |revision: i64| ResourceVersion { kind: ResourceKind::Competition, scope: "host".into(), id: id.into(), revision: revision.to_string() };
            if revision <= 0 || command.request.expected_revision != revision.to_string() {
                return Err(DatabaseError::RevisionConflict(version(revision)));
            }
            let definition: Definition = serde_json::from_str(&definition)?;
            let mut state: LifecycleState = serde_json::from_str(&state)?;
            if state.generation != generation { return Err(DatabaseError::RevisionConflict(version(revision))); }
            if state.lifecycle != Lifecycle::Draft {
                if let Some(owner) = owner {
                    let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id,generation as i64,owner], |r| r.get(0))?;
                    if !valid { return Err(DatabaseError::Invalid("competition owner changed".into())); }
                }
            }
            // Stopping remains available even if an earlier operator was
            // revoked. Retain this caller when possible; later event effects
            // require all retained subjects to be current at admission.
            if state.lifecycle != Lifecycle::Draft {
                let origin: Option<String> = tx.query_row(
                    "SELECT origin FROM competition_execution_generations WHERE competition_id=?1 AND generation=?2",
                    params![id, generation as i64], |row| row.get(0),
                ).optional()?;
                let count: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM competition_execution_subjects WHERE competition_id=?1 AND generation=?2",
                    params![id, generation as i64], |row| row.get(0),
                )?;
                let existing: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM competition_execution_subjects WHERE competition_id=?1 AND generation=?2 AND principal_id=?3 AND session_id=?4)",
                    params![id,generation as i64,command.principal_id,command.session_id], |row| row.get(0),
                )?;
                let valid_origin = matches!(origin.as_deref(), Some("owner")) && count == 0
                    || matches!(origin.as_deref(), Some("delegated")) && (1..=16).contains(&count);
                if valid_origin && (existing || count < 16) {
                    tx.execute("UPDATE competition_execution_generations SET origin='delegated' WHERE competition_id=?1 AND generation=?2", params![id,generation as i64])?;
                    tx.execute("INSERT OR IGNORE INTO competition_execution_subjects(competition_id,generation,principal_id,session_id,grant_revision) VALUES(?1,?2,?3,?4,?5)",
                        params![id,generation as i64,command.principal_id,command.session_id,current.grant_revision])?;
                }
            }
            state.transition(operation.transition(), crate::now_millis(), definition.duration_ms)
                .map_err(|_| DatabaseError::Invalid("invalid competition transition".into()))?;
            tx.execute("UPDATE competition_state SET state_json=?2 WHERE competition_id=?1", params![id,serde_json::to_string(&state)?])?;
            let next: i64 = tx.query_row("SELECT revision FROM competition_control_revisions WHERE competition_id=?1 AND deleted=0", [id], |r| r.get(0))?;
            if revision.checked_add(1) != Some(next) { return Err(DatabaseError::Invalid("competition revision inconsistent".into())); }
            let result = Versioned::new(serde_json::json!({"id":id,"lifecycle":state.lifecycle,"generation":state.generation.to_string()}), vec![version(next)]);
            let result = super::super::commands::finish(&tx, command, &hash, operation.method(), &command.request.expected_revision, result)?;
            identity.check_deadline().map_err(DatabaseError::CommandAuthorization)?;
            auth.snapshot().map_err(DatabaseError::CommandAuthorization)?;
            tx.commit()?;
            Ok((result, true))
        }).map_err(DatabaseError::CommandAuthorization)?
    }
}

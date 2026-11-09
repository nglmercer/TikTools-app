use super::*;
use crate::control::AutomationMutation;

impl DatabaseManager {
    /// Select the stricter reusable path for a disabled Owner edit. The
    /// transaction itself rechecks provenance and authorization: this lookup
    /// is only a routing hint and cannot grant permission.
    pub(crate) fn is_owner_behavior(
        &self,
        kind: crate::control::AutomationKind,
        id: &str,
    ) -> Result<bool, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        Ok(provenance::get(&db, kind.table(), id)?
            .is_some_and(|p| matches!(p.authority, provenance::BehaviorAuthority::Owner)))
    }

    pub(crate) fn execute_behavior_command(
        &self,
        operation: &AutomationMutation,
        command: &super::super::CommandTransaction,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        command
            .request
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid command metadata".into()))?;
        let hash = super::super::commands::payload_hash(
            operation.method(),
            &operation.params(),
            &command.request.expected_revision,
        )?;
        self.behavior_transaction(|tx| {
            if let Some(auth) = &command.authorization {
                let identity = operation
                    .authorize_remote(auth)
                    .map_err(DatabaseError::CommandAuthorization)?;
                if identity.principal_id != command.principal_id
                    || identity.session_id != command.session_id
                {
                    return Err(DatabaseError::CommandAuthorization(
                        crate::collaboration::Error::Forbidden,
                    ));
                }
            }
            super::super::commands::prune(tx)?;
            if let Some(result) = super::super::commands::replay(tx, command, &hash)? {
                return Ok((result, false));
            }
            let expected = Some(command.request.expected_revision.as_str());
            let (kind, id) = operation.target();
            let permit = if operation.is_remote_draft() {
                command
                    .authorization
                    .as_ref()
                    .map(|auth| provenance::authorize_authoring(tx, kind.table(), id, auth, false))
                    .transpose()?
            } else {
                None
            };
            // Validate only fresh delegated writes, after identity and target
            // ownership checks. Retained receipts and legacy owner definitions
            // do not acquire new structure requirements or a different hash.
            let normalized = match operation {
                AutomationMutation::Create { record, .. }
                | AutomationMutation::Update { record, .. }
                    if permit.is_some() =>
                {
                    Some(drafts::normalize(kind.table(), record)?)
                }
                _ => None,
            };
            let result = match operation {
                AutomationMutation::Adopt {
                    reviewed_versions, ..
                } => {
                    if command.authorization.is_some() {
                        return Err(DatabaseError::CommandAuthorization(
                            crate::collaboration::Error::Forbidden,
                        ));
                    }
                    if let Some(reviewed) = reviewed_versions {
                        review::check_versions(
                            tx,
                            kind.table(),
                            id,
                            &command.request.expected_revision,
                            reviewed,
                        )?;
                    }
                    provenance::adopt_owner(
                        tx,
                        kind.table(),
                        id,
                        &command.request.expected_revision,
                    )?
                    .map(|_| json!({"id":id,"adopted":true}))
                }
                AutomationMutation::Create { record, .. } => save_record(
                    tx,
                    kind.table(),
                    normalized.as_ref().unwrap_or(record),
                    expected,
                    BehaviorSave::Create,
                )?,
                AutomationMutation::Update { record, .. } => save_record(
                    tx,
                    kind.table(),
                    normalized.as_ref().unwrap_or(record),
                    expected,
                    BehaviorSave::Update,
                )?,
                AutomationMutation::Delete { .. } => delete_record(tx, kind.table(), id, expected)?
                    .map(|deleted| json!({"id":id,"deleted":deleted})),
                AutomationMutation::SetEnabled { enabled, .. } => {
                    let result = set_enabled(tx, kind.table(), id, *enabled, expected)?;
                    if command.authorization.is_some() {
                        result.map(|_| json!({"id":id,"enabled":enabled}))
                    } else {
                        result
                    }
                }
            };
            let result = if let Some(permit) = permit {
                if permit.identity.principal_id != command.principal_id
                    || permit.identity.session_id != command.session_id
                {
                    return Err(DatabaseError::CommandAuthorization(
                        crate::collaboration::Error::Forbidden,
                    ));
                }
                provenance::mark_delegated_new(tx, kind.table(), id, &permit)?;
                result.map(|_| json!({"id":id,"kind":kind,"enabled":false,"pendingReview":true}))
            } else {
                result
            };
            let result = super::super::commands::finish(
                tx,
                command,
                &hash,
                operation.method(),
                &command.request.expected_revision,
                result,
            )?;
            Ok((result, true))
        })
    }
}

impl DatabaseManager {
    /// Commit an activation or enabled authoring write only while the caller
    /// and every saved author remain current and the final closure is permitted.
    pub(crate) fn execute_reusable_behavior_activation(
        &self,
        operation: &AutomationMutation,
        command: &super::super::CommandTransaction,
        service: &std::sync::Arc<crate::collaboration::CollaborationService>,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        use crate::services::automation_manifest::{
            self as manifest, ManifestSubject, ManifestSubjects,
        };
        if !operation.is_remote_activation()
            && !operation.is_reusable_write()
            && !operation.is_remote_draft_update()
        {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ));
        }
        let auth = command
            .authorization
            .as_ref()
            .ok_or(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ))?;
        command
            .request
            .validate()
            .map_err(|_| DatabaseError::Invalid("invalid command metadata".into()))?;
        let hash = super::super::commands::payload_hash(
            operation.method(),
            &operation.params(),
            &command.request.expected_revision,
        )?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let identity = operation
            .authorize_remote(auth)
            .map_err(DatabaseError::CommandAuthorization)?;
        if identity.principal_id != command.principal_id
            || identity.session_id != command.session_id
        {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ));
        }
        super::super::commands::prune(&tx)?;
        if let Some(result) = super::super::commands::replay(&tx, command, &hash)? {
            tx.commit()?;
            return Ok((result, false));
        }
        let (kind, id) = operation.target();
        // Existing authoring ownership and dependency rules also prevent an
        // activation from changing an Owner or another author's graph.
        let author = provenance::authorize_authoring(
            &tx,
            kind.table(),
            id,
            auth,
            matches!(operation, AutomationMutation::Update { .. })
                || operation.is_remote_activation(),
        )?;
        // Stage the new record inside the uncommitted transaction, so all
        // subjects are derived from its actual final closure, including new
        // dependencies and a changed author session.
        let result = match operation {
            AutomationMutation::SetEnabled { .. } => set_enabled(
                &tx,
                kind.table(),
                id,
                true,
                Some(&command.request.expected_revision),
            )?,
            AutomationMutation::Create { record, .. }
            | AutomationMutation::Update { record, .. } => {
                let mut disabled = record.clone();
                disabled["enabled"] = Value::Bool(false);
                let normalized = drafts::normalize(kind.table(), &disabled)?;
                let mode = if matches!(operation, AutomationMutation::Create { .. }) {
                    BehaviorSave::Create
                } else {
                    BehaviorSave::Update
                };
                let result = save_record(
                    &tx,
                    kind.table(),
                    &normalized,
                    Some(&command.request.expected_revision),
                    mode,
                )?;
                provenance::mark_delegated_new(&tx, kind.table(), id, &author)?;
                if operation.is_reusable_write() {
                    toggle(&tx, kind.table(), id, true)?.ok_or(DatabaseError::NotFound)?;
                }
                result
            }
            _ => {
                return Err(DatabaseError::CommandAuthorization(
                    crate::collaboration::Error::Forbidden,
                ))
            }
        };
        let after = review::snapshot(&tx, kind.table(), id)?;
        if author.affected_events.len() > 512 {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        let mut review_bytes = serde_json::to_vec(&after)?.len();
        let mut reviews = vec![after];
        for event_id in &author.affected_events {
            let candidate = review::snapshot(&tx, "behavior_events", event_id)?;
            review_bytes = review_bytes
                .checked_add(serde_json::to_vec(&candidate)?.len())
                .ok_or(DatabaseError::SnapshotTooLarge)?;
            if review_bytes > crate::collaboration::SNAPSHOT_BYTES {
                return Err(DatabaseError::SnapshotTooLarge);
            }
            reviews.push(candidate);
        }
        let mut required = Vec::new();
        for review in &reviews {
            for subject in manifest::required_subjects(review)
                .map_err(|_| DatabaseError::Invalid("invalid execution review".into()))?
            {
                if !required.contains(&subject) {
                    required.push(subject);
                }
            }
        }
        let caller_pair = (command.principal_id.clone(), command.session_id.clone());
        if !required.contains(&caller_pair) {
            required.push(caller_pair);
        }
        let batch = service
            .execution_authorizations(&required)
            .map_err(DatabaseError::CommandAuthorization)?;
        batch
            .with_current(|identities| {
                let caller = identities
                    .iter()
                    .find(|subject| {
                        subject.principal_id() == command.principal_id
                            && subject.session_id() == command.session_id
                    })
                    .ok_or(DatabaseError::CommandAuthorization(
                        crate::collaboration::Error::Forbidden,
                    ))?;
                let current = operation
                    .authorize_remote(auth)
                    .map_err(DatabaseError::CommandAuthorization)?;
                if current.principal_id != author.identity.principal_id
                    || current.session_id != command.session_id
                    || current.grant_revision != author.identity.grant_revision
                    || current.grant != author.identity.grant
                    || current.grant_revision != caller.grant_revision()
                    || current.grant != *caller.grant()
                {
                    return Err(DatabaseError::CommandAuthorization(
                        crate::collaboration::Error::Forbidden,
                    ));
                }
                for review in &reviews {
                    if review.value.get("kind").and_then(Value::as_str) == Some("event")
                        && review
                            .value
                            .get("record")
                            .is_none_or(|record| !crate::services::public_filter_paths(record))
                    {
                        return Err(DatabaseError::CommandAuthorization(
                            crate::collaboration::Error::Forbidden,
                        ));
                    }
                    let authors = manifest::required_subjects(review)
                        .map_err(|_| DatabaseError::Invalid("invalid execution review".into()))?;
                    let selected: Vec<_> = identities
                        .iter()
                        .filter(|identity| {
                            authors.iter().any(|(p, s)| {
                                p == identity.principal_id() && s == identity.session_id()
                            })
                        })
                        .cloned()
                        .collect();
                    let subjects: Vec<_> = selected
                        .iter()
                        .map(|identity| ManifestSubject {
                            principal_id: identity.principal_id(),
                            session_id: identity.session_id(),
                            grant_revision: identity.grant_revision(),
                            configuration_revision: identity.configuration_revision(),
                            grant: identity.grant(),
                        })
                        .collect();
                    let built = manifest::build(
                        review,
                        ManifestSubjects {
                            host_id: service.host_id(),
                            subjects: &subjects,
                        },
                    )
                    .map_err(|_| DatabaseError::Invalid("invalid execution review".into()))?;
                    crate::services::automation_approval_policy::reusable_preflight(
                        &built, &selected,
                    )
                    .map_err(|_| {
                        DatabaseError::CommandAuthorization(crate::collaboration::Error::Forbidden)
                    })?;
                }
                let result = result.map(|_| {
                    if operation.is_remote_draft_update() {
                        json!({"kind":kind,"id":id,"enabled":false,"pendingReview":true})
                    } else if operation.is_reusable_write() {
                        json!({"kind":kind,"id":id,"enabled":true,"pendingReview":false})
                    } else {
                        json!({"kind":kind,"id":id,"enabled":true})
                    }
                });
                let result = super::super::commands::finish(
                    &tx,
                    command,
                    &hash,
                    operation.method(),
                    &command.request.expected_revision,
                    result,
                )?;
                for identity in identities {
                    identity
                        .check_deadline()
                        .map_err(DatabaseError::CommandAuthorization)?;
                }
                operation
                    .authorize_remote(auth)
                    .map_err(DatabaseError::CommandAuthorization)?;
                tx.commit()?;
                Ok((result, true))
            })
            .map_err(DatabaseError::CommandAuthorization)?
    }
}

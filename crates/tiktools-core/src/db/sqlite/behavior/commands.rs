use super::*;
use crate::control::AutomationMutation;

impl DatabaseManager {
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
                    .map(|auth| provenance::authorize_authoring(tx, kind.table(), id, auth))
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

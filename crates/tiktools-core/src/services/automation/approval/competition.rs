//! Competition operator and immutable winner bindings during authorization and admission.
use super::*;

impl ApprovalCandidate {
    /// Call inside the competition fence's current-subject transaction. The
    /// permit still needs that fence at each effect sink; it never conveys
    /// competition authority by itself. Persisted approvals cannot be widened.
    #[allow(dead_code)]
    pub(crate) fn authorize_competition(
        self,
        identities: &[ExecutionIdentity],
        fence: &crate::db::CompetitionExecutionFence,
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        let operators = fence.subjects();
        if operators.is_empty()
            || self.approval.is_some()
            || identities
                .iter()
                .any(|identity| identity.host_id() != fence.host_id())
        {
            return Err("competition operator binding is unavailable");
        }
        self.authorize_with_inherited(identities, &operators, Some(fence.clone()), None, false)
    }

    /// Inherit every prior author and operator for a chained saved rule.
    pub(crate) fn authorize_competition_chain(
        self,
        admission: &crate::db::CompetitionExecutionAdmission<'_>,
        inherited: &[(String, String)],
        native_competition_event: bool,
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        if self.approval.is_some()
            || native_competition_event
                && admission.identities().iter().any(|identity| {
                    !identity.grant().allows_resource(
                        crate::collaboration::ResourceKind::Competition,
                        admission.fence().competition_id(),
                    )
                })
            || admission
                .fence()
                .subjects()
                .iter()
                .any(|operator| !inherited.contains(operator))
            || admission
                .identities()
                .iter()
                .any(|identity| identity.host_id() != admission.fence().host_id())
        {
            return Err("competition chain binding is unavailable");
        }
        self.authorize_with_inherited(
            admission.identities(),
            inherited,
            Some(admission.fence().clone()),
            None,
            native_competition_event,
        )
    }

    /// Classify the effective winner action, including the exact saved input
    /// overrides, while the winner outbox and competition fence are held.
    #[allow(dead_code)]
    pub(crate) fn authorize_competition_winner(
        self,
        admission: &crate::db::CompetitionExecutionAdmission<'_>,
    ) -> Result<RuntimeApprovalPermit, &'static str> {
        let claim = admission
            .winner()
            .ok_or("competition winner binding is unavailable")?;
        if self.approval.is_some()
            || self.approval_key != ("action".into(), claim.action.action_id.clone())
            || admission
                .identities()
                .iter()
                .any(|identity| identity.host_id() != admission.fence().host_id())
        {
            return Err("competition winner binding is unavailable");
        }
        let mut effective = self
            .review
            .value
            .get("record")
            .cloned()
            .ok_or("saved winner action is unavailable")?;
        let inputs = match &claim.action.inputs {
            Value::Null => serde_json::Map::new(),
            Value::Object(inputs) => inputs.clone(),
            _ => return Err("competition winner inputs are invalid"),
        };
        let config = effective
            .as_object_mut()
            .ok_or("saved winner action is invalid")?
            .entry("config")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or("saved winner action is invalid")?;
        config.extend(inputs);
        let operators = admission.fence().subjects();
        if operators.is_empty() {
            return Err("competition operator binding is unavailable");
        }
        self.authorize_with_inherited(
            admission.identities(),
            &operators,
            Some(admission.fence().clone()),
            Some((claim.clone(), effective)),
            false,
        )
    }
}

impl AutomationService {
    /// The admission proof is available only while the matching competition
    /// transaction and every current subject are locked by the database.
    #[allow(dead_code)]
    pub(crate) fn with_current_competition_approvals<R>(
        &self,
        admission: &crate::db::CompetitionExecutionAdmission<'_>,
        permits: &[RuntimeApprovalPermit],
        owner_rules: &[RuntimeOwnerRuleFence],
        action: &Value,
        apply: impl FnOnce() -> Result<R, String>,
    ) -> Result<R, String> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !execution_chain_current(&index, permits, owner_rules, Some(admission.fence()))
            || permits.iter().any(|permit| {
                permit.check_subjects(admission.identities()).is_err()
                    || permit
                        .competition_winner
                        .as_ref()
                        .is_some_and(|(claim, _)| admission.winner() != Some(claim))
            })
            || !permits
                .iter()
                .any(|permit| permit.authorizes_action(action))
        {
            return Err("saved competition approval definition binding has changed".into());
        }
        apply()
    }

    pub(crate) fn with_current_competition_chain<R>(
        &self,
        admission: &crate::db::CompetitionExecutionAdmission<'_>,
        permits: &[RuntimeApprovalPermit],
        owner_rules: &[RuntimeOwnerRuleFence],
        apply: impl FnOnce() -> Result<R, String>,
    ) -> Result<R, String> {
        let index = recover_rwlock_read(&self.index, "automation index");
        if !execution_chain_current(&index, permits, owner_rules, Some(admission.fence()))
            || permits.iter().any(|permit| {
                permit.check_subjects(admission.identities()).is_err()
                    || permit
                        .competition_winner
                        .as_ref()
                        .is_some_and(|(claim, _)| admission.winner() != Some(claim))
            })
        {
            return Err("saved competition execution binding has changed".into());
        }
        apply()
    }
}

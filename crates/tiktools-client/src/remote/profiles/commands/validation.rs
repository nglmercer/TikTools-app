//! Closed validation of durable intents, outcomes and exact point adjustment results.
use super::*;

impl Intent {
    pub fn validate(&self) -> Result<(), ClientError> {
        if self.operation.is_competition() {
            if self
                .expected_generation
                .as_deref()
                .and_then(|s| tiktools_collaboration::stream_sequence(s).ok())
                .is_none_or(|n| n > i64::MAX as u64)
                || self
                    .expected_revision
                    .as_deref()
                    .and_then(|s| tiktools_collaboration::stream_sequence(s).ok())
                    .is_none_or(|n| n == 0 || n > i64::MAX as u64)
            {
                return Err(invalid());
            }
        } else if self.expected_generation.is_some() {
            return Err(invalid());
        }
        if self.operation.is_proposal() {
            if self
                .proposal_revision
                .as_deref()
                .and_then(|n| tiktools_collaboration::stream_sequence(n).ok())
                .is_none_or(|n| n == 0 || n > i64::MAX as u64)
                || self
                    .expected_revision
                    .as_deref()
                    .and_then(|n| tiktools_collaboration::stream_sequence(n).ok())
                    .is_none_or(|n| {
                        n > i64::MAX as u64
                            || n == 0
                                && matches!(
                                    self.operation,
                                    Operation::UpdateEventDraft
                                        | Operation::UpdateActionDraft
                                        | Operation::UpdateEventActive
                                        | Operation::UpdateActionActive
                                )
                    })
            {
                return Err(invalid());
            }
        } else if self.proposal_revision.is_some() {
            return Err(invalid());
        }
        if !valid_id(&self.command_id)
            || !valid_id(&self.host_instance_id)
            || self.resource_id.is_empty()
            || self.resource_id.len() > 128
            || self.resource_id.trim() != self.resource_id
            || self.resource_id.contains('*')
            || self.resource_id.chars().any(char::is_control)
            || self.created_at < 0
            || self.created_at > 9_007_199_254_740_991
        {
            return Err(invalid());
        }
        match self.operation {
            Operation::AdjustPoints => {
                if self.expected_revision.is_some()
                    || self.resource_id.starts_with('@')
                    || !self
                        .delta
                        .is_some_and(|n| n.is_finite() && n != 0.0 && n.abs() <= 1_000_000_000.0)
                    || !self.currency_name.as_deref().is_some_and(|s| {
                        !s.is_empty()
                            && s.trim() == s
                            && s.chars().count() <= 64
                            && !s.chars().any(char::is_control)
                    })
                {
                    return Err(invalid());
                }
            }
            _ => {
                if self.delta.is_some()
                    || self.currency_name.is_some()
                    || self
                        .expected_revision
                        .as_deref()
                        .is_none_or(|s| tiktools_collaboration::stream_sequence(s).is_err())
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
}

impl PointsResult {
    pub(crate) fn valid_for(&self, requested: f64) -> bool {
        let rounded = |n: f64| (n * 100.0).round() / 100.0;
        self.delta.is_finite()
            && self.total_points.is_finite()
            && self.total_points >= 0.0
            && self.level > 0
            && if requested < 0.0 {
                self.delta == rounded(requested)
            } else {
                // The host bounds its positive subscriber bonus to 10,000%.
                self.delta >= rounded(requested)
                    && self.delta <= rounded(requested + requested * 100.0)
            }
    }
}

impl Record {
    pub(super) fn validate(&self) -> Result<(), ClientError> {
        self.intent.validate()?;
        if !(0..=9_007_199_254_740_991).contains(&self.recorded_at) {
            return Err(invalid());
        }
        if (self.intent.operation != Operation::AdjustPoints || self.outcome != Outcome::Succeeded)
            && self.points_result.is_some()
        {
            return Err(invalid());
        }
        match self.outcome {
            Outcome::Succeeded => {
                if self.error_code.is_some() {
                    return Err(invalid());
                }
                let after = tiktools_collaboration::stream_sequence(
                    self.revision.as_deref().ok_or_else(invalid)?,
                )
                .map_err(|_| invalid())?;
                if self.intent.operation == Operation::AdjustPoints {
                    let result = self.points_result.as_ref().ok_or_else(invalid)?;
                    let requested = self.intent.delta.ok_or_else(invalid)?;
                    if after == 0 || !result.valid_for(requested) {
                        return Err(invalid());
                    }
                    return Ok(());
                }
                let before = tiktools_collaboration::stream_sequence(
                    self.intent
                        .expected_revision
                        .as_deref()
                        .ok_or_else(invalid)?,
                )
                .map_err(|_| invalid())?;
                if before.checked_add(1) != Some(after)
                    && !(self.intent.operation.is_delete() && after == before)
                {
                    return Err(invalid());
                }
            }
            Outcome::Rejected => {
                if self.error_code.as_deref() == Some("proposal_conflict")
                    && !self.intent.operation.is_proposal()
                {
                    return Err(invalid());
                }
                if (self.error_code.as_deref() == Some("automation_not_found")
                    && !self.intent.operation.is_automation_command()
                    && !self.intent.operation.is_proposal())
                    || self.revision.is_some()
                    || !matches!(
                        self.error_code.as_deref(),
                        Some(
                            "invalid_params"
                                | "currency_changed"
                                | "revision_conflict"
                                | "not_found"
                                | "automation_not_found"
                                | "overloaded"
                                | "stale_view"
                                | "not_ready"
                                | "forbidden"
                                | "proposal_conflict"
                        )
                    )
                {
                    return Err(invalid());
                }
            }
            Outcome::ReviewedUnknown => {
                if self.revision.is_some() || self.error_code.as_deref() != Some("command_unknown")
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
}

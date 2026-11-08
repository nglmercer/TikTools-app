//! Loyalty points configuration and balance operations.

use super::OperationError;
use crate::events::DomainEvent;
use crate::ipc::messages::PartialPointsConfig;
use crate::ipc::messages::PointsConfig;
use crate::services::PointAward;
use crate::*;
#[cfg(all(test, feature = "persistence"))]
mod tests;

#[derive(Debug, Clone)]
pub struct PointsAdjustment {
    pub unique_id: String,
    pub delta: f64,
    pub expected_currency: Option<String>,
}

pub(crate) fn points_read_allowed(grant: &collaboration::Grant) -> bool {
    grant
        .capabilities
        .contains(&collaboration::Capability::PointsRead)
        && !grant
            .resources
            .contains_key(&collaboration::ResourceKind::Live)
}

impl PointsAdjustment {
    #[cfg(feature = "persistence")]
    pub(crate) fn validate(&self) -> Result<(), OperationError> {
        if !services::valid_viewer_id(&self.unique_id)
            || !self.delta.is_finite()
            || self.delta.abs() > 1_000_000_000.0
            || self
                .expected_currency
                .as_ref()
                .is_some_and(|name| !valid_currency(name))
        {
            return Err(OperationError::invalid("invalid viewer ID or points delta"));
        }
        Ok(())
    }

    #[cfg(feature = "persistence")]
    pub(crate) fn authorize(
        &self,
        auth: &collaboration::Authorization,
        effective: Option<f64>,
    ) -> Result<collaboration::Authenticated, collaboration::Error> {
        use collaboration::{Capability, Error, ResourceKind};
        let identity = auth.snapshot()?;
        let grant = &identity.grant;
        if !points_read_allowed(grant)
            || !grant.capabilities.contains(&Capability::PointsAdjust)
            || !grant.allows_resource(ResourceKind::Viewer, &self.unique_id)
            || grant.max_points_delta.is_some_and(|max| {
                self.delta.abs() > max || effective.is_some_and(|n| n.abs() > max)
            })
        {
            return Err(Error::Forbidden);
        }
        Ok(identity)
    }
}

pub fn valid_currency(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.chars().count() <= 64
        && !value.chars().any(char::is_control)
}

impl AppCore {
    pub fn points_display_scoped(
        &self,
        grant: &collaboration::Grant,
        limit: Option<i64>,
        id: Option<&str>,
        bytes: usize,
    ) -> Result<collaboration::Versioned<Value>, OperationError> {
        if !points_read_allowed(grant)
            || id.is_some_and(|id| {
                !services::valid_viewer_id(id)
                    || !grant.allows_resource(collaboration::ResourceKind::Viewer, id)
            })
        {
            return Err(OperationError::forbidden("points view is not available"));
        }
        #[cfg(feature = "persistence")]
        {
            let currency = self.points_config().currency_name;
            if !valid_currency(&currency) {
                return Err(OperationError::internal("points unit label is unavailable"));
            }
            self.db
                .points_display_snapshot(
                    grant,
                    limit.unwrap_or(20).clamp(0, 1000) as usize,
                    id,
                    &currency,
                    bytes,
                )
                .map_err(points_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (limit, bytes);
            Err(OperationError::capability_unavailable(
                "points projections require persistence",
            ))
        }
    }
    pub fn points_viewer_scoped(
        &self,
        grant: &collaboration::Grant,
        id: &str,
        bytes: usize,
    ) -> Result<collaboration::Versioned<Option<Value>>, OperationError> {
        if !points_read_allowed(grant)
            || !services::valid_viewer_id(id)
            || !grant.allows_resource(collaboration::ResourceKind::Viewer, id)
        {
            return Err(OperationError::forbidden("viewer is not available"));
        }
        #[cfg(feature = "persistence")]
        {
            self.db
                .points_viewer_snapshot(id, bytes)
                .map_err(points_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = bytes;
            Err(OperationError::capability_unavailable(
                "points projections require persistence",
            ))
        }
    }

    pub fn points_leaderboard_scoped(
        &self,
        grant: &collaboration::Grant,
        limit: Option<i64>,
        bytes: usize,
    ) -> Result<collaboration::Versioned<Vec<Value>>, OperationError> {
        if !points_read_allowed(grant) {
            return Err(OperationError::forbidden(
                "points projection is not available",
            ));
        }
        #[cfg(feature = "persistence")]
        {
            self.db
                .points_leaderboard_snapshot(
                    grant,
                    limit.unwrap_or(100).clamp(0, 1_000) as usize,
                    bytes,
                )
                .map_err(points_error)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (limit, bytes);
            Err(OperationError::capability_unavailable(
                "points projections require persistence",
            ))
        }
    }
    pub fn points_command(
        &self,
        caller: super::CommandCaller,
        request: collaboration::AdditiveCommandRequest,
        operation: PointsAdjustment,
    ) -> Result<collaboration::Versioned<Value>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            use super::commands::{command_error, service_error};
            operation.validate()?;
            request
                .validate()
                .map_err(|_| OperationError::invalid("invalid command metadata"))?;
            let _gate = self
                .coordination_gate
                .lock()
                .map_err(|_| OperationError::internal("command coordination is unavailable"))?;
            if self.is_shutdown() {
                return Err(OperationError::capability_unavailable(
                    "host is shutting down",
                ));
            }
            let service = self.collaboration().map_err(service_error)?;
            let remote = matches!(&caller, super::CommandCaller::Remote(_));
            let (principal_id, session_id, authorization) = match caller {
                super::CommandCaller::LocalOwner => {
                    ("local-owner".into(), "local-owner".into(), None)
                }
                super::CommandCaller::Remote(auth) => {
                    let identity = operation.authorize(&auth, None).map_err(service_error)?;
                    (identity.principal_id, identity.session_id, Some(auth))
                }
            };
            let command = crate::db::CommandTransaction {
                principal_id,
                session_id,
                authorization,
                request,
                current_instance_id: service.instance_id().into(),
                max_ids_per_principal: service
                    .configuration()
                    .map_err(service_error)?
                    .limits
                    .command_ids_per_principal,
            };
            let existing = self
                .db
                .check_command_admission(
                    crate::db::CommandDatabase::Points,
                    &command.principal_id,
                    &command.request.command_id,
                    command.max_ids_per_principal,
                )
                .map_err(|e| command_error(e, &command.request.command_id))?;
            if existing {
                // Retained outcomes remain queryable/replayable even when the
                // award writer is unavailable. Never replace cache state with
                // a historical balance from this result.
                return self
                    .db
                    .replay_points_command(&operation, &command)
                    .map_err(|e| command_error(e, &command.request.command_id));
            }
            let (result, fresh) = self
                .points
                .execute_command(&operation, &command)
                .map_err(|e| command_error(e, &command.request.command_id))?;
            if fresh {
                let award: PointAward = serde_json::from_value(result.value.clone())
                    .map_err(|_| OperationError::internal("invalid points command result"))?;
                let event = DomainEvent::PointsChanged {
                    unique_id: award.unique_id,
                    delta: award.delta,
                    total_points: award.total_points,
                    level: award.level,
                };
                if remote {
                    self.events.publish_presentation_domain(event);
                } else {
                    self.events.publish_domain(event);
                }
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (caller, request, operation);
            Err(OperationError::capability_unavailable(
                "points commands require persistence",
            ))
        }
    }
    // ------------------------------------------------------------------
    // Points.
    // ------------------------------------------------------------------

    pub fn points_config(&self) -> PointsConfig {
        self.points.config()
    }

    pub fn points_update_config(&self, update: PartialPointsConfig) -> PointsConfig {
        self.points.update_config(update)
    }

    pub fn points_leaderboard(&self, limit: Option<i64>) -> Vec<Value> {
        self.points.leaderboard(limit)
    }

    pub fn points_viewer(&self, unique_id: &str) -> Result<Value, OperationError> {
        let needle = unique_id.trim().trim_start_matches('@');
        if needle.is_empty() {
            return Err(OperationError::invalid("uniqueId must not be empty"));
        }
        // Direct indexed lookup (hot cache, then one SQLite primary-key
        // read): O(1)-average independent of leaderboard position, with no
        // 1,000-row window. Cold viewers reload from durable storage.
        self.points.viewer(needle).ok_or_else(|| {
            OperationError::not_found(format!("viewer `{needle}` has no points record"))
        })
    }

    /// Manual adjustment, matching the WebView adjust-points path: creates
    /// the viewer record when it does not exist yet.
    pub fn points_adjust(&self, unique_id: &str, delta: f64) -> Result<PointAward, OperationError> {
        if !delta.is_finite() || delta.abs() > 1_000_000_000.0 {
            return Err(OperationError::invalid("delta must be a finite number"));
        }
        let award = self
            .points
            .award_points(
                unique_id,
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(delta),
                    ..AwardOptions::default()
                },
            )
            .ok_or_else(|| OperationError::invalid("uniqueId must not be empty"))?;
        self.events.publish_domain(DomainEvent::PointsChanged {
            unique_id: award.unique_id.clone(),
            delta: award.delta,
            total_points: award.total_points,
            level: award.level,
        });
        Ok(award)
    }

    pub fn points_reset(&self, unique_id: Option<&str>) {
        let cleaned = unique_id.map(|value| value.trim().trim_start_matches('@').to_owned());
        if cleaned
            .as_deref()
            .is_some_and(|id| !crate::services::valid_viewer_id(id))
        {
            return;
        }
        if !self.points.reset_admitted(cleaned.as_deref()) {
            return;
        }
        self.events
            .publish_domain(DomainEvent::PointsReset { unique_id: cleaned });
    }
}

#[cfg(feature = "persistence")]
fn points_error(error: crate::db::DatabaseError) -> OperationError {
    match error {
        crate::db::DatabaseError::SnapshotTooLarge => OperationError {
            code: "too_large",
            message: "points projection exceeds its bounded budget".into(),
            data: None,
        },
        _ => OperationError::internal("points projection is unavailable"),
    }
}

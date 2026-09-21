//! Loyalty points configuration and balance operations.

use super::OperationError;
use crate::events::DomainEvent;
use crate::ipc::messages::PartialPointsConfig;
use crate::ipc::messages::PointsConfig;
use crate::services::PointAward;
use crate::*;

impl AppCore {
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
        self.points
            .leaderboard(Some(1_000))
            .into_iter()
            .find(|viewer| viewer.get("uniqueId").and_then(Value::as_str) == Some(needle))
            .ok_or_else(|| {
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
        let cleaned = unique_id
            .map(|value| value.trim().trim_start_matches('@').to_owned())
            .filter(|value| !value.is_empty());
        self.points.reset(cleaned.as_deref());
    }
}

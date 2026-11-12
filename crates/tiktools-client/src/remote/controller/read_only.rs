//! Explicit remote read forwarding with current-selection reply checks.
use super::*;

impl Controller {
    pub async fn forward(
        &self,
        epoch: &str,
        method: &str,
        params: Value,
        meta: Option<RequestMetadata>,
    ) -> Result<CallResult<Value>, ClientError> {
        // Page mutations must use the native durable-intent coordinator. Add
        // completed domain reads explicitly; new methods never bypass it.
        if !matches!(
            method,
            "rpc.discover"
                | "rpc.schema"
                | "collaboration.me"
                | "collaboration.commands.get"
                | "live.status"
                | "live.monitor.status"
                | "ui.feed.backfill"
                | "workflows.list"
                | "workflows.get"
                | "points.info"
                | "points.viewer.get"
                | "points.leaderboard"
        ) || meta.as_ref().is_some_and(|m| {
            m.command_id.is_some() || m.host_instance_id.is_some() || m.expected_revision.is_some()
        }) {
            return Err(fail(
                "forbidden",
                "controller manages its own remote session",
            ));
        }
        let remote = {
            let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            if v.status.epoch != epoch || v.status.phase != "ready" {
                return Err(fail("not_ready", "selected controller is not ready"));
            }
            v.remote.clone().ok_or_else(network)?
        };
        if remote.requires_resync() {
            return Err(fail("not_ready", "controller needs recovery"));
        }
        let result = remote.call_value_with_meta(method, params, meta).await;
        if self.status().epoch != epoch {
            return Err(fail(
                "stale_view",
                "reply belongs to a previous controller selection",
            ));
        }
        result
    }
}

//! Map actual game outcomes to the TikTools process-plugin result contract.
use l4d2api_client::TypedResponse;
use l4d2api_protocol::Status;
use tiktools_plugin_sdk::{ActionResult, PluginError, PluginResult};

pub fn from_game(label: &str, response: TypedResponse) -> PluginResult<ActionResult> {
    let details = format!("{} ({})", response.code, response.diag);
    match response.status {
        Status::Applied => {
            if response.requested > 0 && response.actual < response.requested {
                return Err(PluginError::other(format!("{label}: inconsistent game reply {details}")));
            }
            Ok(ActionResult::summary(if response.requested == 0 {
                format!("L4D2 connected: {details}")
            } else {
                format!("{label}: {}/{} applied", response.actual, response.requested)
            })
            .log(format!("request={} gen={} code={}", response.request_id, response.generation, response.code)))
        }
        Status::Partial if response.actual > 0 && response.actual < response.requested => {
            Ok(ActionResult::summary(format!(
                "{label}: {}/{} applied (partial; {})",
                response.actual, response.requested, details
            )).log(format!("request={} gen={}", response.request_id, response.generation)))
        }
        Status::Rejected | Status::NotReady => {
            Err(PluginError::other(format!("{label}: game rejected request: {details}")))
        }
        Status::Unknown => {
            Err(PluginError::other(format!("{label}: outcome unconfirmed ({details}); do not retry automatically")))
        }
        Status::Partial => Err(PluginError::other(format!("{label}: invalid partial outcome {details}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(status: Status, requested: u32, actual: u32) -> TypedResponse {
        TypedResponse { status, code: "OK".into(), requested, actual, diag: "test".into(), generation: "g-1".into(), request_id: "r-1".into() }
    }

    #[test]
    fn only_confirmed_applied_and_partial_are_success() {
        assert!(from_game("Item", response(Status::Applied, 2, 2)).is_ok());
        assert!(from_game("Item", response(Status::Partial, 2, 1)).unwrap().summary.unwrap().contains("partial"));
        for status in [Status::Rejected, Status::NotReady, Status::Unknown] {
            assert!(from_game("Item", response(status, 2, 0)).is_err());
        }
        assert!(from_game("Item", response(Status::Applied, 2, 0)).is_err());
    }
}

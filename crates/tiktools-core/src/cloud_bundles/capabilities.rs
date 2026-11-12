//! Anonymous instance capability discovery and exact operation support.
use super::OperationError;
use crate::cloud_http::{self, Limits, NoContent};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::Duration;

/// A Cloud feature a host method depends on. The instance announces these in
/// `GET /api/v2/capabilities`; an absent key means "not available".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Feature {
    Media(&'static str),
    Overlays(&'static str),
    Packages(&'static str),
    Fork,
}

impl Feature {
    fn capability(self) -> &'static str {
        match self {
            Self::Media(_) => "media.operations",
            Self::Overlays(_) => "overlays.operations",
            Self::Packages(_) => "packages.operations",
            Self::Fork => "library.fork",
        }
    }

    fn operation(self) -> &'static str {
        match self {
            Self::Media(op) | Self::Overlays(op) | Self::Packages(op) => op,
            Self::Fork => "fork",
        }
    }

    pub(super) fn announced_by(self, document: &Value) -> bool {
        let operations = |pointer: &str, op: &str| {
            document
                .pointer(pointer)
                .and_then(Value::as_array)
                .is_some_and(|list| list.iter().any(|entry| entry.as_str() == Some(op)))
        };
        match self {
            Self::Media(op) => operations("/media/operations", op),
            Self::Overlays(op) => operations("/overlays/operations", op),
            Self::Packages(op) => operations("/packages/operations", op),
            Self::Fork => document.pointer("/library/fork/available") == Some(&Value::Bool(true)),
        }
    }

    pub(super) fn unavailable(self) -> OperationError {
        OperationError {
            code: "capability_unavailable",
            message: "this Cloud instance does not offer that feature".into(),
            data: Some(json!({
                "reason": "cloud_feature_unsupported",
                "capability": self.capability(),
                "operation": self.operation(),
            })),
        }
    }
}

/// 15 s and 64 KiB, like the SDK's own capability discovery.
const CAPABILITY_LIMITS: Limits = Limits::new(Duration::from_secs(15), 64 * 1024);

pub(super) fn map_capability_failure(status: StatusCode, _body: &[u8]) -> OperationError {
    match status.as_u16() {
        // A Cloud without the capability document is an older backend.
        404 => OperationError {
            code: "capability_unavailable",
            message: "this Cloud instance does not announce its features".into(),
            data: Some(json!({"reason": "capabilities_missing"})),
        },
        _ => OperationError::unavailable("the Cloud features could not be checked"),
    }
}

/// Reads the anonymous capability document. No credential is sent.
pub(super) async fn announced(origin: &str) -> Result<Value, OperationError> {
    let request = cloud_http::client()?.get(format!("{origin}/api/v2/capabilities"));
    cloud_http::send_json(
        request,
        CAPABILITY_LIMITS,
        NoContent::Unexpected,
        map_capability_failure,
    )
    .await
}

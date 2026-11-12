//! Cloud failure envelopes and operation-specific quota/conflict reasons.
use super::OperationError;
use crate::cloud_http::FailureEnvelope;
use reqwest::StatusCode;
use serde_json::json;

/// What a `409` means for one delete, so the UI can explain it.
#[derive(Clone, Copy)]
enum Blocked {
    /// An overlay revision still names the asset.
    AssetReferenced,
    /// A package still pins the overlay.
    OverlayPinned,
}

/// Cloud failure wording for overlays, packages, assets and the owner library.
/// It names none of them in particular, so every operation here reads sensibly.
pub(super) fn map_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    let error = match envelope.code() {
        Some("notFound") => OperationError::not_found("the Cloud item was not found"),
        Some("conflict") => OperationError::conflict("the item changed; reload and try again"),
        Some("invalidRequest") => OperationError::invalid("the Cloud rejected the request"),
        Some("unauthorized" | "forbidden") => {
            OperationError::forbidden("sign in to Cloud again to continue")
        }
        Some("rateLimited") => OperationError::unavailable("too many requests; try again later"),
        Some("quotaExceeded") => quota_exceeded(),
        Some("misconfigured" | "unsupported") => {
            OperationError::unavailable("this Cloud instance does not offer that feature")
        }
        _ => match status.as_u16() {
            401 | 403 => OperationError::forbidden("sign in to Cloud again to continue"),
            404 => OperationError::not_found("the Cloud item was not found"),
            409 => OperationError::conflict("the item changed; reload and try again"),
            507 => quota_exceeded(),
            _ => OperationError::unavailable("the Cloud is unavailable"),
        },
    };
    envelope.attach(error)
}

/// An account limit (assets, overlays, revisions, library) is full.
fn quota_exceeded() -> OperationError {
    OperationError {
        code: "quota_exceeded",
        message: "the Cloud storage limit for this account is reached; delete something first"
            .into(),
        data: None,
    }
}

fn map_blocked(status: StatusCode, body: &[u8], blocked: Blocked) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    if status.as_u16() != 409 && envelope.code() != Some("conflict") {
        return map_failure(status, body);
    }
    let (reason, message) = match blocked {
        Blocked::AssetReferenced => (
            "assetReferenced",
            "an overlay still uses this image; remove it from the overlay first",
        ),
        Blocked::OverlayPinned => (
            "overlayPinned",
            "a package still pins this overlay; delete the package first",
        ),
    };
    let mut error = envelope.attach(OperationError::conflict(message));
    let mut data = error.data.take().unwrap_or_else(|| json!({}));
    data["reason"] = json!(reason);
    error.data = Some(data);
    error
}

pub(super) fn map_asset_delete_failure(status: StatusCode, body: &[u8]) -> OperationError {
    map_blocked(status, body, Blocked::AssetReferenced)
}

pub(super) fn map_overlay_delete_failure(status: StatusCode, body: &[u8]) -> OperationError {
    map_blocked(status, body, Blocked::OverlayPinned)
}

/// Fork answers with bare status codes (the v2 library routes).
pub(super) fn map_fork_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let envelope = FailureEnvelope::parse(body);
    let reason = |error: OperationError, reason: &str| {
        let mut error = envelope_attach(error, body);
        let mut data = error.data.take().unwrap_or_else(|| json!({}));
        data["reason"] = json!(reason);
        error.data = Some(data);
        error
    };
    match status.as_u16() {
        404 => reason(
            OperationError::not_found("that document is not published, or it no longer exists"),
            "notPublished",
        ),
        409 => reason(
            OperationError::conflict("your library already has a document with this id"),
            "alreadyInLibrary",
        ),
        507 => reason(quota_exceeded(), "libraryQuota"),
        429 => reason(
            OperationError::unavailable("too many requests; try again later"),
            "rateLimited",
        ),
        _ => envelope.attach(match status.as_u16() {
            400 => OperationError::invalid("the Cloud rejected the request"),
            401 | 403 => OperationError::forbidden("sign in to Cloud again to continue"),
            _ => OperationError::unavailable("the Cloud is unavailable"),
        }),
    }
}

fn envelope_attach(error: OperationError, body: &[u8]) -> OperationError {
    FailureEnvelope::parse(body).attach(error)
}

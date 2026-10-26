//! Domain discovery values exposed by the native TikTok client.

use serde::{Deserialize, Serialize};

/// A room returned by TikTok's browser-free live search endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LiveRoom {
    pub unique_id: String,
    pub room_id: String,
    pub nickname: String,
    pub title: String,
    pub viewers: u64,
}

impl From<ttl_live_discovery::LiveRoom> for LiveRoom {
    fn from(room: ttl_live_discovery::LiveRoom) -> Self {
        Self {
            unique_id: room.unique_id,
            room_id: room.room_id,
            nickname: room.nickname,
            title: room.title,
            viewers: room.viewers,
        }
    }
}

/// Why a LIVE browse failed, at the granularity the control plane and the UI
/// need to react differently: sign in, slow down, retry, or give up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryFailureKind {
    /// TikTok requires a signed-in session for this request.
    AuthRequired,
    /// TikTok asked the client to slow down.
    RateLimited,
    /// The request never completed (network failure or timeout).
    Transport,
    /// TikTok answered, but with a shape we do not understand.
    InvalidResponse,
    /// TikTok refused for another reason (region, finished room, ...).
    Refused,
}

/// A classified LIVE browse failure. The message is safe to surface: it is
/// derived from TikTok's own status text and never carries the cookie header.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct DiscoveryFailure {
    pub kind: DiscoveryFailureKind,
    pub message: String,
}

impl DiscoveryFailure {
    /// Classify an upstream discovery error. `Refused` covers both the
    /// login-required refusal (TikTok answers HTTP 200 with a webcast
    /// `status_code`) and other webcast refusals; the status code embedded in
    /// the upstream message is what separates "sign in" from "slow down".
    pub fn from_error(error: ttl_live_discovery::DiscoveryError) -> Self {
        use ttl_live_discovery::DiscoveryError;
        let kind = match &error {
            DiscoveryError::Transport(_) => DiscoveryFailureKind::Transport,
            DiscoveryError::Status { status } => match status {
                401 | 403 => DiscoveryFailureKind::AuthRequired,
                429 => DiscoveryFailureKind::RateLimited,
                _ => DiscoveryFailureKind::Transport,
            },
            DiscoveryError::Decode(_) | DiscoveryError::TooLarge(_) => {
                DiscoveryFailureKind::InvalidResponse
            }
            DiscoveryError::EmptyResponse => DiscoveryFailureKind::AuthRequired,
            DiscoveryError::Refused(text) => classify_refusal(text),
            DiscoveryError::UserNotFound(_)
            | DiscoveryError::NoRoom(_)
            | DiscoveryError::Signer(_) => DiscoveryFailureKind::Refused,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

/// Split a webcast refusal into a login wall versus a rate limit. Extraction
/// is deliberately narrow: anything unrecognized stays `Refused` rather than
/// being guessed into a retryable bucket.
fn classify_refusal(text: &str) -> DiscoveryFailureKind {
    let lowered = text.to_ascii_lowercase();
    if (lowered.contains("rate limit") || lowered.contains("rate_limit"))
        || lowered.contains("too many")
        || (lowered.contains("too frequent") || lowered.contains("frequent requests"))
        || lowered.contains("slow down")
    {
        return DiscoveryFailureKind::RateLimited;
    }
    if lowered.contains("login") || refusal_status_code(text) == Some(2483) {
        return DiscoveryFailureKind::AuthRequired;
    }
    DiscoveryFailureKind::Refused
}

pub(super) fn is_rate_limit_refusal(text: &str) -> bool {
    classify_refusal(text) == DiscoveryFailureKind::RateLimited
}

/// Pull the numeric webcast status code out of a refusal message such as
/// `live search status_code=2483: Please login your account first`.
fn refusal_status_code(text: &str) -> Option<i64> {
    let after = text.split("status_code=").nth(1)?;
    let digits: String = after
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{classify_refusal, refusal_status_code, DiscoveryFailure, DiscoveryFailureKind};
    use ttl_live_discovery::DiscoveryError;

    #[test]
    fn login_required_refusal_is_auth_required() {
        let error = DiscoveryError::Refused(
            "live search status_code=2483: Please login your account first".to_owned(),
        );
        let failure = DiscoveryFailure::from_error(error);
        assert_eq!(failure.kind, DiscoveryFailureKind::AuthRequired);
    }

    #[test]
    fn rate_limit_refusal_is_rate_limited() {
        let error =
            DiscoveryError::Refused("live search status_code=1039: too many requests".to_owned());
        assert_eq!(
            DiscoveryFailure::from_error(error).kind,
            DiscoveryFailureKind::RateLimited
        );
    }

    #[test]
    fn unparseable_refusal_stays_refused_not_retryable() {
        let error =
            DiscoveryError::Refused("live search status_code=10041: room has finished".to_owned());
        assert_eq!(
            DiscoveryFailure::from_error(error).kind,
            DiscoveryFailureKind::Refused
        );
    }

    #[test]
    fn transport_and_decode_are_distinguished() {
        assert_eq!(
            DiscoveryFailure::from_error(DiscoveryError::Transport("socket closed".to_owned()))
                .kind,
            DiscoveryFailureKind::Transport
        );
        assert_eq!(
            DiscoveryFailure::from_error(DiscoveryError::Decode("bad json".to_owned())).kind,
            DiscoveryFailureKind::InvalidResponse
        );
        assert_eq!(
            DiscoveryFailure::from_error(DiscoveryError::EmptyResponse).kind,
            DiscoveryFailureKind::AuthRequired
        );
    }

    #[test]
    fn status_codes_are_extracted_only_from_the_marker() {
        assert_eq!(refusal_status_code("status_code=2483: nope"), Some(2483));
        assert_eq!(refusal_status_code("no marker here"), None);
        assert_eq!(
            classify_refusal("status_code=2483: Please login"),
            DiscoveryFailureKind::AuthRequired
        );
    }
}

//! Discovery/socket error classification and browse failure projection.
use crate::{discovery, TikTokError};

impl TikTokError {
    pub(in super::super) fn from_discovery(error: ttl_live_discovery::DiscoveryError) -> Self {
        use ttl_live_discovery::DiscoveryError;
        match &error {
            DiscoveryError::Status { status: 429 } => Self::RateLimited { retry_after: None },
            DiscoveryError::Transport(_) | DiscoveryError::Status { status: 500..=599 } => {
                Self::RetryableTransport(error.to_string())
            }
            DiscoveryError::Refused(text) if discovery::is_rate_limit_refusal(text) => {
                Self::RateLimited { retry_after: None }
            }
            _ => Self::Discovery(error.to_string()),
        }
    }

    pub(in super::super) fn from_socket(error: ttl_live_ws::WsError) -> Self {
        use ttl_live_ws::WsError;
        match &error {
            WsError::HttpStatus(429) => Self::RateLimited { retry_after: None },
            WsError::Transport(_) | WsError::Closed(_) | WsError::HttpStatus(500..=599) => {
                Self::RetryableTransport(error.to_string())
            }
            _ => Self::Transport(error.to_string()),
        }
    }
}

/// Classify a session-resolution failure for the LIVE browse path. A missing
/// guest identity is an authentication problem; anything else is transport.
pub(in super::super) fn session_failure(error: TikTokError) -> discovery::DiscoveryFailure {
    let kind = match &error {
        TikTokError::EmptySession => discovery::DiscoveryFailureKind::AuthRequired,
        TikTokError::RateLimited { .. } => discovery::DiscoveryFailureKind::RateLimited,
        _ => discovery::DiscoveryFailureKind::Transport,
    };
    discovery::DiscoveryFailure {
        kind,
        message: error.to_string(),
    }
}

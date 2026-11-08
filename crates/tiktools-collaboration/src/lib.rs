//! Host-owned persistent collaboration identity. No network, GUI, Cloud login,
//! plugin execution or ambient credentials are accessed by this crate.
mod authority;
mod commands;
mod delivery;
mod effects;
mod store;
mod streams;
mod types;
pub use authority::{
    Authorization, ExecutionAuthorization, ExecutionAuthorizations, ExecutionIdentity,
};
pub use commands::*;
pub use delivery::*;
pub use effects::*;
pub use store::{hash_secret, CollaborationService};
pub use streams::*;
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("subscription recovery is required")]
    ResyncRequired,
    #[error("collaboration recovery capacity reached")]
    Overloaded,
    #[error("collaboration snapshot exceeds its bounded budget")]
    TooLarge,
    #[error("requested collaboration domain or feed is not yet supported")]
    Unsupported,
    #[error("collaboration is disabled")]
    Disabled,
    #[error("remote control is unavailable in this host build")]
    GatewayUnavailable,
    #[error("collaboration is owned by another host; use its local control API")]
    HostInUse,
    #[error("{0}")]
    Invalid(&'static str),
    #[error("collaboration credential is invalid or expired")]
    Unauthorized,
    #[error("collaboration operation is forbidden")]
    Forbidden,
    #[error("pairing is awaiting owner approval")]
    PairingPending,
    #[error("pairing was rejected")]
    PairingRejected,
    #[error("pairing is invalid or expired")]
    PairingExpired,
    #[error("collaboration resource was not found")]
    NotFound,
    #[error("collaboration revision changed; reload before saving")]
    RevisionConflict,
    #[error("collaboration admission limit reached")]
    RateLimited,
    #[error("collaboration storage is unavailable")]
    Storage(#[from] rusqlite::Error),
    #[error("collaboration storage directory is unavailable")]
    Io(#[from] std::io::Error),
    #[error("collaboration state is invalid")]
    Json(#[from] serde_json::Error),
    #[error("secure randomness is unavailable")]
    Random,
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ResyncRequired => "resync_required",
            Self::Overloaded => "overloaded",
            Self::TooLarge => "too_large",
            Self::Unsupported => "unsupported_remote",
            Self::Disabled => "capability_unavailable",
            Self::GatewayUnavailable => "unsupported_remote",
            Self::HostInUse => "conflict",
            Self::Invalid(_) => "invalid_params",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::PairingPending => "pairing_pending",
            Self::PairingRejected => "pairing_rejected",
            Self::PairingExpired => "pairing_expired",
            Self::NotFound => "not_found",
            Self::RevisionConflict => "revision_conflict",
            Self::RateLimited => "rate_limited",
            _ => "storage",
        }
    }
}

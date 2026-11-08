//! Trusted caller identity. These types deliberately cannot be deserialized
//! from JSON-RPC params; only an authenticated transport constructs them.
use std::{
    collections::BTreeSet,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::ApiError;

pub use tiktools_core::collaboration::Capability as RemoteCapability;

struct Access {
    capabilities: BTreeSet<RemoteCapability>,
    expires_at: Instant,
    revoked: bool,
}

/// One already-authenticated session. Clones share revocation and grants.
/// This is an authorization boundary, not a credential validator or issuer.
#[derive(Clone)]
pub struct RemoteSession {
    principal_id: Arc<str>,
    session_id: Arc<str>,
    access: Arc<RwLock<Access>>,
    authority: Option<tiktools_core::collaboration::Authorization>,
    connection_id: Option<Arc<str>>,
    connection_limits: Option<tiktools_core::collaboration::RemoteLimits>,
}

impl RemoteSession {
    pub fn new(
        principal_id: impl Into<Arc<str>>,
        session_id: impl Into<Arc<str>>,
        capabilities: impl IntoIterator<Item = RemoteCapability>,
        lifetime: Duration,
    ) -> Result<Self, ApiError> {
        let principal_id = principal_id.into();
        let session_id = session_id.into();
        if principal_id.is_empty()
            || principal_id.len() > 256
            || session_id.is_empty()
            || session_id.len() > 256
        {
            return Err(ApiError::invalid_params(
                "invalid authenticated session identity",
            ));
        }
        let expires_at = Instant::now()
            .checked_add(lifetime)
            .ok_or_else(|| ApiError::invalid_params("invalid session lifetime"))?;
        Ok(Self {
            principal_id,
            session_id,
            access: Arc::new(RwLock::new(Access {
                capabilities: capabilities.into_iter().collect(),
                expires_at,
                revoked: false,
            })),
            authority: None,
            connection_id: None,
            connection_limits: None,
        })
    }

    pub fn from_authorization(
        authority: tiktools_core::collaboration::Authorization,
    ) -> Result<Self, ApiError> {
        let identity = authority.snapshot().map_err(ApiError::from)?;
        let mut session = Self::new(
            identity.principal_id,
            identity.session_id,
            identity.grant.capabilities,
            Duration::ZERO,
        )?;
        session.authority = Some(authority);
        Ok(session)
    }

    pub fn grant(&self) -> Result<tiktools_core::collaboration::Grant, ApiError> {
        self.authorize(None)?;
        if let Some(authority) = &self.authority {
            return Ok(authority.snapshot().map_err(ApiError::from)?.grant);
        }
        let access = self
            .access
            .read()
            .map_err(|_| ApiError::new("unauthorized", "remote session is unavailable"))?;
        Ok(tiktools_core::collaboration::Grant {
            capabilities: access.capabilities.clone(),
            ..Default::default()
        })
    }

    pub fn with_connection(
        mut self,
        id: impl Into<Arc<str>>,
        limits: tiktools_core::collaboration::RemoteLimits,
    ) -> Self {
        self.connection_id = Some(id.into());
        self.connection_limits = Some(limits);
        self
    }

    pub fn connection_id(&self) -> Option<&str> {
        self.connection_id.as_deref()
    }

    pub fn connection_limits(&self) -> Option<&tiktools_core::collaboration::RemoteLimits> {
        self.connection_limits.as_ref()
    }

    pub fn authenticated_identity(
        &self,
    ) -> Result<tiktools_core::collaboration::Authenticated, ApiError> {
        self.authorize(None)?;
        self.authority
            .as_ref()
            .ok_or_else(|| {
                ApiError::new(
                    "unsupported_remote",
                    "an authenticated network connection is required",
                )
            })?
            .snapshot()
            .map_err(ApiError::from)
    }

    /// Trusted transports can pass the live authorization view to core domain
    /// coordinators. It cannot be reconstructed from untrusted RPC metadata.
    pub fn authorization(&self) -> Result<tiktools_core::collaboration::Authorization, ApiError> {
        self.authorize(None)?;
        self.authority.clone().ok_or_else(|| {
            ApiError::new("unsupported_remote", "persistent authorization is required")
        })
    }

    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Host-side only: all contexts holding this session see the new grant.
    pub fn set_capabilities(&self, capabilities: impl IntoIterator<Item = RemoteCapability>) {
        if let Ok(mut access) = self.access.write() {
            access.capabilities = capabilities.into_iter().collect();
        }
    }

    pub fn revoke(&self) {
        if let Ok(mut access) = self.access.write() {
            access.revoked = true;
        }
    }

    pub(crate) fn authorize(&self, capability: Option<RemoteCapability>) -> Result<(), ApiError> {
        let access = self
            .access
            .read()
            .map_err(|_| ApiError::new("unauthorized", "remote session is unavailable"))?;
        if access.revoked || self.authority.is_none() && Instant::now() >= access.expires_at {
            return Err(ApiError::new(
                "unauthorized",
                "remote session is no longer valid",
            ));
        }
        if let Some(authority) = &self.authority {
            let identity = authority.snapshot().map_err(ApiError::from)?;
            return if capability
                .is_none_or(|capability| identity.grant.capabilities.contains(&capability))
            {
                Ok(())
            } else {
                Err(forbidden())
            };
        }
        if capability.is_some_and(|required| !access.capabilities.contains(&required)) {
            return Err(forbidden());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum RequestContext {
    LocalOwner,
    Remote(RemoteSession),
}

pub(crate) fn forbidden() -> ApiError {
    // Identical for hidden resources, unknown methods and missing grants.
    ApiError::new(
        "forbidden",
        "operation is not permitted for this remote session",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloned_sessions_observe_revocation_and_grant_changes() {
        let session = RemoteSession::new(
            "principal",
            "session",
            [RemoteCapability::LiveRead],
            Duration::from_secs(60),
        )
        .unwrap();
        let connection = session.clone();
        assert!(connection
            .authorize(Some(RemoteCapability::LiveRead))
            .is_ok());
        session.set_capabilities([]);
        assert_eq!(
            connection
                .authorize(Some(RemoteCapability::LiveRead))
                .unwrap_err()
                .code,
            "forbidden"
        );
        assert!(connection.authorize(None).is_ok());
        session.revoke();
        assert_eq!(connection.authorize(None).unwrap_err().code, "unauthorized");
    }

    #[test]
    fn expired_and_invalid_sessions_fail_closed() {
        let session = RemoteSession::new("p", "s", [], Duration::ZERO).unwrap();
        assert_eq!(session.authorize(None).unwrap_err().code, "unauthorized");
        assert!(RemoteSession::new("", "s", [], Duration::from_secs(1)).is_err());
    }
}

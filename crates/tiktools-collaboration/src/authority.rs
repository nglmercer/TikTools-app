//! Network connections hold these shared, revocable authorization views.
//! Reads never perform SQLite I/O on the async transport's event path.
mod execution;
use crate::peer_clock::PeerLease;
use crate::{Authenticated, Error, Grant};
pub use execution::{ExecutionAuthorization, ExecutionAuthorizations, ExecutionIdentity};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

pub(crate) struct AuthorizationState {
    pub(crate) identity: RwLock<Authenticated>,
    pub(crate) revoked: AtomicBool,
    pub(crate) peer_lease: RwLock<Option<PeerLease>>,
}

#[derive(Clone)]
pub struct Authorization {
    pub(crate) state: Arc<AuthorizationState>,
}

impl Authorization {
    pub(crate) fn new(identity: Authenticated) -> Self {
        Self::with_lease(identity, None)
    }

    pub(crate) fn with_lease(identity: Authenticated, lease: Option<PeerLease>) -> Self {
        Self {
            state: Arc::new(AuthorizationState {
                identity: RwLock::new(identity),
                revoked: AtomicBool::new(false),
                peer_lease: RwLock::new(lease),
            }),
        }
    }

    pub fn snapshot(&self) -> Result<Authenticated, Error> {
        if self.state.revoked.load(Ordering::SeqCst) {
            return Err(Error::Unauthorized);
        }
        let identity = self
            .state
            .identity
            .read()
            .map_err(|_| Error::Unauthorized)?;
        let lease = self
            .state
            .peer_lease
            .read()
            .map_err(|_| Error::Unauthorized)?;
        if let Some(lease) = &*lease {
            let mut current = identity.clone();
            current.access_expires_at = crate::store::now().saturating_add(lease.remaining()?);
            return Ok(current);
        }
        if crate::store::now() >= identity.access_expires_at {
            return Err(Error::Unauthorized);
        }
        Ok(identity.clone())
    }
}

impl AuthorizationState {
    pub(crate) fn revoke(&self) {
        self.revoked.store(true, Ordering::SeqCst);
    }

    pub(crate) fn update_grant(&self, principal: &str, grant: &Grant, revision: &str) {
        match self.identity.write() {
            Ok(mut identity) if identity.principal_id == principal => {
                identity.grant = grant.clone();
                identity.grant_revision = revision.into();
            }
            Err(_) => self.revoke(),
            _ => {}
        }
    }
}

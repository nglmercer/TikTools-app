//! Network connections hold these shared, revocable authorization views.
//! Reads never perform SQLite I/O on the async transport's event path.
mod execution;
use crate::{Authenticated, Error, Grant};
pub use execution::{ExecutionAuthorization, ExecutionAuthorizations, ExecutionIdentity};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

pub(crate) struct AuthorizationState {
    pub(crate) identity: RwLock<Authenticated>,
    pub(crate) revoked: AtomicBool,
}

#[derive(Clone)]
pub struct Authorization {
    pub(crate) state: Arc<AuthorizationState>,
}

impl Authorization {
    pub(crate) fn new(identity: Authenticated) -> Self {
        Self {
            state: Arc::new(AuthorizationState {
                identity: RwLock::new(identity),
                revoked: AtomicBool::new(false),
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

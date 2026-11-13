//! Saved definition subjects use durable session identity, not bearer lifetime.
use crate::{ClientKind, CollaborationService, Error, Grant};
use std::sync::Arc;

/// Current host-resolved identity data, intentionally not deserializable.
///
/// This is a snapshot, never an execution approval or a permanent grant. Use
/// `ExecutionAuthorizations::with_current` at synchronous final admission and
/// bind the current revisions in any separately checked definition manifest.
#[derive(Clone, Debug)]
pub struct ExecutionIdentity {
    pub(crate) host_id: String,
    pub(crate) principal_id: String,
    pub(crate) session_id: String,
    pub(crate) client_kind: ClientKind,
    pub(crate) grant: Grant,
    pub(crate) grant_revision: String,
    pub(crate) configuration_revision: String,
    pub(crate) absolute_expires_at: i64,
    pub(crate) idle_expires_at: i64,
    pub(crate) peer_lease: Option<crate::peer_clock::PeerLease>,
}
impl ExecutionIdentity {
    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn client_kind(&self) -> ClientKind {
        self.client_kind
    }
    pub fn grant(&self) -> &Grant {
        &self.grant
    }
    pub fn grant_revision(&self) -> &str {
        &self.grant_revision
    }
    pub fn configuration_revision(&self) -> &str {
        &self.configuration_revision
    }
    pub fn absolute_expires_at(&self) -> i64 {
        self.absolute_expires_at
    }
    pub fn idle_expires_at(&self) -> i64 {
        self.idle_expires_at
    }
    pub fn expires_at(&self) -> i64 {
        self.absolute_expires_at.min(self.idle_expires_at)
    }
    /// Recheck immediately before admission if synchronous work inside the
    /// closure was long enough to cross a deadline. Does not re-read grants.
    pub fn check_deadline(&self) -> Result<(), Error> {
        if let Some(lease) = &self.peer_lease {
            return lease.remaining().map(|_| ());
        }
        if crate::store::now() >= self.expires_at() {
            Err(Error::Unauthorized)
        } else {
            Ok(())
        }
    }
}

/// Reconstructible host-side subject handle. No bearer token is held or read.
/// Only the owning service verifies the supplied principal/session relation.
#[derive(Clone)]
pub struct ExecutionAuthorization {
    pub(crate) batch: ExecutionAuthorizations,
}
impl ExecutionAuthorization {
    pub fn snapshot(&self) -> Result<ExecutionIdentity, Error> {
        self.with_current(Clone::clone)
    }
    /// Holds the service DB mutex through the synchronous admission closure.
    /// Never await or reenter collaboration from this closure. Permission,
    /// disable and revoke commits are ordered before or after the closure.
    /// This does not claim completion of an external asynchronous effect.
    pub fn with_current<R>(&self, apply: impl FnOnce(&ExecutionIdentity) -> R) -> Result<R, Error> {
        self.batch.with_current(|identities| apply(&identities[0]))
    }
}

/// All subjects of one manifest resolved under a single service DB lock.
///
/// Do not nest single-subject admission closures: they use the same mutex.
/// An empty batch supplies `[]`, not Owner authority, a grant or an approval.
/// Callers derive any Owner authority separately from trusted host provenance.
#[derive(Clone)]
pub struct ExecutionAuthorizations {
    pub(crate) service: Arc<CollaborationService>,
    pub(crate) identities: Vec<(String, String)>,
}
impl ExecutionAuthorizations {
    /// Conjoin subject restrictions without dropping either context. Handles
    /// from different service owners cannot be combined, even if copied stores
    /// happen to carry the same host ID. No grant or approval is manufactured.
    pub fn intersect(&self, other: &Self) -> Result<Self, Error> {
        if !Arc::ptr_eq(&self.service, &other.service) {
            return Err(Error::Forbidden);
        }
        let mut seen = std::collections::BTreeSet::new();
        let subjects: Vec<_> = self
            .identities
            .iter()
            .chain(&other.identities)
            .filter(|subject| seen.insert((*subject).clone()))
            .cloned()
            .collect();
        self.service.execution_authorizations(&subjects)
    }
    pub fn is_empty(&self) -> bool {
        self.identities.is_empty()
    }
    pub fn len(&self) -> usize {
        self.identities.len()
    }
    pub fn snapshot(&self) -> Result<Vec<ExecutionIdentity>, Error> {
        self.with_current(<[ExecutionIdentity]>::to_vec)
    }
    /// Resolve every subject immediately before one synchronous admission.
    ///
    /// The collaboration DB mutex remains held through `apply`; revocation,
    /// configuration and grant writers cannot commit inside this boundary.
    /// The closure must not reenter collaboration or await. If a caller holds
    /// a business writer too, acquire it before this admission and release the
    /// collaboration lock before audit/outbox import. Only the synchronous
    /// admission is ordered, not future network dispatch or effect completion.
    pub fn with_current<R>(
        &self,
        apply: impl FnOnce(&[ExecutionIdentity]) -> R,
    ) -> Result<R, Error> {
        self.service
            .with_execution_identities(&self.identities, apply)
    }
}

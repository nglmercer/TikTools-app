//! Native-host bridge from freshly verified Cloud grants to local authority.
use super::configuration::config;
use super::*;
pub(crate) mod lease;
mod reconciliation;
#[cfg(test)]
mod tests;
pub use reconciliation::{CloudPeerObservation, CloudPeerSource};

pub const PEER_OFFLINE_TTL_MS: i64 = 8 * 60 * MINUTE;

/// Only the trusted host may import this after authenticated Cloud discovery
/// and host grant confirmation. Never expose this operation as a remote RPC.
#[derive(Clone)]
pub struct CloudPeerGrant {
    pub origin: String,
    pub account_id: String,
    pub host_id: String,
    pub device_id: String,
    pub grant_id: String,
    pub cloud_revision: i64,
    pub endpoint_id: String,
    pub label: String,
    pub client_kind: ClientKind,
    pub grant: Grant,
}

impl CloudPeerGrant {
    fn source(&self) -> Result<String, Error> {
        let source = CloudPeerSource {
            origin: self.origin.clone(),
            account_id: self.account_id.clone(),
            host_id: self.host_id.clone(),
        }
        .key()?;
        if self.label.chars().count() > 120
            || self.label.chars().any(char::is_control)
            || self.client_kind == ClientKind::Browser
            || self.cloud_revision <= 0
            || self.cloud_revision == i64::MAX
            || self.endpoint_id.len() != 64
            || !self
                .endpoint_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Invalid("invalid Cloud peer binding"));
        }
        for id in [&self.host_id, &self.device_id, &self.grant_id] {
            if Uuid::parse_str(id).map_or(true, |parsed| parsed.to_string() != *id) {
                return Err(Error::Invalid("invalid Cloud peer binding ID"));
            }
        }
        self.grant.validate()?;
        Ok(source)
    }
}

struct StoredBinding {
    principal: String,
    session: String,
    endpoint: String,
    cloud_revision: i64,
    grant_id: String,
    applied: i64,
    previous_json: String,
    revoked: bool,
    session_revoked: bool,
}

impl CollaborationService {
    /// Applies a fresh confirmed grant without creating Owner authority.
    /// Endpoint rotation, Cloud rewinds, local grant edits and local revocation
    /// require explicit owner reconciliation; background sync cannot undo them.
    pub fn apply_cloud_peer_grant(&self, input: CloudPeerGrant) -> Result<Authenticated, Error> {
        let source = input.source()?;
        let mut db = self.db()?;
        enabled(&db)?;
        if config(&db)?.peer.is_none() {
            return Err(Error::Disabled);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous: Option<StoredBinding> = tx.query_row(
            "SELECT b.principal_id,b.session_id,b.endpoint_id,b.cloud_revision,b.grant_id,b.applied_revision,p.grant_json,p.revoked,s.revoked FROM cloud_peer_bindings b JOIN principals p ON p.id=b.principal_id JOIN sessions s ON s.id=b.session_id WHERE b.source=?1 AND b.device_id=?2",
            params![source,input.device_id], |row| Ok(StoredBinding { principal:row.get(0)?, session:row.get(1)?, endpoint:row.get(2)?, cloud_revision:row.get(3)?, grant_id:row.get(4)?, applied:row.get(5)?, previous_json:row.get(6)?, revoked:row.get(7)?, session_revoked:row.get(8)? }),
        ).optional()?;
        let grant_json = serde_json::to_string(&input.grant)?;
        let peer_lease = crate::peer_clock::PeerLease::issue()?;
        let until = now()
            .checked_add(PEER_OFFLINE_TTL_MS)
            .ok_or(Error::Unauthorized)?;
        let (principal, session, revision) = match previous {
            Some(StoredBinding {
                principal,
                session,
                endpoint,
                cloud_revision,
                grant_id,
                applied,
                previous_json,
                revoked,
                session_revoked,
            }) => {
                if revoked || session_revoked {
                    return Err(Error::Unauthorized);
                }
                if endpoint != input.endpoint_id
                    || grant_id != input.grant_id
                    || input.cloud_revision < cloud_revision
                    || input.cloud_revision == cloud_revision && previous_json != grant_json
                {
                    return Err(Error::RevisionConflict);
                }
                let local_revision: i64 = tx.query_row(
                    "SELECT revision FROM principals WHERE id=?1",
                    [&principal],
                    |r| r.get(0),
                )?;
                let previous_kind: String =
                    tx.query_row("SELECT kind FROM sessions WHERE id=?1", [&session], |r| {
                        r.get(0)
                    })?;
                if previous_kind != kind(input.client_kind) {
                    return Err(Error::RevisionConflict);
                }
                if local_revision != applied {
                    return Err(Error::RevisionConflict);
                }
                let revision = if cloud_revision == input.cloud_revision {
                    applied
                } else {
                    applied.checked_add(1).ok_or(Error::RevisionConflict)?
                };
                tx.execute(
                    "UPDATE principals SET grant_json=?2,revision=?3,label=?4 WHERE id=?1",
                    params![principal, grant_json, revision, input.label],
                )?;
                tx.execute("UPDATE sessions SET access_expires=?2,absolute_expires=?2,last_activity=?3 WHERE id=?1", params![session,until,now()])?;
                tx.execute("UPDATE cloud_peer_bindings SET cloud_revision=?3,applied_revision=?4,valid_until=?5 WHERE source=?1 AND device_id=?2", params![source,input.device_id,input.cloud_revision,revision,until])?;
                (principal, session, revision)
            }
            None => {
                let principal = Uuid::new_v4().to_string();
                let session = Uuid::new_v4().to_string();
                let collision: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM cloud_peer_bindings WHERE endpoint_id=?1)",
                    [&input.endpoint_id],
                    |r| r.get(0),
                )?;
                if collision {
                    return Err(Error::Forbidden);
                }
                tx.execute(
                    "INSERT INTO principals(id,label,kind,grant_json) VALUES(?1,?2,?3,?4)",
                    params![principal, input.label, kind(input.client_kind), grant_json],
                )?;
                // No bearer credential is issued. Only the authenticated Iroh
                // endpoint key can select this session; these random verifiers
                // cannot be recovered as access/refresh credentials.
                tx.execute("INSERT INTO sessions(id,principal_id,kind,access_hash,access_expires,refresh_hash,created_at,last_activity,absolute_expires) VALUES(?1,?2,?3,?4,?5,NULL,?6,?6,?5)", params![session,principal,kind(input.client_kind),hash_secret(&secret()?),until,now()])?;
                tx.execute("INSERT INTO cloud_peer_bindings(source,device_id,grant_id,endpoint_id,cloud_revision,principal_id,session_id,applied_revision,valid_until) VALUES(?1,?2,?3,?4,?5,?6,?7,1,?8)", params![source,input.device_id,input.grant_id,input.endpoint_id,input.cloud_revision,principal,session,until])?;
                (principal, session, 1)
            }
        };
        lease::save(&tx, &session, &peer_lease)?;
        audit(
            &tx,
            "cloud-peer-bridge",
            "peer.grant.confirmed",
            &principal,
            "succeeded",
        )?;
        tx.commit()?;
        let identity = Authenticated {
            principal_id: principal,
            session_id: session,
            grant: input.grant,
            grant_revision: revision.to_string(),
            client_kind: input.client_kind,
            access_expires_at: until,
        };
        self.update_authorizations(|state| match state.identity.write() {
            Ok(mut current) if current.principal_id == identity.principal_id => {
                *current = identity.clone();
                match state.peer_lease.write() {
                    Ok(mut lease) => *lease = Some(peer_lease.clone()),
                    Err(_) => state.revoke(),
                }
            }
            Err(_) => state.revoke(),
            _ => {}
        })?;
        Ok(identity)
    }

    /// The caller must pass the key authenticated by the encrypted transport,
    /// never an endpoint ID supplied inside an application message.
    pub fn authenticate_peer(&self, endpoint: &str) -> Result<Authorization, Error> {
        self.authenticate_peer_binding(endpoint, None)
    }

    /// WebRTC signatures must match the exact locally imported Cloud binding.
    /// The source is supplied by native host configuration, never by SDP/RPC.
    pub fn authenticate_rtc_peer(
        &self,
        endpoint: &str,
        source: &CloudPeerSource,
        device: &str,
        revision: i64,
    ) -> Result<Authorization, Error> {
        self.authenticate_peer_binding(endpoint, Some((source, device, revision)))
    }
    fn authenticate_peer_binding(
        &self,
        endpoint: &str,
        binding: Option<(&CloudPeerSource, &str, i64)>,
    ) -> Result<Authorization, Error> {
        let source = binding.map(|(source, _, _)| source.key()).transpose()?;
        let device = binding.map(|(_, device, _)| device);
        let revision = binding.map(|(_, _, revision)| revision);
        let db = self.db()?;
        enabled(&db)?;
        if config(&db)?.peer.is_none() {
            return Err(Error::Disabled);
        }
        let mut identity = db.query_row("SELECT p.id,s.id,p.grant_json,p.revision,s.kind,b.valid_until FROM cloud_peer_bindings b JOIN principals p ON p.id=b.principal_id JOIN sessions s ON s.id=b.session_id WHERE b.endpoint_id=?1 AND (?2 IS NULL OR (b.source=?2 AND b.device_id=?3 AND b.cloud_revision=?4)) AND b.valid_until>0 AND p.revoked=0 AND s.revoked=0 AND s.absolute_expires>0", params![endpoint,source,device,revision], |r| Ok(Authenticated { principal_id:r.get(0)?, session_id:r.get(1)?, grant:grant(r,2)?, grant_revision:r.get::<_,i64>(3)?.to_string(), client_kind:parse_kind(&r.get::<_,String>(4)?)?, access_expires_at:r.get(5)? })).optional()?.ok_or(Error::Unauthorized)?;
        let lease = lease::for_session(&db, &identity.session_id)?.ok_or(Error::Unauthorized)?;
        identity.access_expires_at = now().saturating_add(lease.remaining()?);
        let mut views = self
            .authorizations
            .lock()
            .map_err(|_| Error::Unauthorized)?;
        views.retain(|_, view| view.strong_count() > 0);
        let key = format!("peer:{endpoint}");
        if let Some(state) = views.get(&key).and_then(Weak::upgrade) {
            if state.revoked.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(Error::Unauthorized);
            }
            let mut current = state.identity.write().map_err(|_| Error::Unauthorized)?;
            *current = identity;
            *state.peer_lease.write().map_err(|_| Error::Unauthorized)? = Some(lease);
            drop(current);
            return Ok(Authorization { state });
        }
        let authority = Authorization::with_lease(identity, Some(lease));
        views.insert(key, Arc::downgrade(&authority.state));
        Ok(authority)
    }
}

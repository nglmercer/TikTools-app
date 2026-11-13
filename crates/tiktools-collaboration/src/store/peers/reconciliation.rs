//! Suspend stale Cloud projections without changing local Owner grants.
use super::*;

#[derive(Clone)]
pub struct CloudPeerSource {
    pub origin: String,
    pub account_id: String,
    pub host_id: String,
}
impl CloudPeerSource {
    pub(super) fn key(&self) -> Result<String, Error> {
        let origin =
            url::Url::parse(&self.origin).map_err(|_| Error::Invalid("invalid peer origin"))?;
        if origin.scheme() != "https"
            || origin.origin().ascii_serialization() != self.origin
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || self.account_id.is_empty()
            || self.account_id.len() > 256
            || self.account_id.chars().any(char::is_control)
            || Uuid::parse_str(&self.host_id).map_or(true, |id| id.to_string() != self.host_id)
        {
            return Err(Error::Invalid("invalid Cloud peer source"));
        }
        Ok(hash_secret(&format!(
            "peer-source-v1\0{}\0{}\0{}",
            self.origin, self.account_id, self.host_id
        )))
    }
}

/// One entry from a complete, freshly authenticated host grant snapshot.
#[derive(Clone)]
pub struct CloudPeerObservation {
    pub device_id: String,
    pub grant_id: String,
    pub revision: i64,
    pub approved: bool,
    pub revoked: bool,
}

impl CollaborationService {
    /// Called only after a successful complete Cloud snapshot, never after a
    /// timeout or a short-lived address disappearing from discovery.
    pub fn reconcile_cloud_peers(
        &self,
        source: &CloudPeerSource,
        observations: &[CloudPeerObservation],
    ) -> Result<(), Error> {
        let key = source.key()?;
        if observations.len() > 256 {
            return Err(Error::TooLarge);
        }
        let mut observed = BTreeMap::new();
        for item in observations {
            if item.revision <= 0
                || item.revision == i64::MAX
                || [item.device_id.as_str(), item.grant_id.as_str()]
                    .iter()
                    .any(|id| Uuid::parse_str(id).map_or(true, |v| v.to_string() != *id))
                || observed
                    .insert((item.device_id.clone(), item.grant_id.clone()), item)
                    .is_some()
            {
                return Err(Error::Invalid("invalid Cloud peer snapshot"));
            }
        }
        self.suspend_cloud_peers(Some(&key), |device, grant, revision| {
            match observed.get(&(device.to_owned(), grant.to_owned())) {
                Some(item) if item.grant_id == grant && item.revision >= revision => {
                    if item.revoked {
                        (true, true)
                    } else {
                        (!item.approved || item.revision != revision, false)
                    }
                }
                // A stale snapshot cannot remove a more recent confirmed grant.
                Some(item) if item.grant_id == grant && item.revision < revision => (false, false),
                _ => (true, false),
            }
        })
    }

    /// Account logout/switch suspends other Cloud sources. An offline but
    /// remembered identity may retain its existing lease; no lease is renewed.
    pub fn retain_cloud_peer_source(&self, source: Option<&CloudPeerSource>) -> Result<(), Error> {
        let retained = source.map(CloudPeerSource::key).transpose()?;
        self.suspend_cloud_peers_except(retained.as_deref())
    }

    fn suspend_cloud_peers_except(&self, retained: Option<&str>) -> Result<(), Error> {
        self.suspend_cloud_peers(None, |source, _, _| (Some(source) != retained, false))
    }

    fn suspend_cloud_peers(
        &self,
        source: Option<&str>,
        decide: impl Fn(&str, &str, i64) -> (bool, bool),
    ) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let bindings = {
            let mut stmt=tx.prepare("SELECT b.source,b.device_id,b.grant_id,b.cloud_revision,b.principal_id,b.session_id,b.endpoint_id,b.valid_until,p.revoked FROM cloud_peer_bindings b JOIN principals p ON p.id=b.principal_id")?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, i64>(7)?,
                        r.get::<_, bool>(8)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let mut suspended = Vec::new();
        for (
            binding_source,
            device,
            grant,
            revision,
            principal,
            session,
            endpoint,
            until,
            already_revoked,
        ) in bindings
        {
            if source.is_some_and(|key| key != binding_source) {
                continue;
            }
            let (suspend, revoked) = decide(
                if source.is_some() {
                    &device
                } else {
                    &binding_source
                },
                &grant,
                revision,
            );
            if !suspend || (until == 0 && (!revoked || already_revoked)) {
                continue;
            }
            tx.execute(
                "UPDATE cloud_peer_bindings SET valid_until=0 WHERE endpoint_id=?1",
                [&endpoint],
            )?;
            tx.execute(
                "UPDATE sessions SET access_expires=0,absolute_expires=0 WHERE id=?1",
                [&session],
            )?;
            if revoked {
                tx.execute(
                    "UPDATE principals SET revoked=1,revision=revision+1 WHERE id=?1",
                    [&principal],
                )?;
                tx.execute("UPDATE sessions SET revoked=1 WHERE id=?1", [&session])?;
            }
            audit(
                &tx,
                "cloud-peer-bridge",
                "peer.grant.suspended",
                &principal,
                "succeeded",
            )?;
            suspended.push((principal, endpoint));
        }
        tx.commit()?;
        // Existing readers stay irrevocably fenced. A fresh Cloud grant can
        // create a new view, but cannot revive an old connection's authority.
        self.update_authorizations(|state| match state.identity.read() {
            Ok(identity) if suspended.iter().any(|(id, _)| id == &identity.principal_id) => {
                state.revoke()
            }
            Err(_) => state.revoke(),
            _ => {}
        })?;
        let mut views = self
            .authorizations
            .lock()
            .map_err(|_| Error::Unauthorized)?;
        for (_, endpoint) in suspended {
            views.remove(&format!("peer:{endpoint}"));
        }
        Ok(())
    }
}

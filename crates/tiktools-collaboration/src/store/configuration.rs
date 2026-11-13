//! Configuration revisions, gateway status and disabling remote access.
use super::*;

impl CollaborationService {
    pub fn status(&self) -> Result<Status, Error> {
        let db = self.db()?;
        Ok(Status {
            host_id: self.host_id.clone(), host_instance_id: self.instance_id.clone(), configuration: config(&db)?,
            configuration_revision: config_revision(&db)?.to_string(),
            active_sessions: db.query_row("SELECT count(*) FROM sessions s JOIN principals p ON p.id=s.principal_id WHERE s.revoked=0 AND p.revoked=0 AND s.absolute_expires>?1 AND s.last_activity>?2", params![now(), now().saturating_sub(IDLE_TTL)], |row| row.get(0))?,
            pending_requests: db.query_row("SELECT count(*) FROM pairing_requests WHERE state='pending' AND expires_at>?1", [now()], |row| row.get(0))?,
            gateway:self.gateway_status()?,
            peer_gateway:self.peer_gateway_status()?,
        })
    }

    pub fn configure(&self, value: Configuration) -> Result<Status, Error> {
        self.configure_checked(value, None)
    }

    pub fn configure_checked(
        &self,
        value: Configuration,
        expected: Option<&str>,
    ) -> Result<Status, Error> {
        validate_configuration(&value)?;
        let expected = expected.map(crate::stream_sequence).transpose()?;
        {
            let mut db = self.db()?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let previous = config_revision(&tx)?;
            if expected.is_some_and(|revision| revision != previous as u64) {
                return Err(Error::RevisionConflict);
            }
            let revision = previous.checked_add(1).ok_or(Error::Invalid(
                "collaboration configuration revision exhausted",
            ))?;
            tx.execute(
                "UPDATE metadata SET value=?1 WHERE key='configuration'",
                [serde_json::to_string(&value)?],
            )?;
            tx.execute(
                "UPDATE metadata SET value=?1 WHERE key='configuration-revision'",
                [revision.to_string()],
            )?;
            if !value.enabled {
                tx.execute("UPDATE sessions SET revoked=1", [])?;
                tx.execute(
                    "UPDATE invitations SET state='revoked' WHERE state='created'",
                    [],
                )?;
                tx.execute("UPDATE pairing_requests SET state='revoked' WHERE state IN ('pending','approved')", [])?;
            }
            audit(&tx, "local-owner", "configure", &self.host_id, "succeeded")?;
            tx.commit()?;
            if !value.enabled {
                self.update_authorizations(|state| state.revoke())?;
            }
        }
        self.status()
    }

    pub fn configuration(&self) -> Result<Configuration, Error> {
        let db = self.db()?;
        config(&db)
    }

    /// A durable generation makes reapplying an unchanged configuration reload
    /// certificate files. Read configuration and generation under one DB lock.
    pub fn configuration_snapshot(&self) -> Result<(Configuration, String), Error> {
        let db = self.db()?;
        Ok((config(&db)?, config_revision(&db)?.to_string()))
    }

    pub fn gateway_status(&self) -> Result<GatewayStatus, Error> {
        self.gateway
            .lock()
            .map(|status| status.clone())
            .map_err(|_| Error::Invalid("gateway state is unavailable"))
    }

    pub fn peer_gateway_status(&self) -> Result<PeerGatewayStatus, Error> {
        self.peer_gateway
            .lock()
            .map(|status| status.clone())
            .map_err(|_| Error::Invalid("peer gateway state is unavailable"))
    }

    pub fn update_peer_gateway_status(
        &self,
        update: impl FnOnce(&mut PeerGatewayStatus),
    ) -> Result<(), Error> {
        let mut status = self
            .peer_gateway
            .lock()
            .map_err(|_| Error::Invalid("peer gateway state is unavailable"))?;
        update(&mut status);
        Ok(())
    }

    pub fn set_gateway_status(&self, status: GatewayStatus) -> Result<(), Error> {
        *self
            .gateway
            .lock()
            .map_err(|_| Error::Invalid("gateway state is unavailable"))? = status;
        Ok(())
    }

    pub fn update_gateway_status(
        &self,
        update: impl FnOnce(&mut GatewayStatus),
    ) -> Result<(), Error> {
        let mut status = self
            .gateway
            .lock()
            .map_err(|_| Error::Invalid("gateway state is unavailable"))?;
        update(&mut status);
        Ok(())
    }
}

pub(super) fn config(db: &Connection) -> Result<Configuration, Error> {
    let value: String = db.query_row(
        "SELECT value FROM metadata WHERE key='configuration'",
        [],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&value)?)
}

pub(super) fn config_revision(db: &Connection) -> Result<i64, Error> {
    let revision: String = db.query_row(
        "SELECT value FROM metadata WHERE key='configuration-revision'",
        [],
        |row| row.get(0),
    )?;
    revision
        .parse()
        .ok()
        .filter(|version| *version > 0)
        .ok_or(Error::Invalid(
            "invalid collaboration configuration revision",
        ))
}

pub(super) fn enabled(db: &Connection) -> Result<(), Error> {
    if config(db)?.enabled {
        Ok(())
    } else {
        Err(Error::Disabled)
    }
}

fn validate_configuration(value: &Configuration) -> Result<(), Error> {
    value.limits.validate()?;
    if let Some(peer) = &value.peer {
        if peer.relay_urls.len() > 4 {
            return Err(Error::Invalid("too many peer relays"));
        }
        for raw in &peer.relay_urls {
            let url = url::Url::parse(raw).map_err(|_| Error::Invalid("invalid peer relay"))?;
            if raw.len() > 2048
                || url.scheme() != "https"
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
            {
                return Err(Error::Invalid("peer relays must be HTTPS origins"));
            }
        }
    }
    let bind: std::net::SocketAddr = value
        .bind_address
        .parse()
        .map_err(|_| Error::Invalid("invalid collaboration bind address"))?;
    if value.tls_certificate.is_some() != value.tls_private_key.is_some() {
        return Err(Error::Invalid(
            "both TLS certificate and private key are required",
        ));
    }
    if value.tls_proxy && (!bind.ip().is_loopback() || value.tls_certificate.is_some()) {
        return Err(Error::Invalid(
            "TLS proxy backend must bind loopback without a second TLS configuration",
        ));
    }
    let validate_origin = |origin: &str| -> Result<url::Url, Error> {
        let url =
            url::Url::parse(origin).map_err(|_| Error::Invalid("invalid collaboration origin"))?;
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(url.scheme() == "https"
            || url.scheme() == "http" && loopback && bind.ip().is_loopback())
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || url.origin().ascii_serialization() != origin
        {
            return Err(Error::Invalid(
                "collaboration origins must be exact HTTPS origins; HTTP is loopback-only",
            ));
        }
        Ok(url)
    };
    if value.allowed_origins.len() > 8 {
        return Err(Error::Invalid("too many allowed collaboration origins"));
    }
    for origin in &value.allowed_origins {
        validate_origin(origin)?;
    }
    if let Some(origin) = &value.advertised_origin {
        let url = validate_origin(origin)?;
        if value.enabled
            && url.scheme() == "https"
            && !value.tls_proxy
            && value.tls_certificate.is_none()
        {
            return Err(Error::Invalid(
                "HTTPS collaboration requires TLS configuration",
            ));
        }
    } else if value.enabled && value.peer.is_none() {
        return Err(Error::Invalid(
            "an advertised origin is required before enabling collaboration",
        ));
    }
    Ok(())
}

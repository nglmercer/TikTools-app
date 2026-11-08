use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex, MutexGuard, Weak},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ring::digest::{digest, SHA256};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use uuid::Uuid;

use crate::*;
mod execution;

const MINUTE: i64 = 60_000;
const INVITE_TTL: i64 = 5 * MINUTE;
const ACCESS_TTL: i64 = 15 * MINUTE;
const IDLE_TTL: i64 = 24 * 60 * MINUTE;
const SESSION_TTL: i64 = 7 * IDLE_TTL;

#[cfg(test)]
std::thread_local! {
    static TEST_NOW: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

/// Uses one private connection, serialized at transaction boundaries. Callers
/// run these synchronous methods on a blocking pool, never the UI thread.
pub struct CollaborationService {
    db: Mutex<Connection>,
    host_id: String,
    instance_id: String,
    authorizations: Mutex<BTreeMap<String, Weak<crate::authority::AuthorizationState>>>,
    gateway: Mutex<GatewayStatus>,
    _ownership: std::fs::File,
}

pub fn hash_secret(secret: &str) -> String {
    hex(digest(&SHA256, secret.as_bytes()).as_ref())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn secret() -> Result<String, Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Random)?;
    Ok(hex(&bytes))
}

pub(crate) fn now() -> i64 {
    #[cfg(test)]
    if let Some(timestamp) = TEST_NOW.with(std::cell::Cell::get) {
        return timestamp;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Read lazily so large grants reduce a page rather than allocate every row.
/// The envelope stays below the default 1 MiB transport budget.
fn page<T: serde::Serialize>(
    rows: impl Iterator<Item = rusqlite::Result<(String, T)>>,
    limit: usize,
) -> Result<ListPage<T>, Error> {
    const MAX_PAGE_BYTES: usize = 512 * 1024;
    let mut items = Vec::new();
    let mut bytes = 0;
    let mut last = None;
    let mut next_after = None;
    for row in rows {
        let (id, value) = row?;
        let size = serde_json::to_vec(&value)?.len();
        if items.len() == limit || bytes + size > MAX_PAGE_BYTES {
            if items.is_empty() {
                return Err(Error::Invalid(
                    "collaboration list item exceeds page budget",
                ));
            }
            next_after = last;
            break;
        }
        bytes += size;
        last = Some(id);
        items.push(value);
    }
    Ok(ListPage { items, next_after })
}

fn kind(kind: ClientKind) -> &'static str {
    match kind {
        ClientKind::Desktop => "desktop",
        ClientKind::Browser => "browser",
        ClientKind::Agent => "agent",
    }
}

fn parse_kind(value: &str) -> rusqlite::Result<ClientKind> {
    match value {
        "desktop" => Ok(ClientKind::Desktop),
        "browser" => Ok(ClientKind::Browser),
        "agent" => Ok(ClientKind::Agent),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn grant(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Grant> {
    let json: String = row.get(index)?;
    serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn audit(
    db: &Connection,
    actor: &str,
    action: &str,
    resource: &str,
    outcome: &str,
) -> Result<(), Error> {
    db.execute("INSERT INTO audit(timestamp,principal_id,action,resource_id,outcome) VALUES (?1,?2,?3,?4,?5)", params![now(), actor, action, resource, outcome])?;
    prune_audit(db)?;
    Ok(())
}

fn prune_audit(db: &Connection) -> Result<(), Error> {
    let cutoff = now().saturating_sub(30 * IDLE_TTL);
    db.execute("DELETE FROM audit WHERE timestamp<?1", [cutoff])?;
    db.execute(
        "DELETE FROM audit_imports WHERE source_timestamp<?1",
        [cutoff],
    )?;
    Ok(())
}

fn config(db: &Connection) -> Result<Configuration, Error> {
    let value: String = db.query_row(
        "SELECT value FROM metadata WHERE key='configuration'",
        [],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&value)?)
}

fn config_revision(db: &Connection) -> Result<i64, Error> {
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

fn enabled(db: &Connection) -> Result<(), Error> {
    if config(db)?.enabled {
        Ok(())
    } else {
        Err(Error::Disabled)
    }
}

fn rate(db: &Connection, category: &str, identity: &str, max: i64) -> Result<(), Error> {
    let window = now() / MINUTE;
    db.execute(
        "DELETE FROM rate_limits WHERE window < ?1",
        [window.saturating_sub(1)],
    )?;
    db.execute("INSERT INTO rate_limits(category,identity,window,count) VALUES (?1,?2,?3,1) ON CONFLICT(category,identity) DO UPDATE SET count=CASE WHEN window=excluded.window THEN count+1 ELSE 1 END, window=excluded.window", params![category, hash_secret(identity), window])?;
    let count: i64 = db.query_row(
        "SELECT count FROM rate_limits WHERE category=?1 AND identity=?2",
        params![category, hash_secret(identity)],
        |row| row.get(0),
    )?;
    if count > max {
        Err(Error::RateLimited)
    } else {
        Ok(())
    }
}

fn valid_secret(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn validate_configuration(value: &Configuration) -> Result<(), Error> {
    value.limits.validate()?;
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
    } else if value.enabled {
        return Err(Error::Invalid(
            "an advertised origin is required before enabling collaboration",
        ));
    }
    Ok(())
}

impl CollaborationService {
    pub fn open(directory: &Path) -> Result<Self, Error> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(directory)?;
        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.is_dir() {
            return Err(Error::Invalid(
                "collaboration directory must not be a symlink",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(Error::Invalid("collaboration directory must be private"));
            }
        }
        let lock_path = directory.join("host.lock");
        if let Ok(metadata) = fs::symlink_metadata(&lock_path) {
            if !metadata.is_file() {
                return Err(Error::Invalid(
                    "collaboration ownership lock must be a regular file",
                ));
            }
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let ownership = options.open(lock_path)?;
        fs2::FileExt::try_lock_exclusive(&ownership).map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                Error::HostInUse
            } else {
                Error::Io(error)
            }
        })?;
        let path = directory.join("collaboration.db");
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if !metadata.is_file() {
                return Err(Error::Invalid(
                    "collaboration database must be a regular file",
                ));
            }
        }
        let db = Connection::open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        db.busy_timeout(Duration::from_secs(2))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            return Err(Error::Invalid("unsupported collaboration database version"));
        }
        db.execute_batch(include_str!("schema.sql"))?;
        db.execute_batch("PRAGMA user_version=1;")?;
        prune_audit(&db)?;
        db.execute(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES ('host-id',?1)",
            [Uuid::new_v4().to_string()],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES ('configuration',?1)",
            [serde_json::to_string(&Configuration::default())?],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO metadata(key,value) VALUES ('configuration-revision','1')",
            [],
        )?;
        // An interrupted exchange cannot collect credentials after a new host
        // process starts. Approved long-lived sessions remain independently valid.
        db.execute(
            "UPDATE pairing_requests SET state='expired' WHERE state IN ('pending','approved')",
            [],
        )?;
        let host_id = db.query_row(
            "SELECT value FROM metadata WHERE key='host-id'",
            [],
            |row| row.get(0),
        )?;
        Ok(Self {
            db: Mutex::new(db),
            host_id,
            instance_id: Uuid::new_v4().to_string(),
            authorizations: Mutex::new(BTreeMap::new()),
            gateway: Mutex::new(GatewayStatus::default()),
            _ownership: ownership,
        })
    }

    fn db(&self) -> Result<MutexGuard<'_, Connection>, Error> {
        self.db
            .lock()
            .map_err(|_| Error::Invalid("collaboration storage lock is unavailable"))
    }

    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn status(&self) -> Result<Status, Error> {
        let db = self.db()?;
        Ok(Status {
            host_id: self.host_id.clone(), host_instance_id: self.instance_id.clone(), configuration: config(&db)?,
            configuration_revision: config_revision(&db)?.to_string(),
            active_sessions: db.query_row("SELECT count(*) FROM sessions s JOIN principals p ON p.id=s.principal_id WHERE s.revoked=0 AND p.revoked=0 AND s.absolute_expires>?1 AND s.last_activity>?2", params![now(), now().saturating_sub(IDLE_TTL)], |row| row.get(0))?,
            pending_requests: db.query_row("SELECT count(*) FROM pairing_requests WHERE state='pending' AND expires_at>?1", [now()], |row| row.get(0))?,
            gateway:self.gateway_status()?,
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

    pub fn create_invitation(&self, input: InviteCreate) -> Result<InviteSecret, Error> {
        input.grant.validate()?;
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "invite", "owner", 10)?;
        let active: i64 = db.query_row(
            "SELECT count(*) FROM invitations WHERE state='created' AND expires_at>?1",
            [now()],
            |row| row.get(0),
        )?;
        if active >= 32 {
            return Err(Error::RateLimited);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let invite_id = Uuid::new_v4().to_string();
        let token = secret()?;
        let expires_at = now() + INVITE_TTL;
        tx.execute("INSERT INTO invitations(id,verifier,grant_json,created_at,expires_at,state) VALUES (?1,?2,?3,?4,?5,'created')", params![invite_id, hash_secret(&token), serde_json::to_string(&input.grant)?, now(), expires_at])?;
        audit(&tx, "local-owner", "invite.create", &invite_id, "succeeded")?;
        tx.commit()?;
        Ok(InviteSecret {
            invite_id,
            secret: token,
            expires_at,
        })
    }

    pub fn invitations(&self, input: ListQuery) -> Result<ListPage<Invitation>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt = db.prepare("SELECT id,grant_json,expires_at,CASE WHEN state='created' AND expires_at<=?1 THEN 'expired' ELSE state END FROM invitations WHERE id>?2 ORDER BY id LIMIT ?3")?;
        let result = page(
            stmt.query_map(params![now(), after, (limit + 1) as i64], |row| {
                let id: String = row.get(0)?;
                Ok((
                    id.clone(),
                    Invitation {
                        invite_id: id,
                        grant: grant(row, 1)?,
                        expires_at: row.get(2)?,
                        state: row.get(3)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn revoke_invitation(&self, id: &str) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE invitations SET state='revoked' WHERE id=?1", [id])? == 0 {
            return Err(Error::NotFound);
        }
        tx.execute("UPDATE pairing_requests SET state='revoked' WHERE invite_id=?1 AND state IN ('pending','approved')", [id])?;
        audit(&tx, "local-owner", "invite.revoke", id, "succeeded")?;
        tx.commit()?;
        Ok(())
    }

    /// source is supplied by the trusted network boundary, never by the JSON
    /// payload. Limits persist even when the following exchange is rejected.
    pub fn redeem(&self, input: Redeem, source: &str) -> Result<PairingRequest, Error> {
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "redeem-global", "host", 60)?;
        rate(&db, "redeem-source", source, 5)?;
        if input.client_label.trim().is_empty()
            || input.client_label.len() > 80
            || input.client_label.chars().any(char::is_control)
            || !valid_secret(&input.secret)
            || !valid_secret(&input.exchange_secret_hash)
        {
            return Err(Error::PairingExpired);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count:i64 = tx.query_row("SELECT count(*) FROM pairing_requests WHERE state IN ('pending','approved') AND expires_at>?1",[now()],|row|row.get(0))?;
        if count >= 32 {
            return Err(Error::RateLimited);
        }
        let invitation:Option<String> = tx.query_row("SELECT grant_json FROM invitations WHERE id=?1 AND verifier=?2 AND state='created' AND expires_at>?3",params![input.invite_id,hash_secret(&input.secret),now()],|row|row.get(0)).optional()?;
        let Some(grant_json) = invitation else {
            return Err(Error::PairingExpired);
        };
        tx.execute(
            "UPDATE invitations SET state='redeemed' WHERE id=?1",
            [&input.invite_id],
        )?;
        let request = PairingRequest {
            request_id: Uuid::new_v4().to_string(),
            client_label: input.client_label.trim().into(),
            client_kind: input.client_kind,
            grant: serde_json::from_str(&grant_json)?,
            state: "pending".into(),
            expires_at: now() + INVITE_TTL,
        };
        tx.execute("INSERT INTO pairing_requests(id,invite_id,exchange_hash,label,kind,grant_json,state,expires_at) VALUES (?1,?2,?3,?4,?5,?6,'pending',?7)",params![request.request_id,input.invite_id,input.exchange_secret_hash,request.client_label,kind(request.client_kind),grant_json,request.expires_at])?;
        audit(
            &tx,
            "pending-controller",
            "pair.redeem",
            &request.request_id,
            "pending",
        )?;
        tx.commit()?;
        Ok(request)
    }

    pub fn requests(&self, input: ListQuery) -> Result<ListPage<PairingRequest>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT id,label,kind,grant_json,state,expires_at FROM pairing_requests WHERE state='pending' AND expires_at>?1 AND id>?2 ORDER BY id LIMIT ?3")?;
        let result = page(
            stmt.query_map(params![now(), after, (limit + 1) as i64], |row| {
                let id: String = row.get(0)?;
                Ok((
                    id.clone(),
                    PairingRequest {
                        request_id: id,
                        client_label: row.get(1)?,
                        client_kind: parse_kind(&row.get::<_, String>(2)?)?,
                        grant: grant(row, 3)?,
                        state: row.get(4)?,
                        expires_at: row.get(5)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn decide_request(&self, id: &str, approve: bool) -> Result<(), Error> {
        let mut db = self.db()?;
        enabled(&db)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,String,String)> = tx.query_row("SELECT label,kind,grant_json FROM pairing_requests WHERE id=?1 AND state='pending' AND expires_at>?2",params![id,now()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
        let Some((label, client_kind, grant_json)) = row else {
            return Err(Error::PairingExpired);
        };
        let principal_id = Uuid::new_v4().to_string();
        if approve {
            let count: i64 =
                tx.query_row("SELECT count(*) FROM principals WHERE revoked=0", [], |r| {
                    r.get(0)
                })?;
            if count >= 256 {
                return Err(Error::RateLimited);
            }
            tx.execute(
                "INSERT INTO principals(id,label,kind,grant_json) VALUES (?1,?2,?3,?4)",
                params![principal_id, label, client_kind, grant_json],
            )?;
        }
        tx.execute(
            "UPDATE pairing_requests SET state=?2,principal_id=?3 WHERE id=?1",
            params![
                id,
                if approve { "approved" } else { "rejected" },
                approve.then_some(principal_id)
            ],
        )?;
        audit(
            &tx,
            "local-owner",
            if approve {
                "pair.approve"
            } else {
                "pair.reject"
            },
            id,
            "succeeded",
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn collect(&self, input: Collect, source: &str) -> Result<Credentials, Error> {
        self.collect_inner(input, source, None)
    }

    /// The network boundary chooses the authentication mode, not clientKind.
    pub fn collect_for(
        &self,
        input: Collect,
        source: &str,
        browser: bool,
    ) -> Result<Credentials, Error> {
        self.collect_inner(input, source, Some(browser))
    }

    fn collect_inner(
        &self,
        input: Collect,
        source: &str,
        browser: Option<bool>,
    ) -> Result<Credentials, Error> {
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "collect-global", "host", 600)?;
        rate(&db, "collect-source", source, 60)?;
        if !valid_secret(&input.exchange_secret) {
            return Err(Error::PairingExpired);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,Option<String>,String,i64)> = tx.query_row("SELECT state,principal_id,kind,expires_at FROM pairing_requests WHERE id=?1 AND exchange_hash=?2",params![input.request_id,hash_secret(&input.exchange_secret)],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional()?;
        let Some((state, principal, client_kind, expires)) = row else {
            return Err(Error::PairingExpired);
        };
        let client_kind = parse_kind(&client_kind)?;
        if browser.is_some_and(|browser| (client_kind == ClientKind::Browser) != browser) {
            return Err(Error::PairingExpired);
        }
        if expires <= now() {
            return Err(Error::PairingExpired);
        }
        match state.as_str() {
            "pending" => return Err(Error::PairingPending),
            "rejected" => return Err(Error::PairingRejected),
            "approved" => {}
            _ => return Err(Error::PairingExpired),
        }
        let principal = principal.ok_or(Error::Unauthorized)?;
        let active: bool = tx.query_row(
            "SELECT revoked=0 FROM principals WHERE id=?1",
            [&principal],
            |r| r.get(0),
        )?;
        if !active {
            return Err(Error::Unauthorized);
        }
        let credentials = issue(&tx, &principal, client_kind)?;
        tx.execute(
            "UPDATE pairing_requests SET state='collected',exchange_hash='' WHERE id=?1",
            [&input.request_id],
        )?;
        audit(
            &tx,
            &principal,
            "session.create",
            &credentials.session_id,
            "succeeded",
        )?;
        tx.commit()?;
        Ok(credentials)
    }

    pub fn authenticate(
        &self,
        token: &str,
        browser: bool,
        touch: bool,
    ) -> Result<Authenticated, Error> {
        let db = self.db()?;
        authenticate_db(&db, token, browser, touch)
    }

    /// Authenticate on the blocking pool, then retain a shared revocable
    /// view for authorization/event delivery without per-event disk reads.
    pub fn authenticate_live(
        &self,
        token: &str,
        browser: bool,
        touch: bool,
    ) -> Result<Authorization, Error> {
        let db = self.db()?;
        let identity = authenticate_db(&db, token, browser, touch)?;
        let verifier = hash_secret(token);
        let mut authorizations = self
            .authorizations
            .lock()
            .map_err(|_| Error::Unauthorized)?;
        authorizations.retain(|_, state| state.strong_count() > 0);
        if let Some(state) = authorizations.get(&verifier).and_then(Weak::upgrade) {
            if state.revoked.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(Error::Unauthorized);
            }
            *state.identity.write().map_err(|_| Error::Unauthorized)? = identity;
            return Ok(Authorization { state });
        }
        let authorization = Authorization::new(identity);
        authorizations.insert(verifier, Arc::downgrade(&authorization.state));
        Ok(authorization)
    }

    fn update_authorizations(
        &self,
        update: impl Fn(&crate::authority::AuthorizationState),
    ) -> Result<(), Error> {
        let mut views = match self.authorizations.lock() {
            Ok(views) => views,
            Err(poisoned) => {
                for view in poisoned.into_inner().values().filter_map(Weak::upgrade) {
                    view.revoke();
                }
                return Err(Error::Unauthorized);
            }
        };
        views.retain(|_, view| {
            if let Some(view) = view.upgrade() {
                update(&view);
                true
            } else {
                false
            }
        });
        Ok(())
    }

    fn invalidate_session(&self, id: &str) -> Result<(), Error> {
        self.update_authorizations(|state| match state.identity.read() {
            Ok(identity) if identity.session_id == id => state.revoke(),
            Err(_) => state.revoke(),
            _ => {}
        })
    }

    pub fn refresh(&self, token: &str) -> Result<Credentials, Error> {
        if !valid_secret(token) {
            return Err(Error::Unauthorized);
        }
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "refresh-global", "host", 600)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let verifier = hash_secret(token);
        let used: Option<String> = tx
            .query_row(
                "SELECT session_id FROM used_refresh WHERE verifier=?1",
                [&verifier],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = used {
            tx.execute("UPDATE sessions SET revoked=1 WHERE id=?1", [&id])?;
            audit(
                &tx,
                "session-holder",
                "session.refresh-reuse",
                &id,
                "revoked",
            )?;
            tx.commit()?;
            self.invalidate_session(&id)?;
            return Err(Error::Unauthorized);
        }
        let row:Option<(String,String,i64)>=tx.query_row("SELECT s.id,s.principal_id,s.absolute_expires FROM sessions s JOIN principals p ON p.id=s.principal_id WHERE s.refresh_hash=?1 AND s.revoked=0 AND p.revoked=0 AND s.kind!='browser' AND s.absolute_expires>?2 AND s.last_activity>?3",params![verifier,now(),now().saturating_sub(IDLE_TTL)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let Some((session_id, principal_id, expires_at)) = row else {
            return Err(Error::Unauthorized);
        };
        let access = secret()?;
        let refresh = secret()?;
        let access_expires_at = (now() + ACCESS_TTL).min(expires_at);
        tx.execute(
            "INSERT INTO used_refresh(verifier,session_id) VALUES (?1,?2)",
            params![verifier, session_id],
        )?;
        tx.execute("UPDATE sessions SET access_hash=?2,access_expires=?3,refresh_hash=?4,last_activity=?5 WHERE id=?1",params![session_id,hash_secret(&access),access_expires_at,hash_secret(&refresh),now()])?;
        audit(
            &tx,
            &principal_id,
            "session.refresh",
            &session_id,
            "succeeded",
        )?;
        tx.commit()?;
        self.invalidate_session(&session_id)?;
        Ok(Credentials {
            principal_id,
            session_id,
            access_token: access,
            access_expires_at,
            refresh_token: Some(refresh),
            expires_at,
        })
    }

    pub fn sessions(&self, input: ListQuery) -> Result<ListPage<Session>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT s.id,s.principal_id,s.kind,s.created_at,s.last_activity,s.absolute_expires,s.revoked,COALESCE(p.label,'') FROM sessions s LEFT JOIN principals p ON p.id=s.principal_id WHERE s.id>?1 ORDER BY s.id LIMIT ?2")?;
        let result = page(
            stmt.query_map(params![after, (limit + 1) as i64], |r| {
                let id: String = r.get(0)?;
                Ok((
                    id.clone(),
                    Session {
                        session_id: id,
                        principal_id: r.get(1)?,
                        client_label: r.get(7)?,
                        client_kind: parse_kind(&r.get::<_, String>(2)?)?,
                        created_at: r.get(3)?,
                        last_activity_at: r.get(4)?,
                        expires_at: r.get(5)?,
                        revoked: r.get(6)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn revoke_session(&self, id: &str, actor: &str) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE sessions SET revoked=1 WHERE id=?1", [id])? == 0 {
            return Err(Error::NotFound);
        }
        audit(&tx, actor, "session.revoke", id, "succeeded")?;
        tx.commit()?;
        self.invalidate_session(id)?;
        Ok(())
    }

    pub fn principals(&self, input: ListQuery) -> Result<ListPage<Principal>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT id,label,kind,grant_json,revision,revoked FROM principals WHERE id>?1 ORDER BY id LIMIT ?2")?;
        let result = page(
            stmt.query_map(params![after, (limit + 1) as i64], |r| {
                let id: String = r.get(0)?;
                Ok((
                    id.clone(),
                    Principal {
                        principal_id: id,
                        client_label: r.get(1)?,
                        client_kind: parse_kind(&r.get::<_, String>(2)?)?,
                        grant: grant(r, 3)?,
                        grant_revision: r.get::<_, i64>(4)?.to_string(),
                        revoked: r.get(5)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn principal(&self, id: &str) -> Result<Principal, Error> {
        if uuid::Uuid::parse_str(id).map_or(true, |parsed| parsed.to_string() != id) {
            return Err(Error::Invalid("invalid principal ID"));
        }
        let db = self.db()?;
        let length: Option<i64> = db
            .query_row(
                "SELECT length(CAST(grant_json AS BLOB)) FROM principals WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        let length = length.ok_or(Error::NotFound)?;
        // Reserve space for the label, identity and JSON-RPC envelope before
        // parsing a stored grant. The service's DB lock spans both reads.
        if length > 512 * 1024 - 2048 {
            return Err(Error::TooLarge);
        }
        db.query_row(
            "SELECT id,label,kind,grant_json,revision,revoked FROM principals WHERE id=?1",
            [id],
            |r| {
                Ok(Principal {
                    principal_id: r.get(0)?,
                    client_label: r.get(1)?,
                    client_kind: parse_kind(&r.get::<_, String>(2)?)?,
                    grant: grant(r, 3)?,
                    grant_revision: r.get::<_, i64>(4)?.to_string(),
                    revoked: r.get(5)?,
                })
            },
        )
        .map_err(Error::from)
    }

    pub fn revoke_principal(&self, id: &str) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE principals SET revoked=1,revision=revision+1 WHERE id=?1",
            [id],
        )? == 0
        {
            return Err(Error::NotFound);
        }
        tx.execute("UPDATE sessions SET revoked=1 WHERE principal_id=?1", [id])?;
        tx.execute("UPDATE pairing_requests SET state='revoked' WHERE principal_id=?1 AND state='approved'",[id])?;
        audit(&tx, "local-owner", "principal.revoke", id, "succeeded")?;
        tx.commit()?;
        self.update_authorizations(|state| match state.identity.read() {
            Ok(identity) if identity.principal_id == id => state.revoke(),
            Err(_) => state.revoke(),
            _ => {}
        })?;
        Ok(())
    }

    pub fn set_grant(&self, input: GrantSet) -> Result<(), Error> {
        input.grant.validate()?;
        let revision: i64 = input
            .expected_revision
            .parse()
            .map_err(|_| Error::Invalid("invalid grant revision"))?;
        if revision <= 0 || revision == i64::MAX {
            return Err(Error::Invalid("invalid grant revision"));
        }
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE principals SET grant_json=?2,revision=revision+1 WHERE id=?1 AND revision=?3 AND revoked=0",params![input.principal_id,serde_json::to_string(&input.grant)?,revision])?==0{return Err(Error::RevisionConflict);}
        audit(
            &tx,
            "local-owner",
            "grant.set",
            &input.principal_id,
            "succeeded",
        )?;
        tx.commit()?;
        self.update_authorizations(|state| {
            state.update_grant(
                &input.principal_id,
                &input.grant,
                &(revision + 1).to_string(),
            )
        })?;
        Ok(())
    }

    /// Idempotent import of closed metadata from an owning database's outbox.
    /// The source acknowledges only after this transaction and publication.
    pub fn import_audit(&self, records: &[AuditRecord]) -> Result<usize, Error> {
        if records.len() > 100 {
            return Err(Error::Invalid("audit import batch exceeds limit"));
        }
        let mut encoded = Vec::with_capacity(records.len());
        let mut bytes = 0usize;
        for record in records {
            record.validate()?;
            let json = serde_json::to_string(record)?;
            bytes = bytes.saturating_add(json.len());
            if bytes > 256 * 1024 {
                return Err(Error::Invalid("audit import exceeds byte limit"));
            }
            let hash = hash_secret(&json);
            encoded.push((record, json, hash));
        }
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        prune_audit(&tx)?;
        let cutoff = now().saturating_sub(30 * IDLE_TTL);
        let mut inserted = 0;
        for (record, json, hash) in encoded {
            if record.timestamp < cutoff {
                continue;
            }
            let previous: Option<String> = tx
                .query_row(
                    "SELECT payload_hash FROM audit_imports WHERE event_id=?1",
                    [&record.event_id],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(previous) = previous {
                if previous != hash {
                    return Err(Error::Invalid("audit event ID was reused"));
                }
                continue;
            }
            tx.execute("INSERT INTO audit(timestamp,principal_id,action,resource_id,outcome) VALUES (?1,?2,?3,?4,?5)",params![record.timestamp,record.principal_id,record.method,record.resource.id,record.result_code])?;
            let cursor = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO audit_details(cursor,details_json) VALUES (?1,?2)",
                params![cursor, json],
            )?;
            tx.execute("INSERT INTO audit_imports(event_id,payload_hash,source_timestamp) VALUES (?1,?2,?3)",params![record.event_id,hash,record.timestamp])?;
            inserted += 1;
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// The host calls this periodically only for an already-open store.
    pub fn prune_audit(&self) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        prune_audit(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn audit(&self, input: AuditQuery) -> Result<Vec<AuditEntry>, Error> {
        if input.after < 0 {
            return Err(Error::Invalid("invalid audit cursor"));
        }
        let limit = input.limit.unwrap_or(50);
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("audit page limit must be between 1 and 100"));
        }
        self.prune_audit()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT a.cursor,a.timestamp,a.principal_id,a.action,a.resource_id,a.outcome,d.details_json FROM audit a LEFT JOIN audit_details d ON d.cursor=a.cursor WHERE a.cursor>?1 ORDER BY a.cursor LIMIT ?2")?;
        let rows = stmt
            .query_map(params![input.after, limit as i64], |r| {
                Ok(AuditEntry {
                    cursor: r.get(0)?,
                    timestamp: r.get(1)?,
                    principal_id: r.get(2)?,
                    action: r.get(3)?,
                    resource_id: r.get(4)?,
                    outcome: r.get(5)?,
                    details: r
                        .get::<_, Option<String>>(6)?
                        .map(|json| {
                            serde_json::from_str(&json).map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    6,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })
                        })
                        .transpose()?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

impl Drop for CollaborationService {
    fn drop(&mut self) {
        let views = self
            .authorizations
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for view in views.values().filter_map(Weak::upgrade) {
            view.revoke();
        }
    }
}

fn authenticate_db(
    db: &Connection,
    token: &str,
    browser: bool,
    touch: bool,
) -> Result<Authenticated, Error> {
    if !valid_secret(token) {
        return Err(Error::Unauthorized);
    }
    enabled(db)?;
    let result:Option<(Authenticated,i64)>=db.query_row("SELECT p.id,s.id,p.grant_json,p.revision,s.kind,s.access_expires,s.last_activity FROM sessions s JOIN principals p ON p.id=s.principal_id WHERE s.access_hash=?1 AND s.revoked=0 AND p.revoked=0 AND s.access_expires>?2 AND s.absolute_expires>?2 AND s.last_activity>?3",params![hash_secret(token),now(),now().saturating_sub(IDLE_TTL)],|r|Ok((Authenticated {principal_id:r.get(0)?,session_id:r.get(1)?,grant:grant(r,2)?,grant_revision:r.get::<_,i64>(3)?.to_string(),client_kind:parse_kind(&r.get::<_,String>(4)?)?,access_expires_at:r.get(5)?},r.get(6)?))).optional()?;
    let (mut identity, last_activity) = result.ok_or(Error::Unauthorized)?;
    if (identity.client_kind == ClientKind::Browser) != browser {
        return Err(Error::Unauthorized);
    }
    let last_activity = if touch {
        let at = now();
        db.execute(
            "UPDATE sessions SET last_activity=?2 WHERE id=?1",
            params![identity.session_id, at],
        )?;
        at
    } else {
        last_activity
    };
    identity.access_expires_at = identity.access_expires_at.min(last_activity + IDLE_TTL);
    Ok(identity)
}

fn issue(
    tx: &Transaction<'_>,
    principal_id: &str,
    client_kind: ClientKind,
) -> Result<Credentials, Error> {
    let session_id = Uuid::new_v4().to_string();
    let access_token = secret()?;
    let refresh_token = if client_kind == ClientKind::Browser {
        None
    } else {
        Some(secret()?)
    };
    let expires_at = now() + SESSION_TTL;
    let access_expires_at = if client_kind == ClientKind::Browser {
        expires_at
    } else {
        now() + ACCESS_TTL
    };
    tx.execute("INSERT INTO sessions(id,principal_id,kind,access_hash,access_expires,refresh_hash,created_at,last_activity,absolute_expires) VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8)",params![session_id,principal_id,kind(client_kind),hash_secret(&access_token),access_expires_at,refresh_token.as_deref().map(hash_secret),now(),expires_at])?;
    Ok(Credentials {
        principal_id: principal_id.into(),
        session_id,
        access_token,
        access_expires_at,
        refresh_token,
        expires_at,
    })
}

#[cfg(test)]
mod tests;

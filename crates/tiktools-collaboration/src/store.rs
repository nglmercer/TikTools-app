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
mod audit;
mod authentication;
mod bootstrap;
mod configuration;
mod execution;
mod invitations;
mod pairing;
mod principals;
mod rows;
mod sessions;

use audit::{audit, prune_audit};
use configuration::{config_revision, enabled};
use rows::{grant, kind, page, parse_kind};
use sessions::issue;

const MINUTE: i64 = 60_000;
const INVITE_TTL: i64 = 5 * MINUTE;
const ACCESS_TTL: i64 = 15 * MINUTE;
const IDLE_TTL: i64 = 24 * 60 * MINUTE;
const SESSION_TTL: i64 = 7 * IDLE_TTL;
/// Browsers have no refresh token, so authenticated activity renews their
/// session in place. This ceiling bounds that renewal from the pairing time.
const BROWSER_SESSION_MAX: i64 = 30 * IDLE_TTL;

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

impl CollaborationService {
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

#[cfg(test)]
mod tests;

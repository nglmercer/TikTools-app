//! Optional loopback browser transport. Owns no AppCore and connects only via IPC.
//! Authentication, pairing readout, assets and sockets share one adapter state.
mod assets;
mod auth;
mod options;
mod pairing;
mod readout;
mod server;
mod socket;

use assets::asset;
#[cfg(all(test, unix))]
use assets::resolve_plugin_asset;
use auth::{secret, session_touch, token_matches};
pub use options::run;
use options::{join_link, multi_session_enabled};
#[cfg(test)]
use options::{parse_action_options, parse_options, parse_print_code_options};
#[cfg(test)]
use pairing::{fixture_code_from, pairing_code};
use pairing::{mint_pairing, pair, renew_pairing, reset_session};
use readout::{
    current_pairing_code, current_reset_token, pairing_file, publish_pairing_code,
    publish_reset_token, reset_file,
};
#[cfg(test)]
use readout::{
    current_pairing_code_at, load_pairing_snapshot, now_unix, snapshot_remaining,
    write_pairing_file,
};

use server::start;
#[cfg(test)]
use socket::bounded_owner_message;
use socket::upgrade;

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rand::RngExt;
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tiktools_control_api::{
    owner_request_bytes, ControlClient, ControlEvent, MAX_OWNER_LIBRARY_REQUEST_BYTES,
    MAX_OWNER_RESPONSE_BYTES,
};
use tokio::{sync::Semaphore, task::JoinSet};

const PAIRING_RENEW_COOLDOWN: Duration = Duration::from_secs(30);
/// Failed pair attempts before the pending code locks out. Shared by `pair`
/// and `renew_pairing`, which enforce it on the same counter.
const PAIRING_MAX_ATTEMPTS: u8 = 8;
/// Sliding idle window. Authenticated activity on the socket or session touch
/// endpoint renews it, so an open tab is never logged out for being open.
const SESSION_LIFETIME: Duration = Duration::from_secs(3600);
/// Hard ceiling from pairing. Activity cannot renew a browser session forever.
const SESSION_ABSOLUTE_LIFETIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// The cookie outlives the idle window so activity can renew it without a
/// re-pair. The server still enforces both windows above.
const SESSION_COOKIE_MAX_AGE: u64 = 7 * 24 * 60 * 60;
#[cfg(test)]
const MAX_FRAME: usize = 1024 * 1024;
/// Lifetime of a pairing code, in seconds. Bounds the in-memory check in
/// `pair` and the runtime-file readout the same way.
const PAIRING_TTL_SECS: u64 = 300;
const PAIRING_TTL: Duration = Duration::from_secs(PAIRING_TTL_SECS);
/// Deterministic pairing override for automated login tests. Never set in
/// production: anyone holding the value can pair without the terminal.
const PAIRING_FIXTURE_ENV: &str = "TIKTOOLS_PAIRING_FIXTURE";
/// Opt-in: several browsers may hold a session at once. Off by default so a
/// paired adapter keeps the single-owner behaviour of the loopback transport.
const MULTI_SESSION_ENV: &str = "TIKTOOLS_WEB_MULTI_SESSION";
const MAX_SESSIONS: usize = 8;
/// Loopback port the adapter binds unless `--port` overrides it. Shared by the
/// server parser and the offline actions so they always look in the same place.
const DEFAULT_PORT: u16 = 8787;
/// Recovery route, shared between the server router and the raw TCP request
/// the `--reset-session` action writes by hand.
const SESSION_RESET_PATH: &str = "/__local/session/reset";

/// One paired browser. The idle window slides with activity; the absolute
/// ceiling is fixed at pairing time and cannot be extended.
struct Session {
    token: String,
    last: Instant,
    paired_at: Instant,
}

struct Auth {
    pairing: Option<String>,
    issued: Instant,
    sessions: Vec<Session>,
    attempts: u8,
    renewed_at: Option<Instant>,
}

struct App {
    root: PathBuf,
    asset_token: String,
    origin: String,
    authority: String,
    join_base: String,
    pairing_file: PathBuf,
    /// Local-only recovery credential: a separate CLI reads this 0600 file to
    /// invalidate every session and mint a fresh code without a restart.
    reset_file: PathBuf,
    reset_token: Mutex<String>,
    multi_session: bool,
    auth: Mutex<Auth>,
    clients: Arc<Semaphore>,
    stopping: tokio::sync::watch::Sender<bool>,
}

/// Readout of the active code for `web --print-code`, so agents and operators
/// can recover a missed terminal print without restarting the adapter. The
/// file is a readout only: reading never extends, consumes, or retries the
/// code, and the 5-minute TTL is enforced by the reader.
#[derive(serde::Serialize, serde::Deserialize)]
struct PairingSnapshot {
    code: String,
    issued_at_unix: u64,
    expires_in_secs: u64,
    pid: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ResetSnapshot {
    token: String,
    pid: u32,
}

async fn pair_script() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], PAIR_SCRIPT)
}

const PAIR_PAGE: &str = include_str!("web/generated/pairing.html");
const PAIR_SCRIPT: &str = include_str!("web/generated/pair.js");

#[cfg(test)]
mod tests;

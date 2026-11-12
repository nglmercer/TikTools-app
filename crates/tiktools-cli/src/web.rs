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
#[cfg(test)]
use assets::resolve_plugin_asset;
use auth::{secret, session_touch};
use options::join_link;
pub use options::run;
#[cfg(test)]
use options::{parse_options, parse_print_code_options};
#[cfg(test)]
use pairing::{fixture_code_from, pairing_code};
use pairing::{mint_pairing, pair, renew_pairing};
use readout::{current_pairing_code, pairing_file, publish_pairing_code};
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
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Request, State,
    },
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
/// Deterministic pairing override for automated login tests. Never set in
/// production: anyone holding the value can pair without the terminal.
const PAIRING_FIXTURE_ENV: &str = "TIKTOOLS_PAIRING_FIXTURE";

struct Auth {
    pairing: Option<String>,
    issued: Instant,
    /// Session token paired with the last authenticated activity that keeps it
    /// alive. Activity slides this window; it never extends the ceiling below.
    session: Option<(String, Instant)>,
    /// Pairing time. Bounds how long activity may keep renewing the session.
    paired_at: Option<Instant>,
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

async fn pair_script() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], PAIR_SCRIPT)
}

const PAIR_PAGE: &str = include_str!("web/pairing.html");
const PAIR_SCRIPT: &str = include_str!("web/pairing.js");

#[cfg(test)]
mod tests;

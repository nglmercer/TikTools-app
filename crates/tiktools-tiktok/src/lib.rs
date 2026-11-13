//! Native TikTok LIVE client used by the Rust application host.
//!
//! This crate deliberately knows nothing about Winit, Wry, SQLite, or the
//! frontend. It composes the browser-free transport from the pinned
//! `tiktok-signer` workspace:
//!
//! * `ttl-live-discovery` resolves rooms and reads room/gift metadata;
//! * `ttl-sign-embedded` signs the direct socket query in-process;
//! * `ttl-sign-headless` builds the transport backend;
//! * `ttl-live-ws` owns socket framing, heartbeats, and acknowledgements; and
//! * `ttl-live-events` normalises protobuf payloads into stable event values.
//!
//! Bun, Node, and a browser are not part of the live connection path.

use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod adapter;
mod client;
use client::socket::{open_socket, ManagedConnection};
mod lifecycle;
mod loss_tracking;
mod retry;
pub use lifecycle::{ConnectionSnapshot, ConnectionState, StampedClientEvent};
pub use loss_tracking::{LostGeneration, TrackedReceiveError, TrackedReceiver};

pub mod discovery;
pub mod events;
pub mod live;
pub mod protocol;

pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub unique_id: String,
    pub session_cookie: String,
    #[serde(default)]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub room_id: Option<String>,
    pub title: Option<String>,
    pub creator: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub unique_id: String,
    /// Native-only evidence that discovery tied this creator to this room.
    /// RPC/JSON input cannot supply this authority.
    #[serde(skip)]
    pub verified_creator: Option<String>,
    pub room_id: String,
    pub title: String,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub viewers: u64,
    pub total_users: u64,
    pub gifts: Vec<GiftInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GiftInfo {
    pub id: String,
    pub name: String,
    pub diamond_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip)]
    pub streakable: bool,
}

#[derive(Debug, Clone)]
pub struct NativeTikTokConfig {
    /// Explicit bundle override. The signer bundle is intentionally loaded
    /// at runtime so a TikTok deployment can be updated without recompiling
    /// the desktop host.
    pub bundle_path: Option<PathBuf>,
    /// Cache location used when the bundle is downloaded from `bundle_url`.
    pub bundle_cache_path: Option<PathBuf>,
    pub bundle_url: String,
    /// Application retry budget for transient network failures. Rate limits
    /// use at most two retries and a separate conservative cooldown.
    pub reconnect: ttl_live_ws::ReconnectPolicy,
}

#[derive(Debug, Error)]
pub enum TikTokError {
    #[error("TikTok creator handle is empty")]
    InvalidCreator,
    #[error("TikTok session is empty and guest bootstrap did not provide an identity")]
    EmptySession,
    #[error("TikTok discovery failed: {0}")]
    Discovery(String),
    #[error("TikTok rate limit (HTTP 429); wait before reconnecting")]
    RateLimited { retry_after: Option<Duration> },
    #[error("TikTok temporary transport failure: {0}")]
    RetryableTransport(String),
    #[error("TikTok signing bundle failed: {0}")]
    Bundle(String),
    #[error("TikTok signer failed: {0}")]
    Signer(String),
    #[error("TikTok live transport failed: {0}")]
    Transport(String),
    #[error("TikTok client task failed: {0}")]
    Task(String),
}

/// Events emitted by the native client. The core consumes this stream and
/// decides what belongs in the UI, points engine, automation, and plugins.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected(ConnectionInfo),
    Event(events::TikToolsEvent),
    Reconnecting { attempt: u64, delay_ms: u64 },
    Disconnected { reason: String },
    Error { phase: ErrorPhase, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorPhase {
    Connect,
    Live,
}

pub trait TikTokClient: Send + Sync {
    fn connect<'a>(
        &'a self,
        request: ConnectRequest,
    ) -> BoxFuture<'a, Result<ConnectionInfo, TikTokError>>;
    fn disconnect<'a>(&'a self) -> BoxFuture<'a, Result<(), TikTokError>>;
    fn discover<'a>(
        &'a self,
        request: ConnectRequest,
    ) -> BoxFuture<'a, Result<DiscoveryResult, TikTokError>>;
}

/// The production client. All mutable socket state is owned by the Tokio
/// task; the public object is a cheap, thread-safe control handle.
pub struct NativeTikTokClient {
    config: NativeTikTokConfig,
    events: lifecycle::EventSender,
    owner: lifecycle::ConnectionOwner,
    cooldown: retry::Cooldown,
}

#[cfg(test)]
mod verified_room_tests;

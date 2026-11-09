//! Optional Cloud account for TikTools Desktop.
//!
//! The account is strictly opt-in: signing in is optional, nothing here runs
//! until the user asks for it, and every local feature keeps working offline. Login reuses the pinned
//! `cloud-client` flow (system browser, PKCE, loopback callback, one-time
//! redemption) instead of inventing a second OAuth dance.
//!
//! Token handling rules, enforced by construction:
//!
//! - Session tokens live in the OS credential store ([`OsTokenStore`]) or in
//!   process memory for the current run only. They are never written to
//!   files, logs, SQLite, or the frontend.
//! - Only nonsecret state ([`AccountStatus`]: origin, account id, verified
//!   profile, login phase, persistence health, error text) crosses into
//!   control responses and the UI.
//! - Bearer credentials are used only on native API routes inside
//!   `cloud-client`.

mod origin;

#[cfg(feature = "account")]
mod account;
#[cfg(feature = "account")]
mod state;
#[cfg(feature = "account")]
mod store;

#[cfg(all(test, feature = "account"))]
mod auth_tests;

#[cfg(feature = "account")]
pub use account::*;
pub use origin::{
    normalize_cloud_origin, resolve_cloud_origin, DEFAULT_CLOUD_ORIGIN, ENV_CLOUD_ORIGIN,
};

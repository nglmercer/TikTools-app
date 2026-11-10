//! Nonsecret account state for control responses and the UI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AccountState {
    Unconfigured,
    SignedOut,
    SigningIn,
    SignedIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Persistence {
    /// Tokens persist across restarts in the OS credential store.
    Keychain,
    /// Tokens live in process memory only; a restart signs out. This is
    /// reported, never a silent fallback to a file.
    MemoryOnly,
}

/// Everything the frontend may show. No tokens, ever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub origin: Option<String>,
    pub state: AccountState,
    /// Opaque identifier for the currently observed login attempt. It binds
    /// cancellation to that attempt and is not a bearer/session credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Verified account email from `/me`, when the sign-in resolved it.
    /// Best-effort: a signed-in status without an email is still valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Verified display name from `/me`, when the provider supplies one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub persistence: Persistence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

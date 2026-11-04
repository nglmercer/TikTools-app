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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    pub persistence: Persistence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

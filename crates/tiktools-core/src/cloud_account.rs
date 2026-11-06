//! Optional Cloud account owned by the core. Signing in is strictly opt-in and
//! every local feature keeps working offline. The origin resolves from
//! `TIKTOOLS_CLOUD_ORIGIN` (runtime, then build time) or the official default;
//! only an explicit opt-out reports unconfigured.

use std::sync::Arc;

use tiktools_cloud_auth::{CloudAuth, LoginBegin, OsTokenStore, TokenStore};

use crate::paths::AppPaths;

pub use tiktools_cloud_auth::{AccountState, AccountStatus, AuthError, Persistence};

pub(crate) struct CloudAccount {
    auth: CloudAuth,
}

impl CloudAccount {
    pub(crate) fn new(paths: &AppPaths) -> Self {
        let store: Arc<dyn TokenStore> = Arc::new(OsTokenStore::new());
        let auth = CloudAuth::from_environment(paths.data.clone(), store);
        Self { auth }
    }

    pub(crate) fn auth(&self) -> &CloudAuth {
        &self.auth
    }
}

/// Nonsecret login handoff for control responses: only the public browser
/// URL, never credentials.
pub struct CloudLoginBegin {
    pub browser_url: String,
}

impl From<LoginBegin> for CloudLoginBegin {
    fn from(begin: LoginBegin) -> Self {
        Self {
            browser_url: begin.browser_url,
        }
    }
}

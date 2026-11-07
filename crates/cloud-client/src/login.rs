//! Native side of Better Auth's loopback and single-use handoff.
use crate::{decode, parse_origin, Session, SyncError};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use better_auth::desktop::LoopbackCallbackReceiver;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;
use url::Url;
use zeroize::Zeroize;

/// In-memory only; do not log, clone, or persist the verifier and state.
pub struct LoginAttempt {
    origin: Url,
    browser_url: Url,
    redirect_uri: String,
    verifier: SecretString,
    receiver: LoopbackCallbackReceiver,
}
pub struct LoginResult {
    pub session: Session,
    pub expires_at: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Redemption<'a> {
    code: &'a str,
    redirect_uri: &'a str,
    code_verifier: &'a str,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Redeemed {
    session_token: String,
    expires_at: u64,
}

impl LoginAttempt {
    /// Generate state and PKCE from the OS RNG and bind an ephemeral loopback listener.
    /// This does not launch a browser or contact the network.
    pub async fn begin(origin: &str) -> Result<Self, SyncError> {
        let origin = parse_origin(origin)?;
        let mut state_bytes = [0_u8; 32];
        let mut verifier_bytes = [0_u8; 48];
        getrandom::fill(&mut state_bytes).map_err(|_| SyncError::Configuration)?;
        getrandom::fill(&mut verifier_bytes).map_err(|_| SyncError::Configuration)?;
        let state = URL_SAFE_NO_PAD.encode(state_bytes);
        let verifier = SecretString::from(URL_SAFE_NO_PAD.encode(verifier_bytes));
        verifier_bytes.zeroize();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.expose_secret().as_bytes()));
        let receiver = LoopbackCallbackReceiver::bind("/callback", &state)
            .await
            .map_err(|_| SyncError::Configuration)?;
        let redirect_uri = receiver.redirect_uri();
        let mut browser_url = origin
            .join("desktop/login")
            .map_err(|_| SyncError::Configuration)?;
        browser_url
            .query_pairs_mut()
            .append_pair("redirectUri", &redirect_uri)
            .append_pair("codeChallenge", &challenge)
            .append_pair("state", &state);
        Ok(Self {
            origin,
            browser_url,
            redirect_uri,
            verifier,
            receiver,
        })
    }
    /// Open this URL in the system browser, not an embedded webview. Do not log it.
    pub fn browser_url(&self) -> &Url {
        &self.browser_url
    }
    #[cfg(feature = "system-browser")]
    pub fn open_system_browser(&self) -> Result<(), SyncError> {
        open::that(self.browser_url.as_str()).map_err(|_| SyncError::Browser)
    }
    /// Wait for the validated callback and redeem the code exactly once.
    pub async fn complete(
        self,
        timeout: Duration,
        cancel: oneshot::Receiver<()>,
    ) -> Result<LoginResult, SyncError> {
        if timeout.is_zero() || timeout > Duration::from_secs(300) {
            return Err(SyncError::Configuration);
        }
        let code = self
            .receiver
            .receive(timeout, cancel)
            .await
            .map_err(|_| SyncError::LoginCancelled)?;
        let code = SecretString::from(code);
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| SyncError::Configuration)?;
        let url = self
            .origin
            .join("api/v1/native/handoff/redeem")
            .map_err(|_| SyncError::Configuration)?;
        let response = client
            .post(url)
            .json(&Redemption {
                code: code.expose_secret(),
                redirect_uri: &self.redirect_uri,
                code_verifier: self.verifier.expose_secret(),
            })
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let redeemed: Redeemed = decode(response).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SyncError::Protocol)?
            .as_secs();
        if redeemed.expires_at <= now {
            return Err(SyncError::Protocol);
        }
        let session = Session::connect(self.origin.as_str(), redeemed.session_token).await?;
        Ok(LoginResult {
            session,
            expires_at: redeemed.expires_at,
        })
    }
}

//! Native login-code HTTP calls. No bearer token is sent on these routes: the
//! reservation/nonce is the only correlation material and the code is entered
//! by the user. The bearer returned by redeem is used only to build a
//! `Session`, which immediately becomes an OS-keychain credential in the host.

use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use url::Url;
use zeroize::Zeroizing;

use crate::{decode, LoginResult, Session, SyncError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReserveResponse {
    reservation_id: String,
    device_nonce: String,
    requires_oauth: bool,
    expires_at: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelayResponse {
    delivered: usize,
    expires_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RedeemRequest<'a> {
    reservation_id: &'a str,
    device_nonce: &'a str,
    code: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RedeemResponse {
    session_token: String,
    expires_at: u64,
}

/// Public state of a login-code operation. Never contains a token or a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCodeStatus {
    pub reservation_id: String,
    pub device_nonce: String,
    pub requires_oauth: bool,
    pub expires_at: u64,
}

fn client() -> Result<reqwest::Client, SyncError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| SyncError::Configuration)
}

fn endpoint(origin: &Url, path: &str) -> Result<Url, SyncError> {
    origin.join(path).map_err(|_| SyncError::Configuration)
}

async fn post<T: for<'de> Deserialize<'de>>(
    origin: &Url,
    path: &str,
    body: &impl Serialize,
) -> Result<T, SyncError> {
    let response = client()?
        .post(endpoint(origin, path)?)
        .json(body)
        .send()
        .await
        .map_err(|_| SyncError::Network)?;
    decode(response).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReserveRequest<'a> {
    email: &'a str,
    device_nonce: &'a str,
}

/// Reserve a code login. An unknown account needs OAuth instead.
pub async fn reserve(
    origin: &Url,
    email: &str,
    device_nonce: &str,
) -> Result<LoginCodeStatus, SyncError> {
    let response: ReserveResponse = post(
        origin,
        "api/v1/native/login-code/reserve",
        &ReserveRequest {
            email,
            device_nonce,
        },
    )
    .await?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SyncError::Protocol)?
        .as_secs();
    if response.expires_at <= now || response.reservation_id.is_empty() {
        return Err(SyncError::Protocol);
    }
    Ok(LoginCodeStatus {
        reservation_id: response.reservation_id,
        device_nonce: response.device_nonce,
        requires_oauth: response.requires_oauth,
        expires_at: response.expires_at,
    })
}

/// Ask the host to relay the code to connected devices. The response never
/// contains the code: it is delivered over the remote-control channel.
pub async fn relay(
    origin: &Url,
    reservation_id: &str,
    device_nonce: &str,
) -> Result<u64, SyncError> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Body<'a> {
        reservation_id: &'a str,
        device_nonce: &'a str,
    }
    let response: RelayResponse = post(
        origin,
        "api/v1/native/login-code/relay",
        &Body {
            reservation_id,
            device_nonce,
        },
    )
    .await?;
    if response.delivered == 0 {
        return Err(SyncError::Protocol);
    }
    Ok(response.expires_at)
}

/// Exchange the relayed code for a native Bearer `Session`.
pub async fn redeem(
    origin: &Url,
    reservation_id: &str,
    device_nonce: &str,
    code: &str,
) -> Result<LoginResult, SyncError> {
    let token = Zeroizing::new(code.to_owned());
    let response: RedeemResponse = post(
        origin,
        "api/v1/native/login-code/redeem",
        &RedeemRequest {
            reservation_id,
            device_nonce,
            code: &token,
        },
    )
    .await?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SyncError::Protocol)?
        .as_secs();
    if response.expires_at <= now || response.session_token.is_empty() {
        return Err(SyncError::Protocol);
    }
    let session = Session::connect(origin.as_str(), response.session_token).await?;
    Ok(LoginResult {
        session,
        expires_at: response.expires_at,
    })
}

/// Deliberately private: the HTTP client never keeps reservation state.
#[allow(dead_code)]
fn _no_headers() -> HashMap<String, String> {
    HashMap::new()
}

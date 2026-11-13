//! `live` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::live::{LiveChannelsParams, LiveConnectParams, LivePickParams};
use tiktools_control_api::modules::live::{
    PlatformConnectParams, PlatformCredentialDelete, PlatformCredentialParams,
    PlatformDisconnectParams, PlatformId,
};

use super::args::{flag_value, split_first};
use super::{result_value, CommandError};

pub enum Command {
    Platforms,
    Connections,
    PlatformConnect(PlatformConnectParams),
    PlatformDisconnect(PlatformDisconnectParams),
    CredentialStore(PlatformCredentialParams),
    CredentialDelete(PlatformCredentialDelete),
    PlatformAnalytics,
    Connect {
        unique_id: String,
        session_cookie: String,
        room_id: Option<String>,
    },
    Pick {
        session_cookie: String,
    },
    Channels {
        keyword: Option<String>,
        offset: Option<u32>,
        limit: Option<u32>,
        session_cookie: String,
    },
    Disconnect,
    Status,
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "live <verb> ...")?;
    match verb {
        "platforms" => Ok(Command::Platforms),
        "connections" => Ok(Command::Connections),
        "platform-analytics" => Ok(Command::PlatformAnalytics),
        "platform-connect" => Ok(Command::PlatformConnect(PlatformConnectParams {
            platform: platform(rest)?,
            target: required_flag(rest, "--target")?,
            room_id: flag_value(rest, "--room-id"),
            credential_ref: flag_value(rest, "--credential-ref"),
        })),
        "platform-disconnect" => Ok(Command::PlatformDisconnect(PlatformDisconnectParams {
            connection_id: required_flag(rest, "--connection-id")?,
        })),
        "credential-store" => Ok(Command::CredentialStore(PlatformCredentialParams {
            platform: platform(rest)?,
            token: std::env::var("TIKTOOLS_PLATFORM_TOKEN")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "credential-store needs TIKTOOLS_PLATFORM_TOKEN".to_owned())?,
        })),
        "credential-delete" => Ok(Command::CredentialDelete(PlatformCredentialDelete {
            platform: platform(rest)?,
            credential_ref: required_flag(rest, "--credential-ref")?,
        })),
        "connect" => {
            let unique_id = flag_value(rest, "--unique-id")
                .ok_or_else(|| "live connect needs --unique-id <id>".to_owned())?;
            let session_cookie = session_cookie(rest, "live connect")?;
            Ok(Command::Connect {
                unique_id,
                session_cookie,
                room_id: flag_value(rest, "--room-id"),
            })
        }
        "pick" => {
            let session_cookie = session_cookie(rest, "live pick")?;
            Ok(Command::Pick { session_cookie })
        }
        "channels" => Ok(Command::Channels {
            keyword: flag_value(rest, "--keyword"),
            offset: flag_u32(rest, "--offset")?,
            limit: flag_u32(rest, "--limit")?,
            // Discovery works without a session (guest bootstrap), so the
            // cookie is optional here, unlike connect/pick.
            session_cookie: optional_session_cookie(rest),
        }),
        "disconnect" => Ok(Command::Disconnect),
        "status" => Ok(Command::Status),
        other => Err(format!("unknown live verb `{other}`").into()),
    }
}
fn required_flag(args: &[String], name: &str) -> Result<String, CommandError> {
    flag_value(args, name).ok_or_else(|| format!("Missing {name}").into())
}
fn platform(args: &[String]) -> Result<PlatformId, CommandError> {
    serde_json::from_value(Value::String(required_flag(args, "--platform")?)).map_err(|_| {
        "Unknown platform; use tiktok, twitch, youtube, facebook, or kick"
            .to_owned()
            .into()
    })
}

/// Reads a numeric `--flag`; a present but non-numeric value is an error
/// rather than a silent default.
fn flag_u32(args: &[String], flag: &str) -> Result<Option<u32>, CommandError> {
    match flag_value(args, flag) {
        Some(raw) => Ok(Some(raw.parse::<u32>().map_err(|_| -> CommandError {
            format!("{flag} needs a whole number").into()
        })?)),
        None => Ok(None),
    }
}

/// Session cookie from `--session-cookie` or `TIKTOOLS_SESSION_COOKIE`;
/// the flag wins when both are set. The cookie value never appears in
/// usage text or diagnostics.
fn session_cookie(args: &[String], usage: &str) -> Result<String, CommandError> {
    let value = optional_session_cookie(args);
    if value.is_empty() {
        return Err(format!(
            "{usage} needs --session-cookie <cookie> (or TIKTOOLS_SESSION_COOKIE)"
        )
        .into());
    }
    Ok(value)
}

fn optional_session_cookie(args: &[String]) -> String {
    flag_value(args, "--session-cookie")
        .or_else(|| {
            std::env::var("TIKTOOLS_SESSION_COOKIE")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_default()
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::Platforms => result_value(client.live_platforms().await?),
        Command::Connections => result_value(client.live_connections().await?),
        Command::PlatformConnect(params) => {
            result_value(client.live_platform_connect(params).await?)
        }
        Command::PlatformDisconnect(params) => {
            result_value(client.live_platform_disconnect(params).await?)
        }
        Command::CredentialStore(params) => {
            result_value(client.live_platform_store_credential(params).await?)
        }
        Command::CredentialDelete(params) => {
            result_value(client.live_platform_delete_credential(params).await?)
        }
        Command::PlatformAnalytics => result_value(client.live_platform_analytics().await?),
        Command::Connect {
            unique_id,
            session_cookie,
            room_id,
        } => result_value(
            client
                .live_connect(LiveConnectParams {
                    unique_id,
                    session_cookie,
                    room_id,
                })
                .await?,
        ),
        Command::Pick { session_cookie } => {
            result_value(client.live_pick(LivePickParams { session_cookie }).await?)
        }
        Command::Channels {
            keyword,
            offset,
            limit,
            session_cookie,
        } => result_value(
            client
                .live_channels(LiveChannelsParams {
                    keyword,
                    offset,
                    limit,
                    session_cookie,
                })
                .await?,
        ),
        Command::Disconnect => result_value(client.live_disconnect().await?),
        Command::Status => result_value(client.live_status().await?),
    }
}

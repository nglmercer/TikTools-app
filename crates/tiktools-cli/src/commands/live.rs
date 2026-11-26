//! `live` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::live::{LiveChannelsParams, LiveConnectParams, LivePickParams};

use super::args::{flag_value, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
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

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "live",
    summary: "TikTok LIVE connection",
    verbs: &[
        VerbSpec::new(
            "connect",
            "live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]",
            "connect to a live room",
        ),
        VerbSpec::new(
            "pick",
            "live pick --session-cookie <cookie>",
            "pick a live room interactively",
        ),
        VerbSpec::new(
            "channels",
            "live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]",
            "discover live channels; works without a session",
        ),
        VerbSpec::new(
            "disconnect",
            "live disconnect",
            "disconnect from the current room",
        )
        .destructive(),
        VerbSpec::new("status", "live status", "current connection status"),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "live <verb> ...")?;
    match verb {
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

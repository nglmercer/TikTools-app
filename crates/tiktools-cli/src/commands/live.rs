//! `live` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::live::{LiveConnectParams, LivePickParams};

use super::args::{flag_value, split_first};
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
    Disconnect,
    Status,
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "live <verb> ...")?;
    match verb {
        "connect" => {
            let unique_id = flag_value(rest, "--unique-id")
                .ok_or_else(|| "live connect needs --unique-id <id>".to_owned())?;
            let session_cookie = flag_value(rest, "--session-cookie")
                .ok_or_else(|| "live connect needs --session-cookie <cookie>".to_owned())?;
            Ok(Command::Connect {
                unique_id,
                session_cookie,
                room_id: flag_value(rest, "--room-id"),
            })
        }
        "pick" => {
            let session_cookie = flag_value(rest, "--session-cookie")
                .ok_or_else(|| "live pick needs --session-cookie <cookie>".to_owned())?;
            Ok(Command::Pick { session_cookie })
        }
        "disconnect" => Ok(Command::Disconnect),
        "status" => Ok(Command::Status),
        other => Err(format!("unknown live verb `{other}`").into()),
    }
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
        Command::Disconnect => result_value(client.live_disconnect().await?),
        Command::Status => result_value(client.live_status().await?),
    }
}

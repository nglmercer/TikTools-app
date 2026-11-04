//! Cloud account commands. `login` runs the whole system-browser loop in
//! one process (begin, print the URL, complete); every other command is a
//! single control call.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};

use super::args::split_first;
use super::{result_value, CommandError};

pub enum Command {
    Status,
    Login,
    Logout,
    Restore,
}

pub fn parse(rest: &[String]) -> Result<Command, CommandError> {
    let (verb, _) =
        split_first(rest, "cloud <status|login|logout|restore>").map_err(CommandError::Usage)?;
    match verb {
        "status" => Ok(Command::Status),
        "login" => Ok(Command::Login),
        "logout" => Ok(Command::Logout),
        "restore" => Ok(Command::Restore),
        other => Err(format!("unknown cloud command `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::Status => result_value(
            client
                .call_value("cloud.status", serde_json::json!({}))
                .await?,
        ),
        Command::Restore => result_value(
            client
                .call_value("cloud.restore", serde_json::json!({}))
                .await?,
        ),
        Command::Logout => result_value(
            client
                .call_value("cloud.logout", serde_json::json!({}))
                .await?,
        ),
        Command::Login => {
            let begun: Value = client
                .call_value("cloud.login.begin", serde_json::json!({}))
                .await?;
            let url = begun
                .get("browserUrl")
                .and_then(|url| url.as_str())
                .unwrap_or_default()
                .to_owned();
            eprintln!("Open this URL in your browser to sign in:\n{url}");
            let _ = client
                .call_value("cloud.login.open", serde_json::json!({}))
                .await;
            let status: Value = client
                .call_value("cloud.login.complete", serde_json::json!({}))
                .await?;
            result_value(status)
        }
    }
}

//! `points` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::points::{
    PartialPointsConfig, PointsAdjustParams, PointsLeaderboardParams, PointsResetParams,
    PointsViewerParams,
};

use super::args::{flag_value, kv_object, one_arg, split_first};
use super::{result_value, typed_params, CommandError};

pub enum Command {
    ConfigGet,
    ConfigSet { config: PartialPointsConfig },
    ViewerGet { id: String },
    Adjust { unique_id: String, delta: f64 },
    Leaderboard { limit: Option<i64> },
    Reset { unique_id: Option<String> },
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "points <verb> ...")?;
    match verb {
        "config" => {
            let (verb, rest) = split_first(rest, "points config <get|set> ...")?;
            match verb {
                "get" => Ok(Command::ConfigGet),
                "set" => Ok(Command::ConfigSet {
                    config: typed_params(kv_object(rest)?)?,
                }),
                other => Err(format!("unknown points config verb `{other}`").into()),
            }
        }
        "viewer" => {
            let (verb, rest) = split_first(rest, "points viewer get <id>")?;
            if verb != "get" {
                return Err(format!("unknown points viewer verb `{verb}`").into());
            }
            Ok(Command::ViewerGet {
                id: one_arg(rest, "points viewer get <id>")?,
            })
        }
        "adjust" => {
            if rest.len() != 2 {
                return Err("points adjust <id> <delta>".to_owned().into());
            }
            let delta: f64 = rest[1]
                .parse()
                .map_err(|_| "delta must be a number".to_owned())?;
            Ok(Command::Adjust {
                unique_id: rest[0].clone(),
                delta,
            })
        }
        "leaderboard" => {
            let limit = flag_value(rest, "--limit")
                .map(|raw| {
                    raw.parse::<i64>()
                        .map_err(|_| "--limit must be an integer".to_owned())
                })
                .transpose()?;
            Ok(Command::Leaderboard { limit })
        }
        "reset" => {
            if rest.len() > 1 {
                return Err("points reset [id]".to_owned().into());
            }
            Ok(Command::Reset {
                unique_id: rest.first().cloned(),
            })
        }
        other => Err(format!("unknown points verb `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::ConfigGet => result_value(client.points_config_get().await?),
        Command::ConfigSet { config } => result_value(client.points_config_set(config).await?),
        Command::ViewerGet { id } => result_value(
            client
                .points_viewer_get(PointsViewerParams { unique_id: id })
                .await?,
        ),
        Command::Adjust { unique_id, delta } => result_value(
            client
                .points_adjust(PointsAdjustParams { unique_id, delta })
                .await?,
        ),
        Command::Leaderboard { limit } => result_value(
            client
                .points_leaderboard(PointsLeaderboardParams { limit })
                .await?,
        ),
        Command::Reset { unique_id } => {
            result_value(client.points_reset(PointsResetParams { unique_id }).await?)
        }
    }
}

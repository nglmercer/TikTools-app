//! `media` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::media::{MediaKind, MediaPlayParams, MediaValidateParams};

use super::args::{flag_value, split_first};
use super::{result_value, CommandError};

pub enum Command {
    Validate {
        path: String,
        kind: Option<MediaKind>,
    },
    Play {
        path: String,
        kind: Option<MediaKind>,
        volume: Option<f32>,
    },
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "media <verb> ...")?;
    match verb {
        "validate" => {
            let (path, rest) = split_first(rest, "media validate <path> [--kind k]")?;
            Ok(Command::Validate {
                path: path.to_owned(),
                kind: parse_kind(flag_value(rest, "--kind"))?,
            })
        }
        "play" => {
            let (path, rest) = split_first(rest, "media play <path> [--kind k] [--volume v]")?;
            let volume = flag_value(rest, "--volume")
                .map(|raw| {
                    raw.parse::<f64>()
                        .map_err(|_| "--volume must be a number within 0.0..=1.0".to_owned())
                })
                .transpose()?
                // The untyped CLI used to forward the f64 through JSON, where
                // non-finite values became null (no volume override). Keep
                // that exact mapping instead of sending inf/NaN to the host.
                .and_then(|volume| volume.is_finite().then_some(volume as f32));
            Ok(Command::Play {
                path: path.to_owned(),
                kind: parse_kind(flag_value(rest, "--kind"))?,
                volume,
            })
        }
        other => Err(format!("unknown media verb `{other}`").into()),
    }
}

/// Parses `--kind audio|video|image|other`. An unknown kind used to fail
/// in the host with `invalid_params`; keep that code so the exit status
/// is unchanged now that the CLI builds typed params.
fn parse_kind(raw: Option<String>) -> Result<Option<MediaKind>, CommandError> {
    raw.map(|raw| match raw.as_str() {
        "audio" => Ok(MediaKind::Audio),
        "video" => Ok(MediaKind::Video),
        "image" => Ok(MediaKind::Image),
        "other" => Ok(MediaKind::Other),
        other => Err(ClientError::new(
            "invalid_params",
            format!("unknown media kind `{other}` (expected audio|video|image|other)"),
        )
        .into()),
    })
    .transpose()
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::Validate { path, kind } => result_value(
            client
                .media_validate(MediaValidateParams { path, kind })
                .await?,
        ),
        Command::Play { path, kind, volume } => result_value(
            client
                .media_play(MediaPlayParams { path, kind, volume })
                .await?,
        ),
    }
}

//! `media` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::media::{MediaKind, MediaValidateParams};

use super::args::{flag_value, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    Validate {
        path: String,
        kind: Option<MediaKind>,
    },
}

const U_VALIDATE: &str = "media validate <path> [--kind audio|video|image|other]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "media",
    summary: "media file validation and playback",
    verbs: &[VerbSpec::new(
        "validate",
        U_VALIDATE,
        "validate a media file before use in actions",
    )],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "media <verb> ...")?;
    match verb {
        "validate" => {
            let (path, rest) = split_first(rest, U_VALIDATE)?;
            Ok(Command::Validate {
                path: path.to_owned(),
                kind: parse_kind(flag_value(rest, "--kind"))?,
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
    }
}

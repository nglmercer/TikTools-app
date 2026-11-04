//! Cloud publisher commands. Every command calls one typed `publisher.*`
//! control operation on the local host; the session token and the backend
//! password never leave the host process, and private keys stay in
//! owner-only files under the application data directory.
//!
//! `enroll` reads the backend account password from stdin so it never lands
//! in shell history: pipe it in (`printf '%s' "$PASS" | tiktools publisher
//! enroll …`) or type it when prompted. Typed input echoes on a terminal;
//! prefer piping from a password manager.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::publisher::{
    PublisherCreateParams, PublisherEnrollParams, PublisherIdParams, PublisherReleaseParams,
    PublisherSubmitParams, PublisherUploadParams,
};

use super::args::{flag_value, one_arg, positional, split_first};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Create { namespace: String },
    Keys { publisher_id: String },
    Enroll { publisher_id: String, key_id: Option<String> },
    Upload { publisher_id: String, path: String },
    Submit {
        upload_id: String,
        key_id: String,
        plugin_id: String,
        version: String,
        target: Option<String>,
    },
    Release { id: String },
    Publish { id: String },
}

pub fn parse(rest: &[String]) -> Result<Command, CommandError> {
    let usage = "publisher <list|create|keys|enroll|upload|submit|release|publish> ...";
    let (verb, rest) = split_first(rest, usage).map_err(CommandError::from)?;
    match verb {
        "list" => Ok(Command::List),
        "create" => Ok(Command::Create {
            namespace: one_arg(rest, "publisher create <namespace>")?,
        }),
        "keys" => Ok(Command::Keys {
            publisher_id: one_arg(rest, "publisher keys <publisher-id>")?,
        }),
        "enroll" => {
            let usage = "publisher enroll <publisher-id> [--key-id <id>]";
            Ok(Command::Enroll {
                publisher_id: positional(rest, 0, usage)?,
                key_id: flag_value(rest, "--key-id"),
            })
        }
        "upload" => {
            let usage = "publisher upload <publisher-id> <path>";
            Ok(Command::Upload {
                publisher_id: positional(rest, 0, usage)?,
                path: positional(rest, 1, usage)?,
            })
        }
        "submit" => {
            let usage =
                "publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]";
            Ok(Command::Submit {
                upload_id: positional(rest, 0, usage)?,
                key_id: positional(rest, 1, usage)?,
                plugin_id: positional(rest, 2, usage)?,
                version: positional(rest, 3, usage)?,
                target: flag_value(rest, "--target"),
            })
        }
        "release" => Ok(Command::Release {
            id: one_arg(rest, "publisher release <release-id>")?,
        }),
        "publish" => Ok(Command::Publish {
            id: one_arg(rest, "publisher publish <release-id>")?,
        }),
        other => Err(format!("unknown publisher verb `{other}`\n{usage}").into()),
    }
}

fn read_password() -> Result<String, CommandError> {
    use std::io::{BufRead, IsTerminal, Write};
    let mut stderr = std::io::stderr();
    let _ = writeln!(
        stderr,
        "Backend account password (piping avoids terminal echo):"
    );
    let _ = stderr.flush();
    if std::io::stdin().is_terminal() {
        let _ = writeln!(
            stderr,
            "warning: typed input will echo; prefer piping from a password manager"
        );
        let _ = stderr.flush();
    }
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|error| format!("could not read the password: {error}"))?;
    let password = line.trim_end_matches(['\r', '\n']).to_owned();
    if password.is_empty() {
        return Err("the password must not be empty".to_owned().into());
    }
    Ok(password)
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.publisher_list().await?),
        Command::Create { namespace } => {
            result_value(client.publisher_create(PublisherCreateParams { namespace }).await?)
        }
        Command::Keys { publisher_id } => {
            result_value(client.publisher_keys(PublisherIdParams { publisher_id }).await?)
        }
        Command::Enroll {
            publisher_id,
            key_id,
        } => {
            let password =
                read_password().map_err(|error: CommandError| to_client_error(error))?;
            result_value(
                client
                    .publisher_enroll(PublisherEnrollParams {
                        publisher_id,
                        password,
                        key_id,
                    })
                    .await?,
            )
        }
        Command::Upload { publisher_id, path } => result_value(
            client
                .publisher_upload(PublisherUploadParams { publisher_id, path })
                .await?,
        ),
        Command::Submit {
            upload_id,
            key_id,
            plugin_id,
            version,
            target,
        } => result_value(
            client
                .publisher_submit(PublisherSubmitParams {
                    upload_id,
                    publisher_key_id: key_id,
                    plugin_id,
                    version,
                    target: target.unwrap_or_else(|| "any".to_owned()),
                })
                .await?,
        ),
        Command::Release { id } => {
            result_value(client.publisher_release(PublisherReleaseParams { id }).await?)
        }
        Command::Publish { id } => {
            result_value(client.publisher_publish(PublisherReleaseParams { id }).await?)
        }
    }
}

fn to_client_error(error: CommandError) -> ClientError {
    match error {
        CommandError::Usage(message) => ClientError::new("usage", message),
        CommandError::Call(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::Command;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn verbs_parse_with_positionals_and_flags() {
        assert!(matches!(parse(&args(&["list"])), Ok(Command::List)));
        assert!(matches!(
            parse(&args(&["create", "acme"])),
            Ok(Command::Create { .. })
        ));
        assert!(matches!(
            parse(&args(&["keys", "pub-1"])),
            Ok(Command::Keys { .. })
        ));
        assert!(matches!(
            parse(&args(&["enroll", "pub-1"])),
            Ok(Command::Enroll {
                key_id: None,
                ..
            })
        ));
        assert!(matches!(
            parse(&args(&["enroll", "pub-1", "--key-id", "acme-1"])),
            Ok(Command::Enroll {
                key_id: Some(_),
                ..
            })
        ));
        assert!(matches!(
            parse(&args(&["upload", "pub-1", "a.plugin"])),
            Ok(Command::Upload { .. })
        ));
        assert!(matches!(
            parse(&args(&[
                "submit", "up-1", "key-1", "acme.tts", "1.2.3"
            ])),
            Ok(Command::Submit { target: None, .. })
        ));
        assert!(matches!(
            parse(&args(&[
                "submit", "up-1", "key-1", "acme.tts", "1.2.3", "--target", "beta"
            ])),
            Ok(Command::Submit {
                target: Some(_),
                ..
            })
        ));
        assert!(matches!(
            parse(&args(&["release", "rel-1"])),
            Ok(Command::Release { .. })
        ));
        assert!(matches!(
            parse(&args(&["publish", "rel-1"])),
            Ok(Command::Publish { .. })
        ));
    }

    #[test]
    fn verbs_reject_missing_positionals() {
        for words in [
            vec!["create"],
            vec!["keys"],
            vec!["enroll"],
            vec!["upload", "pub-1"],
            vec!["submit", "up-1", "key-1"],
            vec!["release"],
            vec!["publish"],
            vec!["nope"],
        ] {
            assert!(parse(&args(&words)).is_err(), "{words:?} must fail");
        }
    }
}

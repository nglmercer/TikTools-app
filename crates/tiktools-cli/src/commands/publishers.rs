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
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Create {
        namespace: String,
    },
    Delete {
        publisher: String,
    },
    Keys {
        publisher_id: String,
    },
    Enroll {
        publisher_id: String,
        key_id: Option<String>,
    },
    Upload {
        publisher_id: String,
        path: String,
    },
    Submit {
        upload_id: String,
        key_id: String,
        plugin_id: String,
        version: String,
        target: Option<String>,
    },
    Release {
        id: String,
    },
    Publish {
        id: String,
    },
}

const U_DOMAIN: &str =
    "publisher <list|create|delete|keys|enroll|upload|submit|release|publish> ...";
const U_CREATE: &str = "publisher create <namespace>";
const U_DELETE: &str = "publisher delete <namespace-or-publisher-id>";
const U_KEYS: &str = "publisher keys <publisher-id>";
const U_ENROLL: &str = "publisher enroll <publisher-id> [--key-id <id>]";
const U_UPLOAD: &str = "publisher upload <publisher-id> <path>";
const U_SUBMIT: &str =
    "publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]";
const U_RELEASE: &str = "publisher release <release-id>";
const U_PUBLISH: &str = "publisher publish <release-id>";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "publisher",
    summary: "cloud publisher flow (requires sign-in)",
    verbs: &[
        VerbSpec::new("list", "publisher list", "list publisher namespaces"),
        VerbSpec::new("create", U_CREATE, "create a publisher namespace"),
        VerbSpec::new("delete", U_DELETE, "delete a namespace or publisher").destructive(),
        VerbSpec::new("keys", U_KEYS, "list signing keys of a publisher"),
        VerbSpec::new(
            "enroll",
            U_ENROLL,
            "enroll a publisher key; reads the backend password from stdin",
        ),
        VerbSpec::new("upload", U_UPLOAD, "upload a plugin package"),
        VerbSpec::new("submit", U_SUBMIT, "submit an upload for review"),
        VerbSpec::new("release", U_RELEASE, "create a release"),
        VerbSpec::new("publish", U_PUBLISH, "publish a release"),
    ],
};

pub fn parse(rest: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(rest, U_DOMAIN).map_err(CommandError::from)?;
    match verb {
        "list" => Ok(Command::List),
        "create" => Ok(Command::Create {
            namespace: one_arg(rest, U_CREATE)?,
        }),
        "delete" => Ok(Command::Delete {
            publisher: one_arg(rest, U_DELETE)?,
        }),
        "keys" => Ok(Command::Keys {
            publisher_id: one_arg(rest, U_KEYS)?,
        }),
        "enroll" => Ok(Command::Enroll {
            publisher_id: positional(rest, 0, U_ENROLL)?,
            key_id: flag_value(rest, "--key-id"),
        }),
        "upload" => Ok(Command::Upload {
            publisher_id: positional(rest, 0, U_UPLOAD)?,
            path: positional(rest, 1, U_UPLOAD)?,
        }),
        "submit" => Ok(Command::Submit {
            upload_id: positional(rest, 0, U_SUBMIT)?,
            key_id: positional(rest, 1, U_SUBMIT)?,
            plugin_id: positional(rest, 2, U_SUBMIT)?,
            version: positional(rest, 3, U_SUBMIT)?,
            target: flag_value(rest, "--target"),
        }),
        "release" => Ok(Command::Release {
            id: one_arg(rest, U_RELEASE)?,
        }),
        "publish" => Ok(Command::Publish {
            id: one_arg(rest, U_PUBLISH)?,
        }),
        other => Err(format!("unknown publisher verb `{other}`\n{U_DOMAIN}").into()),
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
        Command::Create { namespace } => result_value(
            client
                .publisher_create(PublisherCreateParams { namespace })
                .await?,
        ),
        Command::Delete { publisher } => result_value(
            client
                .publisher_delete(PublisherIdParams {
                    publisher_id: publisher,
                })
                .await?,
        ),
        Command::Keys { publisher_id } => result_value(
            client
                .publisher_keys(PublisherIdParams { publisher_id })
                .await?,
        ),
        Command::Enroll {
            publisher_id,
            key_id,
        } => {
            let password = read_password().map_err(|error: CommandError| to_client_error(error))?;
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
        Command::Release { id } => result_value(
            client
                .publisher_release(PublisherReleaseParams { id })
                .await?,
        ),
        Command::Publish { id } => result_value(
            client
                .publisher_publish(PublisherReleaseParams { id })
                .await?,
        ),
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
            parse(&args(&["delete", "tiktok-live-appv3"])),
            Ok(Command::Delete { .. })
        ));
        assert!(matches!(
            parse(&args(&["keys", "pub-1"])),
            Ok(Command::Keys { .. })
        ));
        assert!(matches!(
            parse(&args(&["enroll", "pub-1"])),
            Ok(Command::Enroll { key_id: None, .. })
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
            parse(&args(&["submit", "up-1", "key-1", "acme.tts", "1.2.3"])),
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
            vec!["delete"],
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

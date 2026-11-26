//! Cloud account commands. `login` runs the whole system-browser loop in
//! one process (begin, print the URL, complete); every other command is a
//! single control call.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};

use super::args::split_first;
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    Status,
    Login,
    Logout,
    Restore,
    Backup(BackupCommand),
}

pub enum BackupCommand {
    Create { out: Option<String> },
    Preview { file: String },
    Restore { file: String, yes: bool },
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

const U_DOMAIN: &str =
    "cloud <status|login|logout|restore|backup create|backup preview|backup restore>";
const U_BACKUP: &str = "cloud backup <create|preview|restore>";
const U_PREVIEW: &str = "cloud backup preview <file>";
const U_RESTORE: &str = "cloud backup restore <file> [--yes]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "cloud",
    summary: "cloud account and portable backup/restore (override with TIKTOOLS_CLOUD_ORIGIN)",
    verbs: &[
        VerbSpec::new("status", "cloud status", "cloud account status"),
        VerbSpec::new("login", "cloud login", "sign in through the system browser"),
        VerbSpec::new("logout", "cloud logout", "sign out").destructive(),
        VerbSpec::new("restore", "cloud restore", "restore the cloud session"),
        VerbSpec::new(
            "backup create",
            "cloud backup create [--out FILE]",
            "write a portable backup archive",
        ),
        VerbSpec::new("backup preview", U_PREVIEW, "inspect a backup archive"),
        VerbSpec::new("backup restore", U_RESTORE, "restore a backup archive").destructive(),
    ],
};

pub fn parse(rest: &[String]) -> Result<Command, CommandError> {
    let (verb, tail) = split_first(rest, U_DOMAIN).map_err(CommandError::Usage)?;
    match verb {
        "status" => Ok(Command::Status),
        "login" => Ok(Command::Login),
        "logout" => Ok(Command::Logout),
        "restore" => Ok(Command::Restore),
        "backup" => {
            let (action, args) = split_first(tail, U_BACKUP).map_err(CommandError::Usage)?;
            match action {
                "create" => Ok(Command::Backup(BackupCommand::Create {
                    out: flag_value(args, "--out"),
                })),
                "preview" => {
                    let file = args
                        .first()
                        .cloned()
                        .ok_or_else(|| CommandError::from(U_PREVIEW.to_owned()))?;
                    Ok(Command::Backup(BackupCommand::Preview { file }))
                }
                "restore" => {
                    let file = args
                        .first()
                        .cloned()
                        .ok_or_else(|| CommandError::from(U_RESTORE.to_owned()))?;
                    Ok(Command::Backup(BackupCommand::Restore {
                        file,
                        yes: args.iter().any(|arg| arg == "--yes"),
                    }))
                }
                other => Err(format!("unknown cloud backup action `{other}`").into()),
            }
        }
        other => Err(format!("unknown cloud command `{other}`").into()),
    }
}

async fn read_manifest(file: &str) -> Result<Value, ClientError> {
    let text = tokio::fs::read_to_string(file)
        .await
        .map_err(|error| ClientError::new("io", format!("cannot read {file}: {error}")))?;
    serde_json::from_str(&text)
        .map_err(|error| ClientError::new("protocol", format!("invalid backup file: {error}")))
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
        Command::Backup(BackupCommand::Create { out }) => {
            let manifest = client
                .call_value("cloud.backup.create", serde_json::json!({}))
                .await?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let file = out.unwrap_or_else(|| format!("tiktools-cloud-backup-{stamp}.json"));
            let bytes = serde_json::to_vec_pretty(&manifest)
                .map_err(|error| ClientError::new("protocol", error.to_string()))?;
            tokio::fs::write(&file, bytes)
                .await
                .map_err(|error| ClientError::new("io", format!("cannot write {file}: {error}")))?;
            eprintln!("Backup written to {file}");
            result_value(serde_json::json!({
                "file": file,
                "schemaVersion": manifest.get("schemaVersion").cloned(),
            }))
        }
        Command::Backup(BackupCommand::Preview { file }) => {
            let manifest = read_manifest(&file).await?;
            let summary = client
                .call_value(
                    "cloud.backup.preview",
                    serde_json::json!({"manifest": manifest, "mode": "replace", "dryRun": true}),
                )
                .await?;
            eprintln!("Preview: {summary}");
            result_value(summary)
        }
        Command::Backup(BackupCommand::Restore { file, yes }) => {
            let manifest = read_manifest(&file).await?;
            let summary = client
                .call_value(
                    "cloud.backup.preview",
                    serde_json::json!({"manifest": manifest, "mode": "replace", "dryRun": true}),
                )
                .await?;
            eprintln!("About to restore: {summary}");
            if !yes {
                eprintln!("Re-run with --yes to apply the restore.");
                return result_value(summary);
            }
            let result = client
                .call_value(
                    "cloud.backup.restore",
                    serde_json::json!({"manifest": manifest, "mode": "replace", "dryRun": false}),
                )
                .await?;
            result_value(result)
        }
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

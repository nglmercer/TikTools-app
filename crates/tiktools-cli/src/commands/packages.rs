use std::time::Duration;

use serde_json::{json, Value};
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::packages::{
    JobParams, JobSnapshot, PackageInspectParams, PackageInstallParams, PackagePluginParams,
    PackagePolicyParams, PackageRollbackParams, PackageSpecInput, PackageUpdateParams, PolicyPatch,
};
use tiktools_control_api::modules::registry::{
    CloudCatalogQuery, CloudCatalogSummariesQuery, CloudCatalogVersionsQuery,
    RegistryPackageParams, RegistryRefreshParams, SearchQuery,
};

use super::args::{flag_value, one_arg, positional, split_first};
use super::{result_value, CommandError};

const JOB_POLL_INTERVAL: Duration = Duration::from_millis(250);

pub enum Command {
    Search {
        query: String,
        category: Option<String>,
        page: u32,
    },
    Inspect {
        spec: String,
    },
    Install {
        spec: String,
        yes: bool,
        allow_unsigned: bool,
        wait: bool,
    },
    Outdated,
    Update {
        id: Option<String>,
        version: Option<String>,
        yes: bool,
        wait: bool,
    },
    Rollback {
        id: String,
        version: Option<String>,
        wait: bool,
    },
    Versions {
        id: String,
    },
    Policy {
        id: String,
        patch: PolicyPatch,
    },
    Jobs,
    Job {
        id: String,
    },
    Cancel {
        id: String,
    },
    RegistryList,
    RegistryRefresh,
    RegistryPackage {
        id: String,
    },
    RegistryCloudCatalog {
        query: String,
        after: Option<String>,
        generation: Option<String>,
        target: Option<String>,
    },
    RegistryCloudSummaries {
        query: Option<String>,
        cursor: Option<String>,
        generation: Option<String>,
        target: Option<String>,
    },
    RegistryCloudVersions {
        id: String,
        cursor: Option<String>,
        generation: Option<String>,
        target: Option<String>,
    },
    RegistryCloudPaginationCapabilities,
    RegistryDefaults,
}

pub fn is_local_archive(spec: &str) -> bool {
    let lower = spec.to_ascii_lowercase();
    !lower.starts_with("https://")
        && !lower.starts_with("http://")
        && (lower.ends_with(".plugin")
            || lower.starts_with("file:")
            || spec.contains('/')
            || spec.contains('\\'))
}

pub fn parse_plugin(verb: &str, rest: &[String]) -> Option<Result<Command, CommandError>> {
    let wait = !rest.iter().any(|arg| arg == "--no-wait");
    let yes = rest.iter().any(|arg| arg == "--yes" || arg == "-y");
    let parsed = match verb {
        "search"
            if rest
                .iter()
                .any(|arg| arg == "--registry" || arg.starts_with("--registry=")) =>
        {
            Err("plugin sources are managed by the application"
                .to_owned()
                .into())
        }
        "search" => Ok(Command::Search {
            query: rest
                .iter()
                .take_while(|arg| !arg.starts_with("--"))
                .cloned()
                .collect::<Vec<_>>()
                .join(" "),
            category: flag_value(rest, "--category"),
            page: flag_value(rest, "--page")
                .and_then(|page| page.parse().ok())
                .unwrap_or(1),
        }),
        "inspect" => one_arg(rest, "plugin inspect <spec>")
            .map(|spec| Command::Inspect { spec })
            .map_err(Into::into),
        "outdated" => Ok(Command::Outdated),
        "update" => {
            let all = rest.iter().any(|arg| arg == "--all");
            let id = positional(rest, 0, "plugin update <id> | plugin update --all").ok();
            match (all, id) {
                (true, _) | (false, Some(_)) => Ok(Command::Update {
                    id: if all {
                        None
                    } else {
                        positional(rest, 0, "").ok()
                    },
                    version: flag_value(rest, "--version"),
                    yes,
                    wait,
                }),
                (false, None) => Err(
                    "plugin update <id> [--version v] [--yes] | plugin update --all"
                        .to_owned()
                        .into(),
                ),
            }
        }
        "rollback" => positional(rest, 0, "plugin rollback <id> [version]")
            .map(|id| Command::Rollback {
                id,
                version: positional(rest, 1, "").ok(),
                wait,
            })
            .map_err(Into::into),
        "versions" => one_arg(rest, "plugin versions <id>")
            .map(|id| Command::Versions { id })
            .map_err(Into::into),
        "pin" | "unpin" => one_arg(rest, "plugin pin|unpin <id>")
            .map(|id| Command::Policy {
                id,
                patch: PolicyPatch {
                    channel: None,
                    auto_update: None,
                    pinned: Some(verb == "pin"),
                },
            })
            .map_err(Into::into),
        "policy" => policy(rest),
        "jobs" => Ok(Command::Jobs),
        "job" => one_arg(rest, "plugin job <job-id>")
            .map(|id| Command::Job { id })
            .map_err(Into::into),
        "cancel" => one_arg(rest, "plugin cancel <job-id>")
            .map(|id| Command::Cancel { id })
            .map_err(Into::into),
        _ => return None,
    };
    Some(parsed)
}

pub fn parse_install(spec: &str, rest: &[String]) -> Command {
    Command::Install {
        spec: spec.to_owned(),
        yes: rest.iter().any(|arg| arg == "--yes" || arg == "-y"),
        allow_unsigned: rest.iter().any(|arg| arg == "--allow-unsigned"),
        wait: !rest.iter().any(|arg| arg == "--no-wait"),
    }
}

fn policy(rest: &[String]) -> Result<Command, CommandError> {
    let usage = "plugin policy <id> [--channel stable|beta] [--auto off|notify|patch|minor]";
    let id = positional(rest, 0, usage)?;
    let auto_update = flag_value(rest, "--auto")
        .map(|value| {
            serde_json::from_value(Value::String(value.clone()))
                .map_err(|_| format!("unknown auto-update rule `{value}`\n{usage}"))
        })
        .transpose()?;
    Ok(Command::Policy {
        id,
        patch: PolicyPatch {
            channel: flag_value(rest, "--channel"),
            auto_update,
            pinned: None,
        },
    })
}

pub fn parse_registry(args: &[String]) -> Result<Command, CommandError> {
    let usage =
        "registry <list|refresh|package|cloud-catalog|cloud-summaries|cloud-versions|cloud-pagination|defaults> ...";
    let (verb, rest) = split_first(args, usage)?;
    match verb {
        "list" => Ok(Command::RegistryList),
        "refresh" if rest.is_empty() => Ok(Command::RegistryRefresh),
        "refresh" => Err("registry refresh".to_owned().into()),
        "package" => Ok(Command::RegistryPackage {
            id: one_arg(rest, "registry package <id>")?,
        }),
        "cloud-catalog" => Ok(Command::RegistryCloudCatalog {
            query: flag_value(rest, "--query").unwrap_or_default(),
            after: flag_value(rest, "--after"),
            generation: flag_value(rest, "--generation"),
            target: flag_value(rest, "--target"),
        }),
        "cloud-summaries" => Ok(Command::RegistryCloudSummaries { query: flag_value(rest,"--query"), cursor: flag_value(rest,"--cursor"), generation: flag_value(rest,"--generation"), target: flag_value(rest,"--target") }),
        "cloud-versions" => Ok(Command::RegistryCloudVersions { id: positional(rest,0,"registry cloud-versions <id> [--target name] [--generation value] [--cursor value]")?, cursor: flag_value(rest,"--cursor"), generation: flag_value(rest,"--generation"), target: flag_value(rest,"--target") }),
        "cloud-pagination" => Ok(Command::RegistryCloudPaginationCapabilities),
        "defaults" => Ok(Command::RegistryDefaults),
        other => Err(format!("unknown registry verb `{other}`\n{usage}").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::Search {
            query,
            category,
            page,
        } => result_value(
            client
                .registry_search(SearchQuery {
                    query,
                    category,
                    registry: None,
                    page,
                })
                .await?,
        ),
        Command::Inspect { spec } => result_value(
            client
                .plugins_packages_inspect(PackageInspectParams {
                    spec: PackageSpecInput::Spec { spec },
                })
                .await?,
        ),
        Command::Install {
            spec,
            yes,
            allow_unsigned,
            wait,
        } => install(client, spec, yes, allow_unsigned, wait).await,
        Command::Outdated => result_value(client.plugins_updates_check().await?),
        Command::Update {
            id,
            version,
            yes,
            wait,
        } => match id {
            Some(id) => update(client, id, version, yes, wait).await,
            None => {
                let report = client.plugins_updates_check().await?;
                let mut results = Vec::new();
                for entry in report
                    .updates
                    .into_iter()
                    .filter(|entry| entry.latest.is_some())
                {
                    let outcome = update(client, entry.id.clone(), entry.latest, yes, wait).await;
                    results.push(match outcome {
                        Ok(value) => value,
                        Err(error) => json!({"id": entry.id, "error": error.message}),
                    });
                }
                Ok(Value::Array(results))
            }
        },
        Command::Rollback { id, version, wait } => {
            let job = client
                .plugins_packages_rollback(PackageRollbackParams {
                    plugin_id: id,
                    version,
                })
                .await?
                .job;
            finish(client, job, wait).await
        }
        Command::Versions { id } => result_value(
            client
                .plugins_packages_versions(PackagePluginParams { plugin_id: id })
                .await?,
        ),
        Command::Policy { id, patch } => result_value(
            client
                .plugins_packages_policy(PackagePolicyParams {
                    plugin_id: id,
                    patch,
                })
                .await?,
        ),
        Command::Jobs => result_value(client.plugins_jobs_list().await?),
        Command::Job { id } => {
            result_value(client.plugins_jobs_get(JobParams { job_id: id }).await?)
        }
        Command::Cancel { id } => {
            result_value(client.plugins_jobs_cancel(JobParams { job_id: id }).await?)
        }
        Command::RegistryList => result_value(client.registry_list().await?),
        Command::RegistryRefresh => result_value(
            client
                .registry_refresh(RegistryRefreshParams { name: None })
                .await?,
        ),
        Command::RegistryPackage { id } => result_value(
            client
                .registry_package(RegistryPackageParams { id, registry: None })
                .await?,
        ),
        Command::RegistryCloudCatalog {
            query,
            after,
            generation,
            target,
        } => result_value(
            client
                .registry_cloud_catalog(CloudCatalogQuery {
                    query,
                    after,
                    generation,
                    target,
                })
                .await?,
        ),
        Command::RegistryCloudSummaries {
            query,
            cursor,
            generation,
            target,
        } => result_value(
            client
                .registry_cloud_catalog_summaries(CloudCatalogSummariesQuery {
                    query,
                    cursor,
                    generation,
                    target,
                })
                .await?,
        ),
        Command::RegistryCloudVersions {
            id,
            cursor,
            generation,
            target,
        } => result_value(
            client
                .registry_cloud_catalog_versions(CloudCatalogVersionsQuery {
                    id,
                    cursor,
                    generation,
                    target,
                })
                .await?,
        ),
        Command::RegistryCloudPaginationCapabilities => result_value(
            client
                .registry_cloud_catalog_pagination_capabilities()
                .await?,
        ),
        Command::RegistryDefaults => result_value(client.registry_defaults_get().await?),
    }
}

async fn install(
    client: &TikToolsClient,
    spec: String,
    yes: bool,
    allow_unsigned: bool,
    wait: bool,
) -> Result<Value, ClientError> {
    let preview = client
        .plugins_packages_inspect(PackageInspectParams {
            spec: PackageSpecInput::Spec { spec: spec.clone() },
        })
        .await?;
    if let Some(incompatibility) = &preview.incompatibility {
        return Err(ClientError::new(
            "install_failed",
            incompatibility.to_string(),
        ));
    }
    if preview.requires_sideload_consent && !allow_unsigned {
        return Err(ClientError::new(
            "install_failed",
            format!(
                "{} is not signed by a trusted publisher; re-run with --allow-unsigned to install it anyway",
                preview.id
            ),
        ));
    }
    if !yes {
        return Ok(json!({
            "confirm": "review the package below, then re-run with --yes to install it",
            "preview": preview,
        }));
    }
    let job = client
        .plugins_packages_install(PackageInstallParams {
            spec: PackageSpecInput::Spec { spec },
            consent_token: preview.consent_token,
            policy: None,
        })
        .await?
        .job;
    finish(client, job, wait).await
}

async fn update(
    client: &TikToolsClient,
    id: String,
    version: Option<String>,
    yes: bool,
    wait: bool,
) -> Result<Value, ClientError> {
    let started = client
        .plugins_packages_update(PackageUpdateParams {
            plugin_id: id.clone(),
            version: version.clone(),
            consent_token: None,
        })
        .await;
    let job = match started {
        Ok(result) => result.job,
        Err(error) => return Err(error),
    };
    let finished = wait_for(client, job).await?;
    let consent_required = finished.error.as_ref().is_some_and(|error| {
        error.code == tiktools_control_api::modules::packages::InstallErrorCode::ConsentRequired
    });
    if !consent_required {
        return result_value(finished);
    }
    let preview = finished
        .error
        .as_ref()
        .and_then(|error| error.details.clone())
        .unwrap_or(Value::Null);
    if !yes {
        return Ok(json!({
            "confirm": "this update changes what the plugin can access; review it, then re-run with --yes",
            "preview": preview,
        }));
    }
    let token = preview
        .get("consentToken")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| ClientError::new("protocol", "update review carried no consent token"))?;
    let job = client
        .plugins_packages_update(PackageUpdateParams {
            plugin_id: id,
            version,
            consent_token: Some(token),
        })
        .await?
        .job;
    finish(client, job, wait).await
}

async fn finish(
    client: &TikToolsClient,
    job: JobSnapshot,
    wait: bool,
) -> Result<Value, ClientError> {
    if !wait {
        return result_value(job);
    }
    let job = wait_for(client, job).await?;
    if let Some(error) = &job.error {
        return Err(ClientError {
            code: "install_failed".to_owned(),
            message: error.message.clone(),
            data: serde_json::to_value(error).ok(),
        });
    }
    result_value(job)
}

async fn wait_for(
    client: &TikToolsClient,
    mut job: JobSnapshot,
) -> Result<JobSnapshot, ClientError> {
    while !job.status.is_finished() {
        tokio::time::sleep(JOB_POLL_INTERVAL).await;
        job = client
            .plugins_jobs_get(JobParams {
                job_id: job.id.clone(),
            })
            .await?
            .job;
    }
    Ok(job)
}

#[cfg(test)]
mod cloud_catalog_tests {
    use super::*;

    #[test]
    fn parses_catalog_cursor_without_losing_generation_precision() {
        let args = [
            "cloud-catalog",
            "--query",
            "acme",
            "--after",
            "acme.p49",
            "--generation",
            "9007199254740993",
            "--target",
            "linux",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        match parse_registry(&args).unwrap_or_else(|_| panic!("catalog command should parse")) {
            Command::RegistryCloudCatalog {
                query,
                after,
                generation,
                target,
            } => {
                assert_eq!(query, "acme");
                assert_eq!(after.as_deref(), Some("acme.p49"));
                assert_eq!(generation.as_deref(), Some("9007199254740993"));
                assert_eq!(target.as_deref(), Some("linux"));
            }
            _ => panic!("expected cloud catalog command"),
        }
    }

    #[test]
    fn parses_bounded_version_cursor_without_losing_generation_precision() {
        let args = [
            "cloud-versions",
            "hotkeys",
            "--generation",
            "9007199254740993",
            "--cursor",
            "opaque",
            "--target",
            "linux",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        match parse_registry(&args).unwrap_or_else(|_| panic!("catalog command should parse")) {
            Command::RegistryCloudVersions {
                id,
                generation,
                cursor,
                target,
            } => {
                assert_eq!(id, "hotkeys");
                assert_eq!(generation.as_deref(), Some("9007199254740993"));
                assert_eq!(cursor.as_deref(), Some("opaque"));
                assert_eq!(target.as_deref(), Some("linux"));
            }
            _ => panic!("expected bounded catalog command"),
        }
    }
}

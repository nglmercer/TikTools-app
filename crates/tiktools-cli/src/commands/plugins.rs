//! `plugin` commands.

use std::collections::BTreeMap;

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::plugins::{
    PluginActionParams, PluginIdParams, PluginInstallParams, PluginOptionsParams,
    PluginUninstallParams,
};
use tiktools_control_api::modules::settings::{PluginSettingsGet, PluginSettingsSet};

use super::args::{flag_value, kv_map, one_arg, parse_json_value, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    List,
    Get {
        id: String,
    },
    Install {
        path: String,
        replace: bool,
    },
    Uninstall {
        id: String,
        purge: bool,
    },
    Package(super::packages::Command),
    Enable {
        id: String,
    },
    Disable {
        id: String,
    },
    Start {
        id: String,
    },
    Stop {
        id: String,
    },
    SettingsGet {
        id: String,
    },
    SettingsSet {
        id: String,
        values: BTreeMap<String, Value>,
    },
    SettingsReset {
        id: String,
    },
    Health {
        id: String,
    },
    Options {
        source: String,
    },
    Action {
        action_type: String,
        config: BTreeMap<String, Value>,
        live: bool,
    },
    /// `plugin commands` — list commands contributed by installed plugins.
    Commands,
    /// `plugin help [id]` — show one contributed command's description/params.
    Help {
        id: Option<String>,
    },
    /// `plugin <id> <command> [--params json] [--live]` — invoke a contributed
    /// command. The host resolves the id and checks ownership.
    Run {
        plugin_id: String,
        command_id: String,
        config: BTreeMap<String, Value>,
        live: bool,
    },
}

const U_GET: &str = "plugin get <id>";
const U_UNINSTALL: &str = "plugin uninstall <id> [--purge]";
const U_ENABLE: &str = "plugin enable <id>";
const U_DISABLE: &str = "plugin disable <id>";
const U_START: &str = "plugin start <id>";
const U_STOP: &str = "plugin stop <id>";
const U_SETTINGS_DOMAIN: &str = "plugin settings <get|set|reset> ...";
const U_SETTINGS_GET: &str = "plugin settings get <id>";
const U_SETTINGS_SET: &str = "plugin settings set <id> k=v...";
const U_SETTINGS_RESET: &str = "plugin settings reset <id>";
const U_HEALTH: &str = "plugin health <id>";
const U_OPTIONS: &str = "plugin options <action-type/field>";
const U_ACTION: &str = "plugin action <type> [--config json] [--live]";
const U_RUN: &str = "plugin <id> <command> [--params json] [--live]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "plugin",
    summary: "install and manage plugins",
    verbs: &[
        VerbSpec::new("list", "plugin list", "list installed plugins"),
        VerbSpec::new("get", U_GET, "read one plugin"),
        VerbSpec::new(
            "install",
            super::packages::U_INSTALL,
            "install from a file, id@version or url",
        ),
        VerbSpec::new("uninstall", U_UNINSTALL, "uninstall a plugin").destructive(),
        VerbSpec::new(
            "search",
            super::packages::U_SEARCH,
            "search packages in the registry",
        ),
        VerbSpec::new(
            "inspect",
            super::packages::U_INSPECT,
            "inspect a package spec before installing",
        ),
        VerbSpec::new(
            "outdated",
            "plugin outdated",
            "list installed plugins with available updates",
        ),
        VerbSpec::new("update", super::packages::U_UPDATE, "update one plugin"),
        VerbSpec::new(
            "update --all",
            super::packages::U_UPDATE_ALL,
            "update every outdated plugin",
        ),
        VerbSpec::new(
            "rollback",
            super::packages::U_ROLLBACK,
            "roll back to a previous version",
        )
        .destructive(),
        VerbSpec::new(
            "versions",
            super::packages::U_VERSIONS,
            "list available versions of a plugin",
        ),
        VerbSpec::new(
            "pin",
            super::packages::U_PIN,
            "pin a plugin to its current version",
        ),
        VerbSpec::new("unpin", super::packages::U_UNPIN, "remove the version pin"),
        VerbSpec::new(
            "policy",
            super::packages::U_POLICY,
            "set channel and auto-update policy",
        ),
        VerbSpec::new("jobs", "plugin jobs", "list package jobs"),
        VerbSpec::new("job", super::packages::U_JOB, "inspect one package job"),
        VerbSpec::new(
            "cancel",
            super::packages::U_CANCEL,
            "cancel a running package job",
        )
        .destructive(),
        VerbSpec::new("enable", U_ENABLE, "enable a plugin"),
        VerbSpec::new("disable", U_DISABLE, "disable a plugin"),
        VerbSpec::new("start", U_START, "start a plugin"),
        VerbSpec::new("stop", U_STOP, "stop a plugin").destructive(),
        VerbSpec::new("settings get", U_SETTINGS_GET, "read plugin settings"),
        VerbSpec::new("settings set", U_SETTINGS_SET, "write plugin settings"),
        VerbSpec::new("settings reset", U_SETTINGS_RESET, "reset plugin settings").destructive(),
        VerbSpec::new("health", U_HEALTH, "plugin health check"),
        VerbSpec::new("options", U_OPTIONS, "options of an action type or field"),
        VerbSpec::new("action", U_ACTION, "run a plugin action"),
        VerbSpec::new(
            "commands",
            "plugin commands",
            "list commands contributed by installed plugins",
        ),
        VerbSpec::new(
            "help",
            "plugin help [id]",
            "describe one contributed command",
        ),
        VerbSpec::new("run", U_RUN, "invoke a plugin-contributed command"),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "plugin <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            id: one_arg(rest, U_GET)?,
        }),
        "install" => {
            let (spec, rest) = split_first(rest, super::packages::U_INSTALL)?;
            if super::packages::is_local_archive(spec) && !rest.iter().any(|arg| arg == "--review")
            {
                Ok(Command::Install {
                    path: spec.trim_start_matches("file:").to_owned(),
                    replace: rest.iter().any(|arg| arg == "--replace"),
                })
            } else {
                Ok(Command::Package(super::packages::parse_install(spec, rest)))
            }
        }
        "uninstall" => Ok(Command::Uninstall {
            id: super::args::positional(rest, 0, U_UNINSTALL)?,
            purge: rest.iter().any(|arg| arg == "--purge"),
        }),
        "enable" => Ok(Command::Enable {
            id: one_arg(rest, U_ENABLE)?,
        }),
        "disable" => Ok(Command::Disable {
            id: one_arg(rest, U_DISABLE)?,
        }),
        "start" => Ok(Command::Start {
            id: one_arg(rest, U_START)?,
        }),
        "stop" => Ok(Command::Stop {
            id: one_arg(rest, U_STOP)?,
        }),
        "settings" => {
            let (verb, rest) = split_first(rest, U_SETTINGS_DOMAIN)?;
            match verb {
                "get" => Ok(Command::SettingsGet {
                    id: one_arg(rest, U_SETTINGS_GET)?,
                }),
                "set" => {
                    let (id, pairs) = split_first(rest, U_SETTINGS_SET)?;
                    Ok(Command::SettingsSet {
                        id: id.to_owned(),
                        values: kv_map(pairs)?,
                    })
                }
                "reset" => Ok(Command::SettingsReset {
                    id: one_arg(rest, U_SETTINGS_RESET)?,
                }),
                other => Err(format!("unknown plugin settings verb `{other}`").into()),
            }
        }
        "health" => Ok(Command::Health {
            id: one_arg(rest, U_HEALTH)?,
        }),
        "options" => Ok(Command::Options {
            source: one_arg(rest, U_OPTIONS)?,
        }),
        "action" => {
            let (action_type, rest) = split_first(rest, U_ACTION)?;
            let config = flag_value(rest, "--config")
                .map(|raw| parse_json_value(&raw))
                .transpose()?
                .unwrap_or(Value::Object(Default::default()));
            let Value::Object(config) = config else {
                return Err("--config must be a JSON object".to_owned().into());
            };
            Ok(Command::Action {
                action_type: action_type.to_owned(),
                config: config.into_iter().collect(),
                live: rest.iter().any(|arg| arg == "--live"),
            })
        }
        "commands" => Ok(Command::Commands),
        "help" => Ok(Command::Help {
            id: rest.first().cloned(),
        }),
        // Namespaced dynamic dispatch: `plugin <id> <command>` where <id> is a
        // registered plugin id, not a built-in verb. Packages use `plugin
        // install/update/...`, so only two-token forms reach this branch.
        plugin_id if !rest.is_empty() => {
            let (command_id, rest) = split_first(rest, U_RUN)?;
            let config = flag_value(rest, "--params")
                .map(|raw| parse_json_value(&raw))
                .transpose()?
                .unwrap_or(Value::Object(Default::default()));
            let Value::Object(config) = config else {
                return Err("--params must be a JSON object".to_owned().into());
            };
            Ok(Command::Run {
                plugin_id: plugin_id.to_owned(),
                command_id: command_id.to_owned(),
                config: config.into_iter().collect(),
                live: rest.iter().any(|arg| arg == "--live"),
            })
        }
        other => match super::packages::parse_plugin(other, rest) {
            Some(command) => command.map(Command::Package),
            None => Err(format!("unknown plugin verb `{other}`").into()),
        },
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List => result_value(client.plugins_list().await?),
        Command::Get { id } => {
            result_value(client.plugins_get(PluginIdParams { plugin_id: id }).await?)
        }
        Command::Install { path, replace } => result_value(
            client
                .plugins_install(PluginInstallParams {
                    path,
                    replace_existing: replace,
                })
                .await?,
        ),
        Command::Uninstall { id, purge } => result_value(
            client
                .plugins_uninstall(PluginUninstallParams {
                    plugin_id: id,
                    purge_data: purge,
                })
                .await?,
        ),
        Command::Package(command) => super::packages::execute(client, command).await,
        Command::Enable { id } => result_value(
            client
                .plugins_enable(PluginIdParams { plugin_id: id })
                .await?,
        ),
        Command::Disable { id } => result_value(
            client
                .plugins_disable(PluginIdParams { plugin_id: id })
                .await?,
        ),
        Command::Start { id } => result_value(
            client
                .plugins_start(PluginIdParams { plugin_id: id })
                .await?,
        ),
        Command::Stop { id } => result_value(
            client
                .plugins_stop(PluginIdParams { plugin_id: id })
                .await?,
        ),
        Command::SettingsGet { id } => result_value(
            client
                .plugins_settings_get(PluginSettingsGet { plugin_id: id })
                .await?,
        ),
        Command::SettingsSet { id, values } => result_value(
            client
                .plugins_settings_set(PluginSettingsSet {
                    plugin_id: id,
                    values,
                })
                .await?,
        ),
        Command::SettingsReset { id } => result_value(
            client
                .plugins_settings_reset(PluginSettingsGet { plugin_id: id })
                .await?,
        ),
        Command::Health { id } => result_value(
            client
                .plugins_health(PluginIdParams { plugin_id: id })
                .await?,
        ),
        Command::Options { source } => result_value(
            client
                .plugins_options(PluginOptionsParams {
                    source,
                    refresh: false,
                    plugin_id: None,
                    context: None,
                })
                .await?,
        ),
        Command::Action {
            action_type,
            config,
            live,
        } => result_value(
            client
                .plugins_action_execute(PluginActionParams {
                    action_type,
                    config,
                    live,
                    plugin_id: None,
                })
                .await?,
        ),
        Command::Commands => {
            let commands = plugin_commands(client).await?;
            Ok(serde_json::json!({"commands": commands}))
        }
        Command::Help { id } => {
            let commands = plugin_commands(client).await?;
            let found: Vec<Value> = commands
                .into_iter()
                .filter(|command| match id.as_deref() {
                    None => true,
                    Some(wanted) => command
                        .get("id")
                        .and_then(Value::as_str)
                        .map(|value| value == wanted)
                        .unwrap_or(false),
                })
                .collect();
            if found.is_empty() && id.is_some() {
                return Err(ClientError::new(
                    "invalid_params",
                    format!("unknown plugin command `{}`", id.unwrap_or_default()),
                ));
            }
            Ok(serde_json::json!({"commands": found}))
        }
        Command::Run {
            plugin_id,
            command_id,
            config,
            live,
        } => {
            let commands = plugin_commands(client).await?;
            let entry = commands.into_iter().find(|command| {
                command.get("pluginId").and_then(Value::as_str) == Some(plugin_id.as_str())
                    && command.get("id").and_then(Value::as_str) == Some(command_id.as_str())
            });
            let Some(entry) = entry else {
                return Err(ClientError::new(
                    "invalid_params",
                    format!(
                        "unknown plugin command `{plugin_id} {command_id}`; run `plugin commands`"
                    ),
                ));
            };
            let action_type = entry
                .get("actionType")
                .and_then(Value::as_str)
                .ok_or_else(|| ClientError::new("invalid_params", "command has no actionType"))?
                .to_owned();
            result_value(
                client
                    .plugins_action_execute(PluginActionParams {
                        action_type,
                        config,
                        live,
                        // Always scoped: a plugin cannot execute another plugin's action.
                        plugin_id: Some(plugin_id),
                    })
                    .await?,
            )
        }
    }
}

/// Project `pluginCliCommands` from the behavior snapshot. The host only
/// contributes commands of installed, enabled, available plugins.
async fn plugin_commands(client: &TikToolsClient) -> Result<Vec<Value>, ClientError> {
    let snapshot = client.automation_snapshot().await?;
    Ok(snapshot
        .get("pluginCliCommands")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

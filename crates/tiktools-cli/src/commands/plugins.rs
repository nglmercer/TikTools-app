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

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "plugin <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            id: one_arg(rest, "plugin get <id>")?,
        }),
        "install" => {
            let (spec, rest) = split_first(
                rest,
                "plugin install <file.plugin> [--replace] | plugin install <id[@version]|url> [--yes] [--allow-unsigned]",
            )?;
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
            id: super::args::positional(rest, 0, "plugin uninstall <id> [--purge]")?,
            purge: rest.iter().any(|arg| arg == "--purge"),
        }),
        "enable" => Ok(Command::Enable {
            id: one_arg(rest, "plugin enable <id>")?,
        }),
        "disable" => Ok(Command::Disable {
            id: one_arg(rest, "plugin disable <id>")?,
        }),
        "start" => Ok(Command::Start {
            id: one_arg(rest, "plugin start <id>")?,
        }),
        "stop" => Ok(Command::Stop {
            id: one_arg(rest, "plugin stop <id>")?,
        }),
        "settings" => {
            let (verb, rest) = split_first(rest, "plugin settings <get|set|reset> ...")?;
            match verb {
                "get" => Ok(Command::SettingsGet {
                    id: one_arg(rest, "plugin settings get <id>")?,
                }),
                "set" => {
                    let (id, pairs) = split_first(rest, "plugin settings set <id> k=v...")?;
                    Ok(Command::SettingsSet {
                        id: id.to_owned(),
                        values: kv_map(pairs)?,
                    })
                }
                "reset" => Ok(Command::SettingsReset {
                    id: one_arg(rest, "plugin settings reset <id>")?,
                }),
                other => Err(format!("unknown plugin settings verb `{other}`").into()),
            }
        }
        "health" => Ok(Command::Health {
            id: one_arg(rest, "plugin health <id>")?,
        }),
        "options" => Ok(Command::Options {
            source: one_arg(rest, "plugin options <action-type/field>")?,
        }),
        "action" => {
            let (action_type, rest) =
                split_first(rest, "plugin action <type> [--config json] [--live]")?;
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
            let (command_id, rest) =
                split_first(rest, "plugin <id> <command> [--params json] [--live]")?;
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

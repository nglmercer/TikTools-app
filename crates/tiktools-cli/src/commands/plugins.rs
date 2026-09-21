//! `plugin` commands.

use std::collections::BTreeMap;

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::plugins::{
    PluginActionParams, PluginIdParams, PluginInstallParams, PluginOptionsParams,
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
    },
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
}

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "plugin <verb> ...")?;
    match verb {
        "list" => Ok(Command::List),
        "get" => Ok(Command::Get {
            id: one_arg(rest, "plugin get <id>")?,
        }),
        "install" => {
            let (path, rest) = split_first(rest, "plugin install <path> [--replace]")?;
            Ok(Command::Install {
                path: path.to_owned(),
                replace: rest.iter().any(|arg| arg == "--replace"),
            })
        }
        "uninstall" => Ok(Command::Uninstall {
            id: one_arg(rest, "plugin uninstall <id>")?,
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
        other => Err(format!("unknown plugin verb `{other}`").into()),
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
        Command::Uninstall { id } => result_value(
            client
                .plugins_uninstall(PluginIdParams { plugin_id: id })
                .await?,
        ),
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
    }
}

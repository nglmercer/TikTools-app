//! `automation` commands.

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::modules::automation::{
    AutomationCreateParams, AutomationFireParams, AutomationGetParams, AutomationListParams,
    AutomationTestParams, AutomationUpdateParams,
};

use super::args::{flag_value, parse_json_value, positional, required_record, split_first};
use super::spec::{DomainSpec, VerbSpec};
use super::{result_value, CommandError};

pub enum Command {
    List {
        kind: String,
    },
    Context {
        event_type: Option<String>,
    },
    Get {
        id: String,
        kind: String,
    },
    Create {
        kind: String,
        record: Value,
    },
    Update {
        id: String,
        kind: String,
        record: Value,
    },
    Delete {
        id: String,
        kind: String,
    },
    Enable {
        id: String,
        kind: String,
    },
    Disable {
        id: String,
        kind: String,
    },
    Test {
        id: Option<String>,
        record: Option<Value>,
        kind: String,
        trigger: Option<String>,
        event: Option<Value>,
    },
    Fire {
        trigger: String,
        event: Option<Value>,
        record: Option<Value>,
    },
    ModerationPenalty {
        points: f64,
    },
}

const U_GET: &str = "automation get <id> [--kind event|action]";
const U_CREATE: &str = "automation create --record json [--kind event|action]";
const U_UPDATE: &str = "automation update <id> --record json [--kind event|action]";
const U_DELETE: &str = "automation delete <id> [--kind event|action]";
const U_ENABLE: &str = "automation enable <id> [--kind event|action]";
const U_DISABLE: &str = "automation disable <id> [--kind event|action]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "automation",
    summary: "event and action automation rules",
    verbs: &[
        VerbSpec::new("list", "automation list [--kind event|action|all]", "list automation rules"),
        VerbSpec::new("context", "automation context [--event-type t]", "template context available to rules"),
        VerbSpec::new("get", U_GET, "read one rule"),
        VerbSpec::new("create", U_CREATE, "create a rule"),
        VerbSpec::new("update", U_UPDATE, "update a rule"),
        VerbSpec::new("delete", U_DELETE, "delete a rule").destructive(),
        VerbSpec::new("enable", U_ENABLE, "enable a rule"),
        VerbSpec::new("disable", U_DISABLE, "disable a rule"),
        VerbSpec::new(
            "test",
            "automation test (--id <id> | --record json) [--kind event|action] [--trigger t] [--event json]",
            "dry-run a rule against a sample event",
        ),
        VerbSpec::new(
            "fire",
            "automation fire --trigger <type> [--event json] [--record json]",
            "fire a trigger manually",
        ),
        VerbSpec::new(
            "moderation-penalty",
            "automation moderation-penalty --points <delta>",
            "apply a moderation penalty in points",
        ),
    ],
};

pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (verb, rest) = split_first(args, "automation <verb> ...")?;
    let kind = || flag_value(rest, "--kind").unwrap_or_else(|| "event".to_owned());
    match verb {
        "list" => Ok(Command::List {
            kind: flag_value(rest, "--kind").unwrap_or_else(|| "all".to_owned()),
        }),
        "context" => Ok(Command::Context {
            event_type: flag_value(rest, "--event-type"),
        }),
        "get" => Ok(Command::Get {
            id: positional(rest, 0, U_GET)?,
            kind: kind(),
        }),
        "create" => Ok(Command::Create {
            kind: kind(),
            record: required_record(rest, U_CREATE)?,
        }),
        "update" => Ok(Command::Update {
            id: positional(rest, 0, U_UPDATE)?,
            kind: kind(),
            record: required_record(rest, U_UPDATE)?,
        }),
        "delete" => Ok(Command::Delete {
            id: positional(rest, 0, U_DELETE)?,
            kind: kind(),
        }),
        "enable" => Ok(Command::Enable {
            id: positional(rest, 0, U_ENABLE)?,
            kind: kind(),
        }),
        "disable" => Ok(Command::Disable {
            id: positional(rest, 0, U_DISABLE)?,
            kind: kind(),
        }),
        "test" => {
            let id = flag_value(rest, "--id");
            let record = flag_value(rest, "--record")
                .map(|raw| parse_json_value(&raw))
                .transpose()?;
            if id.is_none() && record.is_none() {
                return Err("automation test needs --id <id> or --record json"
                    .to_owned()
                    .into());
            }
            Ok(Command::Test {
                id,
                record,
                kind: kind(),
                trigger: flag_value(rest, "--trigger"),
                event: flag_value(rest, "--event")
                    .map(|raw| parse_json_value(&raw))
                    .transpose()?,
            })
        }
        "fire" => {
            let trigger = flag_value(rest, "--trigger").ok_or_else(|| {
                "automation fire needs --trigger <event-type> (e.g. --trigger tiktok.gift)"
                    .to_owned()
            })?;
            let event = flag_value(rest, "--event")
                .map(|raw| parse_json_value(&raw))
                .transpose()?;
            let record = flag_value(rest, "--record")
                .map(|raw| parse_json_value(&raw))
                .transpose()?;
            Ok(Command::Fire {
                trigger,
                event,
                record,
            })
        }
        "moderation-penalty" => {
            let raw = flag_value(rest, "--points").ok_or_else(|| {
                "automation moderation-penalty needs --points <delta> (e.g. --points -10)"
                    .to_owned()
            })?;
            let points: f64 = raw
                .parse()
                .map_err(|_| format!("--points must be a number, got `{raw}`"))?;
            tiktools_core::services::validate_penalty_points(points)?;
            Ok(Command::ModerationPenalty { points })
        }
        other => Err(format!("unknown automation verb `{other}`").into()),
    }
}

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Value, ClientError> {
    match command {
        Command::List { kind } => result_value(
            client
                .automation_list(AutomationListParams { kind: Some(kind) })
                .await?,
        ),
        Command::Context { event_type } => {
            result_value(client.automation_context_for(event_type).await?)
        }
        Command::Get { id, kind } => result_value(
            client
                .automation_get(AutomationGetParams {
                    id,
                    kind: Some(kind),
                })
                .await?,
        ),
        Command::Create { kind, record } => result_value(
            client
                .automation_create(AutomationCreateParams {
                    kind: Some(kind),
                    record,
                })
                .await?,
        ),
        Command::Update { id, kind, record } => result_value(
            client
                .automation_update(AutomationUpdateParams {
                    id,
                    kind: Some(kind),
                    record,
                })
                .await?,
        ),
        Command::Delete { id, kind } => result_value(
            client
                .automation_delete(AutomationGetParams {
                    id,
                    kind: Some(kind),
                })
                .await?,
        ),
        Command::Enable { id, kind } => result_value(
            client
                .automation_enable(AutomationGetParams {
                    id,
                    kind: Some(kind),
                })
                .await?,
        ),
        Command::Disable { id, kind } => result_value(
            client
                .automation_disable(AutomationGetParams {
                    id,
                    kind: Some(kind),
                })
                .await?,
        ),
        Command::Test {
            id,
            record,
            kind,
            trigger,
            event,
        } => result_value(
            client
                .automation_test(AutomationTestParams {
                    id,
                    record,
                    kind: Some(kind),
                    trigger,
                    event,
                })
                .await?,
        ),
        Command::Fire {
            trigger,
            event,
            record,
        } => result_value(
            client
                .automation_fire(AutomationFireParams {
                    trigger,
                    event,
                    record,
                })
                .await?,
        ),
        Command::ModerationPenalty { points } => {
            // Two typed calls sharing one validated amount: the action first
            // (the host generates its id), then the event referencing it.
            // Both records come from the core builders so the CLI can never
            // drift from the behavior-record schema.
            let action_record = tiktools_core::services::moderation_penalty_action_record(points)
                .map_err(|message| ClientError::new("invalid_params", message))?;
            let action = client
                .automation_create(AutomationCreateParams {
                    kind: Some("action".to_owned()),
                    record: action_record,
                })
                .await?;
            let action_id = action.get("id").and_then(Value::as_str).ok_or_else(|| {
                ClientError::new("protocol", "created action has no id".to_owned())
            })?;
            let event_record = tiktools_core::services::moderation_penalty_event_record(action_id)
                .map_err(|message| ClientError::new("invalid_params", message))?;
            let event = client
                .automation_create(AutomationCreateParams {
                    kind: Some("event".to_owned()),
                    record: event_record,
                })
                .await?;
            result_value(serde_json::json!({"action": action, "event": event}))
        }
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
    fn moderation_penalty_parses_points() {
        // `CommandError` has no Debug (it prints usage instead), so match
        // rather than unwrap.
        assert!(matches!(
            parse(&args(&["moderation-penalty", "--points", "-10"])),
            Ok(Command::ModerationPenalty { points }) if points == -10.0
        ));
    }

    #[test]
    fn moderation_penalty_rejects_missing_or_invalid_points() {
        assert!(parse(&args(&["moderation-penalty"])).is_err());
        assert!(parse(&args(&["moderation-penalty", "--points", "abc"])).is_err());
        assert!(parse(&args(&["moderation-penalty", "--points", "0"])).is_err());
        assert!(parse(&args(&["moderation-penalty", "--points", "nan"])).is_err());
    }
}

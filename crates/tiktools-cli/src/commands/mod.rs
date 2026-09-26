//! Command dispatch: every command resolves to one typed client call.
//!
//! Parsing is fully synchronous and runs before any client exists, so
//! usage errors (exit 2) never depend on host state. Each domain module
//! parses its argv slice into a [`Plan`] variant and later executes it
//! through [`TikToolsClient`]. The only untyped path is the raw
//! `rpc <method>` passthrough, which has no typed method by definition.

pub mod api;
pub mod args;
pub mod automation;
pub mod globals;
pub mod live;
pub mod media;
pub mod plugins;
pub mod points;
pub mod processors;
pub mod raw;
pub mod system;
pub mod template;
pub mod workflows;

use serde_json::Value;
use tiktools_client::{ClientError, TikToolsClient};

pub use raw::{serve_plan, ServeMode};

/// Command failure: `Usage` (exit 2, prints usage) or `Call` (a host or
/// transport error with the host's code, exit 1 or 3).
pub enum CommandError {
    Usage(String),
    Call(ClientError),
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self::Usage(message)
    }
}

impl From<ClientError> for CommandError {
    fn from(error: ClientError) -> Self {
        Self::Call(error)
    }
}

/// A fully parsed command: synchronous parsing validates every argument
/// before the client connects, then [`execute_plan`] performs the one
/// typed call.
pub enum Plan {
    Plugins(plugins::Command),
    Processors(processors::Command),
    Live(live::Command),
    Points(points::Command),
    Automation(automation::Command),
    Globals(globals::Command),
    Workflows(workflows::Command),
    Media(media::Command),
    System(system::Command),
    Template(template::Command),
    Api(api::Command),
    Rpc { method: String, params: Value },
}

/// Display-ready execution result: the render key for human output, the
/// value to print, an optional per-command `--format` override, and
/// whether the command already streamed its own output.
pub struct Output {
    pub command: String,
    pub value: Value,
    pub json: Option<bool>,
    pub written: bool,
    pub exit: i32,
}

impl Output {
    pub fn new(command: &str, value: Value) -> Self {
        Self {
            command: command.to_owned(),
            value,
            json: None,
            written: false,
            exit: 0,
        }
    }

    pub fn json_override(mut self, json: Option<bool>) -> Self {
        self.json = json;
        self
    }

    /// Overrides the success exit code (verification reports failure
    /// through the report while still printing it).
    pub fn exit_code(mut self, exit: i32) -> Self {
        self.exit = exit;
        self
    }

    /// Streaming commands (`api events`) print as they go; the dispatcher
    /// must not print anything afterwards.
    pub fn written() -> Self {
        Self {
            command: String::new(),
            value: Value::Null,
            json: None,
            written: true,
            exit: 0,
        }
    }
}

/// Redacts secrets from agent-facing output unless `--secrets-visible`
/// opts out.
pub fn redact_output(value: Value, secrets_visible: bool) -> Value {
    if secrets_visible {
        value
    } else {
        crate::output::redact_secrets(value)
    }
}

/// Parses argv into a plan. Never touches the network or the host.
pub fn parse_command(args: &[String]) -> Result<Plan, CommandError> {
    let head = args[0].as_str();
    let rest = &args[1..];
    let plan = match head {
        "plugin" => Plan::Plugins(plugins::parse(rest)?),
        "processor" | "processors" => Plan::Processors(processors::parse(rest)?),
        "live" => Plan::Live(live::parse(rest)?),
        "points" => Plan::Points(points::parse(rest)?),
        "automation" => Plan::Automation(automation::parse(rest)?),
        "globals" => Plan::Globals(globals::parse(rest)?),
        "workflow" => Plan::Workflows(workflows::parse(rest)?),
        "media" => Plan::Media(media::parse(rest)?),
        "system" => Plan::System(system::parse(rest)?),
        "template" => Plan::Template(template::parse(rest)?),
        "api" => Plan::Api(api::parse(rest)?),
        "rpc" => raw::parse_rpc(rest)?,
        // `serve_plan` handles every `host` form before parsing commands;
        // reaching here means host was misrouted, so report its usage.
        "host" => return Err("host --stdio [--events] | host --ipc".to_owned().into()),
        other => return Err(format!("unknown command `{other}`").into()),
    };
    Ok(plan)
}

/// Executes a parsed plan through one typed client call.
pub async fn execute_plan(client: &TikToolsClient, plan: Plan) -> Result<Output, ClientError> {
    match plan {
        Plan::Plugins(command) => Ok(Output::new(
            "plugin",
            plugins::execute(client, command).await?,
        )),
        Plan::Processors(command) => Ok(Output::new(
            "processor",
            processors::execute(client, command).await?,
        )),
        Plan::Live(command) => Ok(Output::new("live", live::execute(client, command).await?)),
        Plan::Points(command) => Ok(Output::new(
            "points",
            points::execute(client, command).await?,
        )),
        Plan::Automation(command) => Ok(Output::new(
            "automation",
            automation::execute(client, command).await?,
        )),
        Plan::Globals(command) => Ok(Output::new(
            "globals",
            globals::execute(client, command).await?,
        )),
        Plan::Workflows(command) => Ok(Output::new(
            "workflow",
            workflows::execute(client, command).await?,
        )),
        Plan::Media(command) => Ok(Output::new("media", media::execute(client, command).await?)),
        Plan::System(command) => Ok(Output::new(
            "system",
            system::execute(client, command).await?,
        )),
        Plan::Template(command) => Ok(Output::new(
            "template",
            template::execute(client, command).await?,
        )),
        Plan::Api(command) => api::execute(client, command).await,
        Plan::Rpc { method, params } => Ok(Output::new(
            "rpc",
            client.call_value(&method, params).await?,
        )),
    }
}

/// Maps a typed result back to JSON for the output layer. Serialization
/// of a typed result is infallible in practice; a failure surfaces as a
/// protocol error like a malformed host result would.
pub fn result_value(result: impl serde::Serialize) -> Result<Value, ClientError> {
    serde_json::to_value(result)
        .map_err(|error| ClientError::new("protocol", format!("bad result shape: {error}")))
}

/// Builds typed params that deserialize from a JSON value (k=v objects).
/// A mismatch means the CLI arguments do not fit the method's params, the
/// same `invalid_params` failure the host returned when the CLI passed
/// raw JSON through.
pub fn typed_params<T>(params: Value) -> Result<T, CommandError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(params)
        .map_err(|error| ClientError::new("invalid_params", error.to_string()).into())
}

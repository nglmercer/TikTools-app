//! `tiktools`: thin CLI over the shared control plane.
//!
//! Every command builds the same JSON-RPC request a stdio/IPC/WebView client
//! would send and executes it against the running host over local IPC, so
//! the CLI always observes the desktop's one [`AppCore`]. `--standalone`
//! opts into an isolated in-process [`AppCore`] instead.
//! JSON mode rules: stdout carries JSON only, diagnostics go to stderr,
//! no ANSI, no progress bars, stable exit codes (0 ok, 1 operation error,
//! 2 usage, 3 transport failure).

use std::sync::Arc;

use serde_json::{json, Value};
use tiktools_control_api::{ControlApi, ControlClient, RpcId, RpcRequest};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct NullEmitter;

impl HostEmitter for NullEmitter {
    fn emit(&self, _message: HostMessage) {}
}

fn headless_api() -> ControlApi {
    ControlApi::new(Arc::new(AppCore::new(Arc::new(NullEmitter))))
}

const USAGE: &str = "\
usage: tiktools [--json] [--standalone] <command> [args]

commands:
  plugin list | get <id> | install <path> [--replace] | uninstall <id>
         | enable <id> | disable <id> | start <id> | stop <id>
         | settings get <id> | settings set <id> k=v... | settings reset <id>
         | health <id> | options <action-type/field> | action <type> [--config json] [--live]
  processor list | status | test <plugin-id> <processor-id> [event-json]
  live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]
       | pick --session-cookie <cookie> | disconnect | status
  points config get | config set k=v... | viewer get <id> | adjust <id> <delta>
       | leaderboard [--limit N] | reset [id]
  automation list [--kind event|action|all] | get <id> [--kind k]
           | create --record json [--kind k] | update <id> --record json [--kind k]
           | delete <id> [--kind k] | enable <id> [--kind k] | disable <id> [--kind k]
           | test (--id <id> | --record json) [--kind k] [--trigger t]
           | context
  workflow list | get <id> | save --record json | delete <id>
           | enable <id> | disable <id>
  media validate <path> [--kind audio|video|image|other]
      | play <path> [--kind k] [--volume 0..1]
  system info | health | snapshot | doctor
  rpc <method> [params-json]
  rpc --stdio [--events]        NDJSON request/response loop on stdio
  host --stdio [--events]       headless host on stdio (streams events with --events)
  host --ipc                    headless host on local IPC (refuses when a host is running)

commands run against the running host over local IPC by default; when no
host is running they fail with host_unavailable instead of starting a
second runtime. --standalone runs the command against an isolated
in-process runtime instead. json values may use @path to read from a file.
--json prints the raw result object to stdout; without it, lists print one
line per item.
";

fn main() {
    let code = run();
    std::process::exit(code);
}

fn run() -> i32 {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let json_mode = raw.iter().any(|arg| arg == "--json");
    let standalone = raw.iter().any(|arg| arg == "--standalone");
    let args: Vec<String> = raw
        .into_iter()
        .filter(|arg| arg != "--json" && arg != "--standalone")
        .collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" || args[0] == "help" {
        print!("{USAGE}");
        return 0;
    }
    if args[0] == "--version" || args[0] == "-V" || args[0] == "version" {
        println!("tiktools {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("tiktools: could not start async runtime: {error}");
            return 3;
        }
    };
    runtime.block_on(dispatch(args, json_mode, standalone))
}

async fn dispatch(args: Vec<String>, json_mode: bool, standalone: bool) -> i32 {
    let request = match build_request(&args) {
        Ok(Request::Rpc(request)) => request,
        Ok(Request::Stdio { events }) => {
            return serve_stdio(headless_api(), events).await;
        }
        Ok(Request::Ipc) => {
            return serve_ipc_guarded().await;
        }
        Err(message) => {
            eprintln!("tiktools: {message}\n\n{USAGE}");
            return 2;
        }
    };
    if standalone {
        return execute_local(headless_api(), request, json_mode, &args[0]).await;
    }
    execute_remote(request, json_mode, &args[0]).await
}

async fn execute_local(
    api: ControlApi,
    request: RpcRequest,
    json_mode: bool,
    command: &str,
) -> i32 {
    let response = api.execute(request).await;
    if !response.is_ok() {
        let error = response
            .error
            .as_ref()
            .map(|error| error.code.as_str())
            .unwrap_or("error");
        let message = response
            .error
            .as_ref()
            .map(|error| error.message.as_str())
            .unwrap_or("request failed");
        if json_mode {
            println!("{}", serde_json::to_string(&response).unwrap_or_default());
        } else {
            eprintln!("tiktools: [{error}] {message}");
        }
        return 1;
    }
    let result = response.result.unwrap_or(Value::Null);
    if json_mode {
        println!("{}", serde_json::to_string(&result).unwrap_or_default());
    } else {
        print_human(command, result);
    }
    0
}

/// Default path: executes against the running host over local IPC. A
/// missing host is a transport failure (`host_unavailable`, exit 3), never
/// a silent second runtime.
async fn execute_remote(request: RpcRequest, json_mode: bool, command: &str) -> i32 {
    let client = match ControlClient::connect().await {
        Ok(client) => client,
        Err(error) => {
            print_error(json_mode, &error.code, &error.message);
            return 3;
        }
    };
    match client.call_value(&request.method, request.params).await {
        Ok(result) => {
            if json_mode {
                println!("{}", serde_json::to_string(&result).unwrap_or_default());
            } else {
                print_human(command, result);
            }
            0
        }
        Err(error) => {
            // Host operation errors keep the host's code; a dropped
            // connection mid-call is a transport failure.
            let exit = if error.code == "transport" || error.code == "protocol" {
                3
            } else {
                1
            };
            print_error(json_mode, &error.code, &error.message);
            exit
        }
    }
}

fn print_error(json_mode: bool, code: &str, message: &str) {
    if json_mode {
        println!(
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": {"code": code, "message": message},
            })
        );
    } else {
        eprintln!("tiktools: [{code}] {message}");
    }
}

/// `host --ipc` acquires the OS control-host ownership primitive. A connect
/// probe is not a lock (it races startup); ownership failure is the single
/// authority for "another host is running".
async fn serve_ipc_guarded() -> i32 {
    serve_ipc(headless_api()).await
}

async fn serve_stdio(api: ControlApi, events: bool) -> i32 {
    // Headless hosts own the plugin lifecycle exactly like the desktop:
    // polling starts with the host, never with a UI. Idempotent.
    api.core()
        .spawn_plugin_event_poll(&tokio::runtime::Handle::current());
    if let Err(error) = tiktools_control_api::run_stdio(api, events).await {
        eprintln!("tiktools: stdio host failed: {error}");
        return 3;
    }
    0
}

async fn serve_ipc(api: ControlApi) -> i32 {
    // Headless hosts own the plugin lifecycle exactly like the desktop:
    // polling starts with the host, never with a UI. Idempotent.
    api.core()
        .spawn_plugin_event_poll(&tokio::runtime::Handle::current());
    if let Err(error) = tiktools_control_api::run_ipc(api).await {
        if error.kind() == std::io::ErrorKind::AddrInUse {
            eprintln!("tiktools: a control host is already running on the local IPC endpoint");
            return 1;
        }
        eprintln!("tiktools: IPC host failed: {error}");
        return 3;
    }
    0
}

enum Request {
    Rpc(RpcRequest),
    Stdio { events: bool },
    Ipc,
}

fn rpc(method: &str, params: Value) -> Request {
    Request::Rpc(RpcRequest::new(RpcId::Number(1), method, params))
}

fn build_request(args: &[String]) -> Result<Request, String> {
    let head = args[0].as_str();
    let rest = &args[1..];
    match head {
        "plugin" => plugin_request(rest),
        "processor" | "processors" => processor_request(rest),
        "live" => live_request(rest),
        "points" => points_request(rest),
        "automation" => automation_request(rest),
        "workflow" => workflow_request(rest),
        "media" => media_request(rest),
        "system" => system_request(rest),
        "rpc" => rpc_request(rest),
        "host" => host_request(rest),
        other => Err(format!("unknown command `{other}`")),
    }
}

// ------------------------------------------------------------------
// plugins
// ------------------------------------------------------------------

fn plugin_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "plugin <verb> ...")?;
    match verb {
        "list" => Ok(rpc("plugins.list", json!({}))),
        "get" => {
            let id = one_arg(rest, "plugin get <id>")?;
            Ok(rpc("plugins.get", json!({ "pluginId": id })))
        }
        "install" => {
            let (path, rest) = split_first(rest, "plugin install <path> [--replace]")?;
            let replace = rest.iter().any(|arg| arg == "--replace");
            Ok(rpc(
                "plugins.install",
                json!({ "path": path, "replaceExisting": replace }),
            ))
        }
        "uninstall" => {
            let id = one_arg(rest, "plugin uninstall <id>")?;
            Ok(rpc("plugins.uninstall", json!({ "pluginId": id })))
        }
        "enable" => {
            let id = one_arg(rest, "plugin enable <id>")?;
            Ok(rpc("plugins.enable", json!({ "pluginId": id })))
        }
        "disable" => {
            let id = one_arg(rest, "plugin disable <id>")?;
            Ok(rpc("plugins.disable", json!({ "pluginId": id })))
        }
        "start" => {
            let id = one_arg(rest, "plugin start <id>")?;
            Ok(rpc("plugins.start", json!({ "pluginId": id })))
        }
        "stop" => {
            let id = one_arg(rest, "plugin stop <id>")?;
            Ok(rpc("plugins.stop", json!({ "pluginId": id })))
        }
        "settings" => {
            let (verb, rest) = split_first(rest, "plugin settings <get|set|reset> ...")?;
            match verb {
                "get" => {
                    let id = one_arg(rest, "plugin settings get <id>")?;
                    Ok(rpc("plugins.settings.get", json!({ "pluginId": id })))
                }
                "set" => {
                    let (id, pairs) = split_first(rest, "plugin settings set <id> k=v...")?;
                    Ok(rpc(
                        "plugins.settings.set",
                        json!({ "pluginId": id, "values": kv_object(pairs)? }),
                    ))
                }
                "reset" => {
                    let id = one_arg(rest, "plugin settings reset <id>")?;
                    Ok(rpc("plugins.settings.reset", json!({ "pluginId": id })))
                }
                other => Err(format!("unknown plugin settings verb `{other}`")),
            }
        }
        "health" => {
            let id = one_arg(rest, "plugin health <id>")?;
            Ok(rpc("plugins.health", json!({ "pluginId": id })))
        }
        "options" => {
            let source = one_arg(rest, "plugin options <action-type/field>")?;
            Ok(rpc("plugins.options", json!({ "source": source })))
        }
        "action" => {
            let (action_type, rest) =
                split_first(rest, "plugin action <type> [--config json] [--live]")?;
            let config = flag_value(rest, "--config")
                .map(|raw| parse_json_value(&raw))
                .transpose()?
                .unwrap_or(json!({}));
            if !config.is_object() {
                return Err("--config must be a JSON object".to_owned());
            }
            let live = rest.iter().any(|arg| arg == "--live");
            Ok(rpc(
                "plugins.action.execute",
                json!({ "actionType": action_type, "config": config, "live": live }),
            ))
        }
        other => Err(format!("unknown plugin verb `{other}`")),
    }
}

// ------------------------------------------------------------------
// processors / live / media / system
// ------------------------------------------------------------------

fn processor_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "processor <verb> ...")?;
    match verb {
        "list" => Ok(rpc("processors.list", json!({}))),
        "status" => Ok(rpc("processors.status", json!({}))),
        "test" => {
            if rest.len() < 2 {
                return Err("processor test <plugin-id> <processor-id> [event-json]".to_owned());
            }
            let event = rest
                .get(2)
                .map(|raw| parse_json_value(raw))
                .transpose()?
                .unwrap_or(json!({}));
            Ok(rpc(
                "processors.test",
                json!({ "pluginId": rest[0], "processorId": rest[1], "event": event }),
            ))
        }
        other => Err(format!("unknown processor verb `{other}`")),
    }
}

fn live_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "live <verb> ...")?;
    match verb {
        "connect" => {
            let unique_id = flag_value(rest, "--unique-id")
                .ok_or_else(|| "live connect needs --unique-id <id>".to_owned())?;
            let session_cookie = flag_value(rest, "--session-cookie")
                .ok_or_else(|| "live connect needs --session-cookie <cookie>".to_owned())?;
            let room_id = flag_value(rest, "--room-id")
                .map(Value::String)
                .unwrap_or(Value::Null);
            Ok(rpc(
                "live.connect",
                json!({ "uniqueId": unique_id, "sessionCookie": session_cookie, "roomId": room_id }),
            ))
        }
        "pick" => {
            let session_cookie = flag_value(rest, "--session-cookie")
                .ok_or_else(|| "live pick needs --session-cookie <cookie>".to_owned())?;
            Ok(rpc("live.pick", json!({ "sessionCookie": session_cookie })))
        }
        "disconnect" => Ok(rpc("live.disconnect", json!({}))),
        "status" => Ok(rpc("live.status", json!({}))),
        other => Err(format!("unknown live verb `{other}`")),
    }
}

fn media_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "media <verb> ...")?;
    match verb {
        "validate" => {
            let (path, rest) = split_first(rest, "media validate <path> [--kind k]")?;
            let kind = flag_value(rest, "--kind")
                .map(Value::String)
                .unwrap_or(Value::Null);
            Ok(rpc("media.validate", json!({ "path": path, "kind": kind })))
        }
        "play" => {
            let (path, rest) = split_first(rest, "media play <path> [--kind k] [--volume v]")?;
            let kind = flag_value(rest, "--kind")
                .map(Value::String)
                .unwrap_or(Value::Null);
            let volume = flag_value(rest, "--volume")
                .map(|raw| {
                    raw.parse::<f64>()
                        .map(Value::from)
                        .map_err(|_| "--volume must be a number within 0.0..=1.0".to_owned())
                })
                .transpose()?
                .unwrap_or(Value::Null);
            Ok(rpc(
                "media.play",
                json!({ "path": path, "kind": kind, "volume": volume }),
            ))
        }
        other => Err(format!("unknown media verb `{other}`")),
    }
}

fn system_request(args: &[String]) -> Result<Request, String> {
    let (verb, _) = split_first(args, "system <verb> ...")?;
    match verb {
        "info" => Ok(rpc("system.info", json!({}))),
        "health" => Ok(rpc("system.health", json!({}))),
        "snapshot" => Ok(rpc("system.snapshot", json!({}))),
        "doctor" => Ok(rpc("system.doctor", json!({}))),
        other => Err(format!("unknown system verb `{other}`")),
    }
}

// ------------------------------------------------------------------
// points
// ------------------------------------------------------------------

fn points_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "points <verb> ...")?;
    match verb {
        "config" => {
            let (verb, rest) = split_first(rest, "points config <get|set> ...")?;
            match verb {
                "get" => Ok(rpc("points.config.get", json!({}))),
                "set" => Ok(rpc("points.config.set", kv_object(rest)?)),
                other => Err(format!("unknown points config verb `{other}`")),
            }
        }
        "viewer" => {
            let (verb, rest) = split_first(rest, "points viewer get <id>")?;
            if verb != "get" {
                return Err(format!("unknown points viewer verb `{verb}`"));
            }
            let id = one_arg(rest, "points viewer get <id>")?;
            Ok(rpc("points.viewer.get", json!({ "uniqueId": id })))
        }
        "adjust" => {
            if rest.len() != 2 {
                return Err("points adjust <id> <delta>".to_owned());
            }
            let delta: f64 = rest[1]
                .parse()
                .map_err(|_| "delta must be a number".to_owned())?;
            Ok(rpc(
                "points.adjust",
                json!({ "uniqueId": rest[0], "delta": delta }),
            ))
        }
        "leaderboard" => {
            let limit = flag_value(rest, "--limit")
                .map(|raw| {
                    raw.parse::<i64>()
                        .map(Value::from)
                        .map_err(|_| "--limit must be an integer".to_owned())
                })
                .transpose()?
                .unwrap_or(Value::Null);
            Ok(rpc("points.leaderboard", json!({ "limit": limit })))
        }
        "reset" => {
            if rest.len() > 1 {
                return Err("points reset [id]".to_owned());
            }
            let unique_id = rest
                .first()
                .cloned()
                .map(Value::String)
                .unwrap_or(Value::Null);
            Ok(rpc("points.reset", json!({ "uniqueId": unique_id })))
        }
        other => Err(format!("unknown points verb `{other}`")),
    }
}

// ------------------------------------------------------------------
// automation
// ------------------------------------------------------------------

fn automation_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "automation <verb> ...")?;
    let kind = || flag_value(rest, "--kind").unwrap_or_else(|| "event".to_owned());
    match verb {
        "list" => {
            let kind = flag_value(rest, "--kind").unwrap_or_else(|| "all".to_owned());
            Ok(rpc("automation.list", json!({ "kind": kind })))
        }
        "context" => Ok(rpc("automation.context", json!({}))),
        "get" => {
            let id = positional(rest, 0, "automation get <id> [--kind k]")?;
            Ok(rpc("automation.get", json!({ "id": id, "kind": kind() })))
        }
        "create" => {
            let record = required_record(rest, "automation create --record json [--kind k]")?;
            Ok(rpc(
                "automation.create",
                json!({ "kind": kind(), "record": record }),
            ))
        }
        "update" => {
            let id = positional(rest, 0, "automation update <id> --record json [--kind k]")?;
            let record = required_record(rest, "automation update <id> --record json [--kind k]")?;
            Ok(rpc(
                "automation.update",
                json!({ "id": id, "kind": kind(), "record": record }),
            ))
        }
        "delete" => {
            let id = positional(rest, 0, "automation delete <id> [--kind k]")?;
            Ok(rpc(
                "automation.delete",
                json!({ "id": id, "kind": kind() }),
            ))
        }
        "enable" => {
            let id = positional(rest, 0, "automation enable <id> [--kind k]")?;
            Ok(rpc(
                "automation.enable",
                json!({ "id": id, "kind": kind() }),
            ))
        }
        "disable" => {
            let id = positional(rest, 0, "automation disable <id> [--kind k]")?;
            Ok(rpc(
                "automation.disable",
                json!({ "id": id, "kind": kind() }),
            ))
        }
        "test" => {
            let id = flag_value(rest, "--id")
                .map(Value::String)
                .unwrap_or(Value::Null);
            let record = flag_value(rest, "--record")
                .map(|raw| parse_json_value(&raw))
                .transpose()?
                .unwrap_or(Value::Null);
            if id.is_null() && record.is_null() {
                return Err("automation test needs --id <id> or --record json".to_owned());
            }
            let trigger = flag_value(rest, "--trigger")
                .map(Value::String)
                .unwrap_or(Value::Null);
            Ok(rpc(
                "automation.test",
                json!({ "id": id, "record": record, "kind": kind(), "trigger": trigger }),
            ))
        }
        other => Err(format!("unknown automation verb `{other}`")),
    }
}

// ------------------------------------------------------------------
// graph workflows
// ------------------------------------------------------------------

fn workflow_request(args: &[String]) -> Result<Request, String> {
    let (verb, rest) = split_first(args, "workflow <verb> ...")?;
    match verb {
        "list" => Ok(rpc("workflows.list", json!({}))),
        "get" => {
            let id = one_arg(rest, "workflow get <id>")?;
            Ok(rpc("workflows.get", json!({ "id": id })))
        }
        "save" => {
            let graph = required_record(rest, "workflow save --record json")?;
            Ok(rpc("workflows.save", json!({ "graph": graph })))
        }
        "delete" => {
            let id = one_arg(rest, "workflow delete <id>")?;
            Ok(rpc("workflows.delete", json!({ "id": id })))
        }
        "enable" => {
            let id = one_arg(rest, "workflow enable <id>")?;
            Ok(rpc("workflows.enable", json!({ "id": id })))
        }
        "disable" => {
            let id = one_arg(rest, "workflow disable <id>")?;
            Ok(rpc("workflows.disable", json!({ "id": id })))
        }
        other => Err(format!("unknown workflow verb `{other}`")),
    }
}

// ------------------------------------------------------------------
// raw rpc / host
// ------------------------------------------------------------------

fn rpc_request(args: &[String]) -> Result<Request, String> {
    if args.first().map(String::as_str) == Some("--stdio") {
        let events = args.iter().any(|arg| arg == "--events");
        return Ok(Request::Stdio { events });
    }
    let (method, rest) = split_first(args, "rpc <method> [params-json] | rpc --stdio [--events]")?;
    let params = rest
        .first()
        .map(|raw| parse_json_value(raw))
        .transpose()?
        .unwrap_or(json!({}));
    Ok(rpc(method, params))
}

fn host_request(args: &[String]) -> Result<Request, String> {
    let (mode, rest) = split_first(args, "host --stdio [--events] | host --ipc")?;
    match mode {
        "--stdio" => Ok(Request::Stdio {
            events: rest.iter().any(|arg| arg == "--events"),
        }),
        "--ipc" => Ok(Request::Ipc),
        other => Err(format!("unknown host mode `{other}`")),
    }
}

// ------------------------------------------------------------------
// argument helpers
// ------------------------------------------------------------------

fn split_first<'a>(args: &'a [String], usage: &str) -> Result<(&'a str, &'a [String]), String> {
    args.split_first()
        .map(|(first, rest)| (first.as_str(), rest))
        .ok_or_else(|| usage.to_owned())
}

fn one_arg(args: &[String], usage: &str) -> Result<String, String> {
    if args.len() != 1 {
        return Err(usage.to_owned());
    }
    Ok(args[0].clone())
}

fn positional(args: &[String], index: usize, usage: &str) -> Result<String, String> {
    args.iter()
        .filter(|arg| !arg.starts_with("--"))
        .nth(index)
        .cloned()
        .ok_or_else(|| usage.to_owned())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn required_record(args: &[String], usage: &str) -> Result<Value, String> {
    match flag_value(args, "--record") {
        Some(raw) => parse_json_value(&raw),
        None => Err(usage.to_owned()),
    }
}

/// Parses inline JSON or `@path` file JSON.
fn parse_json_value(raw: &str) -> Result<Value, String> {
    if let Some(path) = raw.strip_prefix('@') {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("could not read {path}: {error}"))?;
        serde_json::from_str(&text).map_err(|error| format!("{path} is not valid JSON: {error}"))
    } else {
        serde_json::from_str(raw).map_err(|error| format!("invalid JSON: {error}"))
    }
}

/// Parses `k=v` pairs; each value is JSON-decoded with a string fallback so
/// `port=8080` becomes a number while `url=http://x` stays a string.
fn kv_object(pairs: &[String]) -> Result<Value, String> {
    let mut object = serde_json::Map::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("expected k=v, got `{pair}`"))?;
        if key.trim().is_empty() {
            return Err(format!("expected k=v, got `{pair}`"));
        }
        let value: Value =
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_owned()));
        object.insert(key.to_owned(), value);
    }
    Ok(Value::Object(object))
}

// ------------------------------------------------------------------
// human output (one line per item for lists; pretty JSON otherwise)
// ------------------------------------------------------------------

fn print_human(command: &str, result: Value) {
    match command {
        "plugin" => {
            if let Some(plugins) = result.get("plugins").and_then(Value::as_array) {
                if plugins.is_empty() {
                    println!("no plugins installed");
                    return;
                }
                for plugin in plugins {
                    let descriptor = plugin.get("descriptor");
                    let id = descriptor
                        .and_then(|descriptor| descriptor.get("id"))
                        .and_then(Value::as_str)
                        .or_else(|| plugin.get("id").and_then(Value::as_str))
                        .unwrap_or("?");
                    let version = descriptor
                        .and_then(|descriptor| descriptor.get("version"))
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let mut flags = Vec::new();
                    for (key, label) in [
                        ("installed", "installed"),
                        ("enabled", "enabled"),
                        ("running", "running"),
                        ("available", "available"),
                    ] {
                        if plugin.get(key).and_then(Value::as_bool) == Some(true) {
                            flags.push(label);
                        }
                    }
                    println!("{id} {version} [{}]", flags.join(","));
                }
                return;
            }
            print_json(&result);
        }
        "automation" => {
            let events = result
                .get("events")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let actions = result
                .get("actions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if result.get("events").is_some() || result.get("actions").is_some() {
                for record in events.iter().chain(actions.iter()) {
                    let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
                    let name = record.get("name").and_then(Value::as_str).unwrap_or("?");
                    let enabled = record.get("enabled").and_then(Value::as_bool) == Some(true);
                    println!(
                        "{} {} [{}]",
                        id,
                        name,
                        if enabled { "enabled" } else { "disabled" }
                    );
                }
                if events.is_empty() && actions.is_empty() {
                    println!("no automations");
                }
                return;
            }
            print_json(&result);
        }
        "points" => {
            if let Some(viewers) = result.get("viewers").and_then(Value::as_array) {
                if viewers.is_empty() {
                    println!("no viewers yet");
                    return;
                }
                for viewer in viewers {
                    let id = viewer
                        .get("uniqueId")
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let points = viewer
                        .get("points")
                        .and_then(Value::as_f64)
                        .unwrap_or_default();
                    let level = viewer.get("level").and_then(Value::as_u64).unwrap_or(1);
                    println!("{id}: {points} (level {level})");
                }
                return;
            }
            print_json(&result);
        }
        "system" => {
            if let Some(checks) = result.get("checks").and_then(Value::as_array) {
                println!(
                    "doctor ok: {}",
                    result.get("ok").and_then(Value::as_bool) == Some(true)
                );
                for check in checks {
                    let id = check.get("id").and_then(Value::as_str).unwrap_or("?");
                    let status = check.get("status").and_then(Value::as_str).unwrap_or("?");
                    let message = check.get("message").and_then(Value::as_str).unwrap_or("");
                    if message.is_empty() {
                        println!("  [{status}] {id}");
                    } else {
                        println!("  [{status}] {id}: {message}");
                    }
                }
                return;
            }
            print_json(&result);
        }
        "workflow" => {
            if let Some(workflows) = result.get("workflows").and_then(Value::as_array) {
                if workflows.is_empty() {
                    println!("no workflows");
                    return;
                }
                for workflow in workflows {
                    let id = workflow.get("id").and_then(Value::as_str).unwrap_or("?");
                    let name = workflow.get("name").and_then(Value::as_str).unwrap_or("?");
                    let enabled = workflow.get("enabled").and_then(Value::as_bool) == Some(true);
                    println!(
                        "{} {} [{}]",
                        id,
                        name,
                        if enabled { "enabled" } else { "disabled" }
                    );
                }
                return;
            }
            print_json(&result);
        }
        _ => print_json(&result),
    }
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

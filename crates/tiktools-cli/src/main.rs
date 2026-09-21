//! `tiktools`: thin CLI over the shared control plane.
//!
//! Every command resolves to one typed [`TikToolsClient`] call and executes
//! it against the running host over local IPC, so the CLI always observes
//! the desktop's one [`AppCore`]. `--standalone` opts into an isolated
//! in-process [`AppCore`] instead.
//! JSON mode rules: stdout carries JSON only, diagnostics go to stderr,
//! no ANSI, no progress bars, stable exit codes (0 ok, 1 operation error,
//! 2 usage, 3 transport failure).
//!
//! [`TikToolsClient`]: tiktools_client::TikToolsClient
//! [`AppCore`]: tiktools_core::AppCore

mod client;
mod commands;
mod host;
mod output;

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
    match commands::serve_plan(&args) {
        Ok(Some(commands::ServeMode::Stdio { events })) => {
            return host::serve_stdio(client::headless_api(), events).await;
        }
        Ok(Some(commands::ServeMode::Ipc)) => {
            return host::serve_ipc_guarded().await;
        }
        Ok(None) => {}
        Err(message) => {
            eprintln!("tiktools: {message}\n\n{USAGE}");
            return 2;
        }
    }
    // Parse before connecting: usage errors never depend on host state.
    let (command, plan) = match commands::parse_command(&args) {
        Ok(parsed) => parsed,
        Err(commands::CommandError::Usage(message)) => {
            eprintln!("tiktools: {message}\n\n{USAGE}");
            return 2;
        }
        Err(commands::CommandError::Call(error)) => {
            let exit = client::exit_code_for(&error);
            client::print_error(json_mode, &error.code, &error.message);
            return exit;
        }
    };
    let resolved = match client::resolve_client(standalone).await {
        Ok(resolved) => resolved,
        Err(error) => {
            client::print_error(json_mode, &error.code, &error.message);
            return 3;
        }
    };
    match commands::execute_plan(&resolved, plan).await {
        Ok(result) => {
            if json_mode {
                println!("{}", serde_json::to_string(&result).unwrap_or_default());
            } else {
                output::print_human(&command, result);
            }
            0
        }
        Err(error) => {
            let exit = client::exit_code_for(&error);
            client::print_error(json_mode, &error.code, &error.message);
            exit
        }
    }
}

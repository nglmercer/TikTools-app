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
#[cfg(feature = "local-web")]
mod web;

const USAGE: &str = "\
usage: tiktools [--json] [--standalone] <command> [args]

commands:
         | connect --profile NAME --host ID [--kind desktop|agent]
         | pair --profile NAME [--kind desktop|agent] [--label NAME] [--invite-stdin]
  plugin list | get <id> | install <file.plugin> [--replace] | uninstall <id> [--purge]
         | install <id[@version]> [--yes] [--no-wait]
         | search <query> [--category c] [--page N] | inspect <spec>
         | outdated | update <id> [--version v] [--yes] | update --all [--yes]
         | rollback <id> [version] | versions <id> | pin <id> | unpin <id>
         | policy <id> [--channel stable|beta] [--auto off|notify|patch|minor]
         | jobs | job <job-id> | cancel <job-id>
         | enable <id> | disable <id> | start <id> | stop <id>
         | settings get <id> | settings set <id> k=v... | settings reset <id>
         | health <id> | options <action-type/field> | action <type> [--config json] [--live]
  registry list | refresh | package <id> | defaults
  processor list | status | test <plugin-id> <processor-id> [event-json]
  live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]
       | pick --session-cookie <cookie> | disconnect | status
       | channels [--keyword <text>] [--offset N] [--limit N] [--session-cookie <cookie>]
  points config get | config set k=v... | viewer get <id> | adjust <id> <delta>
       | leaderboard [--limit N] | reset [id]
  globals list | get <key> | set <key> <value> | delete <key>
  automation list [--kind event|action|all] | get <id> [--kind k]
           | create --record json [--kind k] | update <id> --record json [--kind k]
           | delete <id> [--kind k] | enable <id> [--kind k] | disable <id> [--kind k]
           | test (--id <id> | --record json) [--kind k] [--trigger t] [--event json]
           | fire --trigger <type> [--event json] [--record json]
           | moderation-penalty --points <delta>
           | context
  workflow list | get <id> | save --record json | delete <id>
           | enable <id> | disable <id>
  media validate <path> [--kind audio|video|image|other]
      | play <path> [--kind k] [--volume 0..1]
  system info | health | snapshot | doctor
  template import <file> [--param k=v]... [--event-name n] [--action-name n]
           [--dry-run] [--no-save] [--disabled]
         | export --event <id> --out <file> [--title t] [--id template-id]
         | list | delete <template-id>
         | profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]
         | profile-export --out <file> [--name n]
  api discover [--format json|human]
    | schema <method> [--format json|human]
    | call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm]
    |      [--redact|--secrets-visible] [--explain] [--format json|human]
    |      [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]
    | events [--topics t,...] [--timeout SEC] [--max-events N]
    | verify [--coverage|--smoke] [--sandbox] [--timeout SEC]
  rpc <method> [params-json]
  rpc --stdio [--events]        NDJSON request/response loop on stdio
         cloud enroll [--label NAME] | join [--invite-stdin]
         cloud connect --host ID --profile NAME [--kind desktop|agent]
         cloud pair --profile NAME [--label NAME] [--kind desktop|agent] [--invite-stdin]
                                Native Cloud P2P onboarding; no local host runtime
  cloud status | login | logout | restore
                                Cloud account (official Cloud; override with TIKTOOLS_CLOUD_ORIGIN)
  publisher list | create <namespace> | delete <namespace-or-publisher-id> | keys <publisher-id>
            | enroll <publisher-id> [--key-id <id>] | upload <publisher-id> <path>
            | submit <upload-id> <key-id> <plugin-id> <version> [--target any]
            | release <release-id> | publish <release-id>
                                Cloud publisher flow (requires sign-in)
  host --stdio [--events]       headless host on stdio (streams events with --events)
  host --ipc                    headless host on local IPC (refuses when a host is running)
  web --assets <directory> [--port N]  local browser adapter (requires local-web feature)
  web --print-code [--port N]       print the active pairing code without restarting

commands run against the running host over local IPC by default; when no
host is running they fail with host_unavailable instead of starting a
second runtime. --standalone runs the command against an isolated
in-process runtime instead. json values may use @path to read from a file.
back to a local runtime. It cannot be combined with --standalone, serving,
privately; no invitation or credential secret is accepted as a process argument.
--json prints the raw result object to stdout; without it, lists print one
line per item.
live connect/pick read the session cookie from --session-cookie or the
TIKTOOLS_SESSION_COOKIE environment variable (the flag wins); the value
never appears in diagnostics. api call redacts secrets from its output
unless --secrets-visible is passed, and refuses destructive methods
without --confirm.
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
    if let Some(error) = selection_error(&args, standalone) {
        eprintln!("tiktools: {error}");
        return 2;
    }
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
    let code = runtime.block_on(dispatch(args, json_mode, standalone));
    // Tokio's blocking stdin read cannot be cancelled while a pipe stays
    // open. Hosts have already awaited AppCore cleanup; do not let idle
    // stdio keep SIGINT/SIGTERM shutdown waiting forever on runtime drop.
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
    code
}

async fn dispatch(args: Vec<String>, json_mode: bool, standalone: bool) -> i32 {
    if args.first().is_some_and(|arg| arg == "web") {
        #[cfg(feature = "local-web")]
        return web::run(&args[1..], standalone).await;
        #[cfg(not(feature = "local-web"))]
        {
            eprintln!("tiktools: local web adapter unavailable; rebuild with --features local-web");
            return 2;
        }
    }
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
    let plan = match commands::parse_command(&args) {
        Ok(parsed) => parsed,
        Err(commands::CommandError::Usage(message)) => {
            eprintln!("tiktools: {message}\n\n{USAGE}");
            return 2;
        }
        Err(commands::CommandError::Call(error)) => {
            let exit = client::exit_code_for(&error);
            client::print_error(json_mode, &error);
            return exit;
        }
    };
    let json_mode = match &plan {
        commands::Plan::Api(command) => command.output_json().unwrap_or(json_mode),
        _ => json_mode,
    };
    // Sandbox verification forces an isolated throwaway runtime even
    // without --standalone, so CI and agents can verify safely anywhere.
    if commands::api::is_sandbox_verify(&args) {
        let _sandbox = match client::SandboxHome::create() {
            Ok(sandbox) => sandbox,
            Err(error) => {
                eprintln!("tiktools: could not create sandbox: {error}");
                return 3;
            }
        };
        return execute_and_print(&client::direct_client(), plan, json_mode).await;
    }
    let resolved = match client::resolve_selected(standalone, None).await {
        Ok(resolved) => resolved,
        Err(error) => {
            client::print_error(json_mode, &error);
            return 3;
        }
    };
    execute_and_print(&resolved, plan, json_mode).await
}

fn selection_error(args: &[String], standalone: bool) -> Option<&'static str> {
    let _ = (args, standalone);
    None
}

async fn execute_and_print(
    resolved: &tiktools_client::TikToolsClient,
    plan: commands::Plan,
    json_mode: bool,
) -> i32 {
    match commands::execute_plan(resolved, plan).await {
        Ok(outcome) => {
            if !outcome.written {
                let json_mode = outcome.json.unwrap_or(json_mode);
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string(&outcome.value).unwrap_or_default()
                    );
                } else {
                    output::print_human(&outcome.command, outcome.value);
                }
            }
            outcome.exit
        }
        Err(error) => {
            let exit = client::exit_code_for(&error);
            client::print_error(json_mode, &error);
            exit
        }
    }
}

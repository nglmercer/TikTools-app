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

/// Renders help from the command specs: human text by default, the full
/// machine-readable registry with `--json`.
fn help_render(filter: Option<&str>, json: bool) -> Result<String, String> {
    let domains = commands::spec::registry();
    if json {
        let selected = commands::spec::select(&domains, filter)?;
        return Ok(
            serde_json::to_string(&commands::spec::render_json(&selected)).unwrap_or_default(),
        );
    }
    commands::spec::render(&domains, filter)
}

fn main() {
    tiktools_tls::ensure_default_crypto_provider();
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
        // Help is offline and generated from the command specs, so agents can
        // rely on `--json` here before any host exists.
        let filter = if args.first().is_some_and(|arg| arg == "help") {
            args.get(1)
        } else {
            None
        };
        if args.len() > 2 {
            eprintln!("tiktools: help takes at most one command name");
            return 2;
        }
        match help_render(filter.map(String::as_str), json_mode) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("tiktools: {error}");
                return 2;
            }
        }
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
            eprintln!(
                "tiktools: {message}\nhint: try `tiktools help` or `tiktools help <command>`"
            );
            return 2;
        }
    }
    // Parse before connecting: usage errors never depend on host state.
    let plan = match commands::parse_command(&args) {
        Ok(parsed) => parsed,
        Err(commands::CommandError::Usage(message)) => {
            eprintln!(
                "tiktools: {message}\nhint: try `tiktools help` or `tiktools help <command>`"
            );
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

use super::*;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// One offline CLI action: talks to the running adapter over loopback instead
/// of starting the server. One registry entry drives dispatch, option parsing
/// and the usage text, so a new action never touches the plumbing.
struct CliAction {
    flag: &'static str,
    usage: &'static str,
    summary: &'static str,
    handler: fn(u16) -> i32,
}

pub(super) const U_PRINT_CODE: &str = "web --print-code [--port N]";
pub(super) const U_RESET_SESSION: &str = "web --reset-session [--port N]";

const CLI_ACTIONS: &[CliAction] = &[
    CliAction {
        flag: "--print-code",
        usage: U_PRINT_CODE,
        summary: "print the active pairing code without restarting the adapter",
        handler: print_pairing_code,
    },
    CliAction {
        flag: "--reset-session",
        usage: U_RESET_SESSION,
        summary: "revoke every paired browser and print a fresh code",
        handler: reset_pairing_from_cli,
    },
];

const SERVER_USAGE: &str =
    "web --assets <directory> [--port N] [--join-origin URL] [--multi-session]";

fn usage() -> String {
    let mut text = format!("usage: tiktools {SERVER_USAGE}\n");
    for action in CLI_ACTIONS {
        let _ = writeln!(text, "       tiktools {}", action.usage);
    }
    text.push_str("Local browser adapter; start tiktools host --ipc first.\n");
    for action in CLI_ACTIONS {
        let _ = writeln!(text, "{}: {}", action.flag, action.summary);
    }
    let _ = write!(
        text,
        "--multi-session (or {MULTI_SESSION_ENV}=1) lets several browsers pair at once."
    );
    text
}

pub async fn run(args: &[String], standalone: bool) -> i32 {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{}", usage());
        return 0;
    }
    if let Some(action) = CLI_ACTIONS
        .iter()
        .find(|action| args.iter().any(|arg| arg == action.flag))
    {
        return match parse_action_options(args, action.flag) {
            Ok(port) => (action.handler)(port),
            Err(error) => {
                eprintln!("tiktools web: {error}");
                2
            }
        };
    }
    let (assets, port, join_origin, multi_session) = match parse_options(args, standalone) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            return 2;
        }
    };
    match start(assets, port, join_origin, multi_session).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            3
        }
    }
}

pub(super) fn multi_session_enabled(flag: bool) -> bool {
    flag || matches!(
        std::env::var(MULTI_SESSION_ENV),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true")
    )
}

/// A registered action (`web <flag> [--port N]`) reads or resets adapter
/// state without restarting it, so only `--port` may accompany the flag.
pub(super) fn parse_action_options(args: &[String], flag: &str) -> Result<u16, String> {
    let mut port: u16 = DEFAULT_PORT;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            value if value == flag => {}
            "--port" => {
                port = options
                    .next()
                    .ok_or("--port requires a number")?
                    .parse()
                    .map_err(|_| "invalid port")?;
            }
            _ => {
                return Err(format!(
                    "unknown option: {option}; use web {flag} [--port N]"
                ));
            }
        }
    }
    Ok(port)
}

#[cfg(test)]
pub(super) fn parse_print_code_options(args: &[String]) -> Result<u16, String> {
    parse_action_options(args, "--print-code")
}

fn print_pairing_code(port: u16) -> i32 {
    match current_pairing_code(port) {
        Ok((code, remaining)) => {
            // Bare code on stdout for agents and scripts; the human note goes
            // to stderr. Use it immediately: the code is single-use.
            println!("{code}");
            eprintln!("one-use pairing code (single use, valid for {remaining}s); never store it");
            0
        }
        Err(error) => {
            eprintln!("tiktools web --print-code: {error}");
            3
        }
    }
}

/// The reset endpoint is loopback-only and authenticated by a 0600 token the
/// adapter wrote at startup. A plain TCP request keeps this subcommand usable
/// without a runtime or an HTTP client dependency (like `--print-code`).
fn post_session_reset(port: u16, token: &str) -> Result<(), String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("local browser adapter is not running ({error})"))?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    let body = format!("{{\"token\":\"{token}\"}}");
    let request = format!(
        "POST {SESSION_RESET_PATH} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;
    match response.split_whitespace().nth(1) {
        Some("200") => Ok(()),
        Some("401") => Err("reset token rejected; restart the adapter".into()),
        Some("403") => Err("reset rejected by the adapter".into()),
        Some(code) => Err(format!("adapter returned HTTP {code}")),
        None => Err("invalid adapter response".into()),
    }
}

fn reset_pairing_from_cli(port: u16) -> i32 {
    let result = current_reset_token(port)
        .and_then(|token| post_session_reset(port, &token))
        .and_then(|()| current_pairing_code(port));
    match result {
        Ok((code, remaining)) => {
            println!("{code}");
            eprintln!(
                "session reset; all paired browsers revoked; new one-use code (valid for {remaining}s); never store it"
            );
            0
        }
        Err(error) => {
            eprintln!("tiktools web --reset-session: {error}");
            3
        }
    }
}

pub(super) fn parse_options(
    args: &[String],
    standalone: bool,
) -> Result<(PathBuf, u16, Option<String>, bool), String> {
    if standalone {
        return Err("web connects to the existing host; omit --standalone".into());
    }
    let mut assets = None;
    let mut port: u16 = DEFAULT_PORT;
    let mut join_origin = None;
    let mut multi_session = false;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--multi-session" => multi_session = true,
            "--assets" => {
                assets = Some(PathBuf::from(
                    options.next().ok_or("--assets requires a directory")?,
                ))
            }
            "--port" => {
                port = options
                    .next()
                    .ok_or("--port requires a number")?
                    .parse()
                    .map_err(|_| "invalid port")?
            }
            "--join-origin" => {
                join_origin = Some(normalize_join_origin(
                    options.next().ok_or("--join-origin requires a URL")?,
                )?);
            }
            _ => return Err(format!("unknown option: {option}; use {SERVER_USAGE}")),
        }
    }
    Ok((
        assets.ok_or("--assets is required")?,
        port,
        join_origin,
        multi_session,
    ))
}

/// Join links are terminal convenience only: keep them on loopback HTTP so a
/// printed one-click URL can never point a browser at a remote host.
pub(super) fn normalize_join_origin(value: &str) -> Result<String, String> {
    let base = value.trim_end_matches('/');
    let (host, port_text) = base
        .strip_prefix("http://127.0.0.1:")
        .map(|port| ("127.0.0.1", port))
        .or_else(|| {
            base.strip_prefix("http://localhost:")
                .map(|port| ("localhost", port))
        })
        .ok_or("join origin must be an http://127.0.0.1:<port> or http://localhost:<port> URL")?;
    if port_text.is_empty() || !port_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid join-origin port".into());
    }
    let port: u16 = port_text.parse().map_err(|_| "invalid join-origin port")?;
    if port == 0 {
        return Err("invalid join-origin port".into());
    }
    Ok(format!("http://{host}:{port}"))
}

pub(super) fn join_link(join_base: &str, pairing: &str) -> String {
    format!("{join_base}/#pair={pairing}")
}

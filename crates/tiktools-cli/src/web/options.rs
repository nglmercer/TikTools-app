use super::*;

pub async fn run(args: &[String], standalone: bool) -> i32 {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("usage: tiktools web --assets <directory> [--port N] [--join-origin URL]\n       tiktools web --print-code [--port N]\nLocal browser adapter; start tiktools host --ipc first.\n--print-code prints the active six-digit pairing code without restarting the adapter.");
        return 0;
    }
    if args.iter().any(|arg| arg == "--print-code") {
        return match parse_print_code_options(args) {
            Ok(port) => print_pairing_code(port),
            Err(error) => {
                eprintln!("tiktools web: {error}");
                2
            }
        };
    }
    let (assets, port, join_origin) = match parse_options(args, standalone) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            return 2;
        }
    };
    match start(assets, port, join_origin).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("tiktools web: {error}");
            3
        }
    }
}

/// `web --print-code [--port N]`: read the active pairing code back without
/// restarting the adapter. Only `--port` may accompany `--print-code`.
pub(super) fn parse_print_code_options(args: &[String]) -> Result<u16, String> {
    let mut port: u16 = 8787;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--print-code" => {}
            "--port" => {
                port = options
                    .next()
                    .ok_or("--port requires a number")?
                    .parse()
                    .map_err(|_| "invalid port")?;
            }
            _ => {
                return Err(format!(
                    "unknown option: {option}; use web --print-code [--port N]"
                ));
            }
        }
    }
    Ok(port)
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

pub(super) fn parse_options(
    args: &[String],
    standalone: bool,
) -> Result<(PathBuf, u16, Option<String>), String> {
    if standalone {
        return Err("web connects to the existing host; omit --standalone".into());
    }
    let mut assets = None;
    let mut port: u16 = 8787;
    let mut join_origin = None;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
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
            _ => {
                return Err(format!(
                    "unknown option: {option}; use web --assets <directory> [--port N] [--join-origin URL]"
                ))
            }
        }
    }
    Ok((assets.ok_or("--assets is required")?, port, join_origin))
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

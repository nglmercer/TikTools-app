//! Bounded loopback HTTP probes for gateway health and widget assets.
use super::*;

pub(super) async fn probe_gateway(bind: std::net::IpAddr, port: u16) -> ProbeOutcome {
    match http_get_status(bind, port, "/health").await {
        Fetch::Unreachable => return ProbeOutcome::NotListening,
        Fetch::Status(Some(200)) => {}
        // Connected, but no valid gateway answer.
        Fetch::Status(_) => return ProbeOutcome::BadGateway,
    }
    for kind in VALID_WIDGETS {
        let path = format!("/widgets/{kind}/");
        if !matches!(
            http_get_status(bind, port, &path).await,
            Fetch::Status(Some(200))
        ) {
            return ProbeOutcome::AssetsMissing;
        }
    }
    ProbeOutcome::Ready
}

async fn http_get_status(bind: std::net::IpAddr, port: u16, path: &str) -> Fetch {
    let stream =
        match tokio::time::timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect((bind, port)))
            .await
        {
            Ok(Ok(stream)) => stream,
            _ => return Fetch::Unreachable,
        };
    let mut stream = stream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let request = format!("GET {path} HTTP/1.0\r\nHost: {bind}\r\nConnection: close\r\n\r\n");
    if tokio::time::timeout(PROBE_TIMEOUT, stream.write_all(request.as_bytes()))
        .await
        .ok()
        .is_none_or(|result: Result<(), std::io::Error>| result.is_err())
    {
        return Fetch::Status(None);
    }
    let mut body = Vec::with_capacity(1024);
    // Responses are tiny (`/health`, widget heads); cap the read so a
    // rogue listener cannot hold the probe open with a large body.
    let mut limited = stream.take(64 * 1024);
    if tokio::time::timeout(PROBE_TIMEOUT, limited.read_to_end(&mut body))
        .await
        .ok()
        .is_none_or(|result: Result<usize, std::io::Error>| result.is_err())
    {
        return Fetch::Status(None);
    }
    Fetch::Status(parse_http_status(&body))
}

/// Parses `HTTP/1.x <status>` from a raw response head. Anything
/// malformed yields `None` instead of guessing.
pub(super) fn parse_http_status(bytes: &[u8]) -> Option<u16> {
    let head = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .and_then(|line| std::str::from_utf8(line).ok())?;
    let mut parts = head.split_whitespace();
    if !parts
        .next()
        .is_some_and(|version| version.starts_with("HTTP/"))
    {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fetch {
    /// Nothing listening: refused, timed out, or otherwise unreachable.
    Unreachable,
    /// Connected; the status line parsed (`None` when malformed).
    Status(Option<u16>),
}

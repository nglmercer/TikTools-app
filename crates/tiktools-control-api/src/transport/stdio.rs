//! JSON-RPC over stdin/stdout.

use super::server::serve_stream;
use crate::ControlApi;

/// Serves JSON-RPC over stdin/stdout until EOF or `system.shutdown`.
/// `stream_events` interleaves domain-event notifications on stdout.
pub async fn run_stdio(api: ControlApi, stream_events: bool) -> std::io::Result<()> {
    run_stdio_shared(std::sync::Arc::new(api), stream_events).await
}

pub async fn run_stdio_shared(
    api: std::sync::Arc<ControlApi>,
    stream_events: bool,
) -> std::io::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    serve_stream(
        &api,
        tokio::io::BufReader::new(stdin),
        stdout,
        stream_events,
    )
    .await
}

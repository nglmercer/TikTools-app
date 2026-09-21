//! NDJSON line framing, request limits, and response encoding.

use crate::{ApiError, ControlApi, RpcResponse};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Hard cap for one request line (bytes, including the newline).
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Hard cap for the serialized `params` payload of one request.
pub const MAX_PARAMS_BYTES: usize = 256 * 1024;

pub(crate) enum LineOutcome {
    Ok,
    TooLarge,
}

/// Bytes of an overlong line kept for request-id recovery, so the
/// `too_large` error still correlates with the pending call instead of
/// hanging it until timeout. Must cover the `{"jsonrpc","id",...}` prefix.
const ID_PREFIX_RECOVERY_BYTES: usize = 256;

/// Reads one `\n`-terminated line capped at [`MAX_REQUEST_BYTES`]. Overlong
/// lines are discarded through the terminator so framing stays aligned,
/// keeping only an id-recovery prefix in `line`.
pub(crate) async fn read_capped_line<R>(
    reader: &mut R,
    line: &mut Vec<u8>,
) -> std::io::Result<Option<LineOutcome>>
where
    R: AsyncBufRead + Unpin,
{
    line.clear();
    let mut total = 0usize;
    let mut too_large = false;
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok((!too_large && total > 0).then_some(LineOutcome::Ok));
        }
        let end = chunk.iter().position(|byte| *byte == b'\n');
        match end {
            Some(index) => {
                let take = index + 1;
                if !too_large {
                    total += take;
                    if total > MAX_REQUEST_BYTES {
                        too_large = true;
                        // Keep the line prefix for id recovery even when a
                        // single chunk blows the cap with `line` still empty.
                        let room = ID_PREFIX_RECOVERY_BYTES.saturating_sub(line.len());
                        line.extend_from_slice(&chunk[..take.min(room)]);
                        line.truncate(ID_PREFIX_RECOVERY_BYTES);
                    } else {
                        line.extend_from_slice(&chunk[..take]);
                    }
                }
                reader.consume(take);
                return Ok(Some(if too_large {
                    LineOutcome::TooLarge
                } else {
                    LineOutcome::Ok
                }));
            }
            None => {
                if !too_large {
                    total += chunk.len();
                    if total > MAX_REQUEST_BYTES {
                        too_large = true;
                        let room = ID_PREFIX_RECOVERY_BYTES.saturating_sub(line.len());
                        line.extend_from_slice(&chunk[..chunk.len().min(room)]);
                        line.truncate(ID_PREFIX_RECOVERY_BYTES);
                    } else {
                        line.extend_from_slice(chunk);
                    }
                }
                let len = chunk.len();
                reader.consume(len);
            }
        }
    }
}

pub(crate) async fn execute_line(api: &ControlApi, line: &[u8]) -> RpcResponse {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return RpcResponse::error(
            crate::RpcId::Null,
            ApiError::invalid_params("empty request"),
        );
    }
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(raw) => api.execute_value(&raw).await,
        Err(error) => RpcResponse::error(
            crate::RpcId::extract_from_prefix(trimmed),
            ApiError::invalid_params(format!("invalid JSON: {error}")),
        ),
    }
}

pub(crate) async fn write_response<W>(writer: &mut W, response: &RpcResponse) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut bytes = serde_json::to_vec(response)
        .unwrap_or_else(|_| b"{\"jsonrpc\":\"2.0\",\"id\":null}".to_vec());
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

/// Writes one JSON-RPC notification line (domain event or event gap).
pub(crate) async fn write_notification<W>(
    writer: &mut W,
    notification: &serde_json::Value,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut bytes = serde_json::to_vec(notification).unwrap_or_else(|_| b"{}".to_vec());
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

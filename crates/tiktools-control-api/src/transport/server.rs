//! Connection serving loop shared by all transports.

use super::framing::{
    execute_line, read_capped_line, write_notification, write_response, LineOutcome,
};
use crate::{event_notification, ApiError, ControlApi, RpcResponse};
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

pub(crate) async fn serve_stream<R, W>(
    api: &ControlApi,
    mut reader: R,
    mut writer: W,
    stream_events: bool,
) -> std::io::Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut events = stream_events.then(|| api.subscribe());
    let mut line = Vec::with_capacity(4096);
    loop {
        tokio::select! {
            biased;
            result = read_capped_line(&mut reader, &mut line) => {
                match result? {
                    None => return Ok(()),
                    Some(LineOutcome::Ok) => {
                        let response = execute_line(api, &line).await;
                        line.clear();
                        write_response(&mut writer, &response).await?;
                        // Flush events the request published (a biased select
                        // would otherwise starve them behind piped requests).
                        drain_events(&mut writer, &mut events, api.core()).await?;
                        if api.core().is_shutdown() {
                            writer.flush().await?;
                            return Ok(());
                        }
                    }
                    Some(LineOutcome::TooLarge) => {
                        // `line` holds the recovery prefix: preserve the
                        // request id so the caller fails fast with
                        // `too_large` instead of timing out.
                        let id = crate::RpcId::extract_from_prefix(
                            &String::from_utf8_lossy(line.as_slice()),
                        );
                        line.clear();
                        let response = RpcResponse::error(id, ApiError::too_large());
                        write_response(&mut writer, &response).await?;
                    }
                }
            }
            event = async {
                match events.as_mut() {
                    Some(receiver) => Some(receiver.recv().await),
                    None => std::future::pending().await,
                }
            } => {
                use tiktools_core::events::DomainRecvError;
                match event {
                    Some(Ok(event)) => {
                        write_notification(
                            &mut writer,
                            &event_notification(&event),
                        )
                        .await?;
                    }
                    // Reliable lag is never silent: the client learns it
                    // missed authoritative events and must resync, while
                    // the connection stays alive for later events.
                    Some(Err(DomainRecvError::ReliableLagged(lost))) => {
                        tracing::warn!(
                            lost,
                            "control IPC event subscriber lagged on the reliable lane"
                        );
                        api.core().record_event_gap();
                        write_notification(&mut writer, &crate::gap_notification(lost)).await?;
                    }
                    // Lossy lag only skipped feed/snapshots: continue with
                    // no gap signal.
                    Some(Err(DomainRecvError::LossyLagged(skipped))) => {
                        tracing::debug!(
                            skipped,
                            "control IPC event subscriber skipped a lossy burst"
                        );
                    }
                    // The bus is gone: terminate the event stream, not the
                    // connection. Disarming here also avoids a Closed
                    // busy loop (a closed receiver is always ready).
                    Some(Err(DomainRecvError::Closed)) | None => {
                        events = None;
                    }
                }
            }
        }
    }
}

/// Writes every queued domain event as a notification. Lag never
/// stops the drain: reliable lag emits an explicit gap notification and
/// draining continues, lossy lag just continues. Only an empty lane or
/// a closed bus ends the drain (a closed bus disarms the stream).
pub(crate) async fn drain_events<W>(
    writer: &mut W,
    events: &mut Option<tiktools_core::events::DomainSubscription>,
    core: &tiktools_core::AppCore,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    use tiktools_core::events::DomainTryRecvError;
    let Some(receiver) = events.as_mut() else {
        return Ok(());
    };
    loop {
        match receiver.try_recv() {
            Ok(event) => {
                let notification = event_notification(&event);
                let mut bytes =
                    serde_json::to_vec(&notification).unwrap_or_else(|_| b"{}".to_vec());
                bytes.push(b'\n');
                writer.write_all(&bytes).await?;
            }
            Err(DomainTryRecvError::ReliableLagged(lost)) => {
                tracing::warn!(lost, "control IPC event drain lagged on the reliable lane");
                core.record_event_gap();
                let notification = crate::gap_notification(lost);
                let mut bytes =
                    serde_json::to_vec(&notification).unwrap_or_else(|_| b"{}".to_vec());
                bytes.push(b'\n');
                writer.write_all(&bytes).await?;
            }
            Err(DomainTryRecvError::LossyLagged(skipped)) => {
                tracing::debug!(skipped, "control IPC event drain skipped a lossy burst");
            }
            Err(DomainTryRecvError::Empty) => break,
            Err(DomainTryRecvError::Closed) => {
                *events = None;
                break;
            }
        }
    }
    writer.flush().await
}

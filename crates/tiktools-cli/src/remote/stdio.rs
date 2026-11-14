//! Persistent agent client. This module never creates AppCore or a local host.
use super::*;
use serde::Deserialize;
use tiktools_client::{validate_params, ControlEvent, RequestMetadata, TikToolsClient};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

const MAX_LINE: usize = 128 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default = "empty_params")]
    params: Value,
    #[serde(default)]
    confirm: bool,
    #[serde(default)]
    meta: Option<RequestMetadata>,
}
fn empty_params() -> Value {
    json!({})
}

async fn line(reader: &mut (impl AsyncBufRead + Unpin)) -> std::io::Result<Option<Vec<u8>>> {
    let mut output = Vec::new();
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok((!output.is_empty()).then_some(output));
        }
        let end = chunk
            .iter()
            .position(|b| *b == b'\n')
            .map(|n| n + 1)
            .unwrap_or(chunk.len());
        if output.len() + end > MAX_LINE {
            return Err(std::io::Error::other("NDJSON request exceeds 128 KiB"));
        }
        output.extend_from_slice(&chunk[..end]);
        let finished = chunk[end - 1] == b'\n';
        reader.consume(end);
        if finished {
            return Ok(Some(output));
        }
    }
}

fn emit(value: Value) {
    println!("{}", crate::output::redact_secrets(value));
}
fn failed(id: Value, failure: ClientError) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":failure.code,"message":failure.message,"data":failure.data}})
}

async fn request(client: &TikToolsClient, bytes: &[u8], budget: Duration) -> Value {
    let request: Request = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => {
            return failed(
                Value::Null,
                error("invalid_request", "expected one JSON-RPC request per line"),
            )
        }
    };
    if request.jsonrpc != "2.0"
        || !(request.id.is_string() || request.id.is_u64() || request.id.is_i64())
    {
        return failed(
            Value::Null,
            error(
                "invalid_request",
                "jsonrpc 2.0 and a string/integer id are required",
            ),
        );
    }
    let id = request.id;
    let execution = async {
        validate_params(&request.method, &request.params)?;
        let schema = client
            .rpc_schema(tiktools_control_api::modules::rpc::SchemaParams {
                method: request.method.clone(),
            })
            .await?;
        if (schema.side_effect || schema.destructive) && !request.confirm {
            return Err(error(
                "confirmation_required",
                "effectful requests require confirm:true",
            ));
        }
        client
            .call_value_with_meta(&request.method, request.params, request.meta)
            .await
    };
    match tokio::time::timeout(budget, execution).await {
        Ok(Ok(result)) => {
            json!({"jsonrpc":"2.0","id":id,"result":result.result,"meta":result.meta})
        }
        Ok(Err(failure)) => failed(id, failure),
        Err(_) => failed(
            id,
            error(
                "timeout",
                "request timed out; accepted writes may still complete",
            ),
        ),
    }
}

pub(super) async fn run(args: &[String]) -> Result<(), ClientError> {
    let opts = crate::commands::teams::options(
        args,
        &["--profile", "--timeout"],
        &["--stdio", "--events"],
    )
    .map_err(|message| error("invalid_params", &message))?;
    if !opts.contains_key("--stdio") {
        return Err(error("invalid_params", "remote serve requires --stdio"));
    }
    let name = opts
        .get("--profile")
        .filter(|name| tiktools_client::remote::profiles::valid_name(name))
        .ok_or_else(|| error("invalid_params", "--profile NAME is required"))?;
    let budget = crate::commands::teams::timeout(&opts, 30)
        .map_err(|message| error("invalid_params", &message))?;
    let remote = tokio::time::timeout(budget, ProfileManager::system()?.open(name))
        .await
        .map_err(|_| error("timeout", "remote connection deadline exceeded"))??;
    let hello = remote.hello();
    emit(
        json!({"jsonrpc":"2.0","method":"connection.ready","params":{
            "profile":name,"hostId":hello.host_id,"hostInstanceId":hello.host_instance_id,
            "connectionId":hello.connection_id,"sessionId":hello.session_id,
            "grantRevision":hello.grant_revision,"capabilities":hello.capabilities
        }}),
    );
    let client = TikToolsClient::Remote(remote.clone());
    let mut events = remote.subscribe();
    let mut reader = BufReader::new(tokio::io::stdin());
    let stopping = termination();
    tokio::pin!(stopping);
    let result = loop {
        tokio::select! {
            incoming = line(&mut reader) => match incoming {
                Ok(Some(bytes)) => {
                    tokio::select! {
                        response = request(&client, &bytes, budget) => emit(response),
                        _ = &mut stopping => break Ok(()),
                    }
                }
                Ok(None) => break Ok(()),
                Err(_) => break Err(error("invalid_request", "NDJSON input failed or exceeded its limit")),
            },
            // Always drain the stream: closure also wakes idle clients that
            // did not request event output. The SDK emits a gap on shutdown.
            incoming = events.recv() => match incoming {
                Ok(ControlEvent::Reliable(event)) if opts.contains_key("--events") => emit(json!({"jsonrpc":"2.0","method":"event","params":event})),
                Ok(ControlEvent::Domain(event)) if opts.contains_key("--events") => emit(json!({"jsonrpc":"2.0","method":"event","params":event})),
                Ok(ControlEvent::Gap { lost }) | Err(tokio::sync::broadcast::error::RecvError::Lagged(lost)) if opts.contains_key("--events") => emit(json!({"jsonrpc":"2.0","method":"event.gap","params":{"lost":lost,"resync":true}})),
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {},
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break Err(error("transport", "remote event stream closed")),
            },
            _ = &mut stopping => break Ok(()),
        }
        if remote.is_closed() {
            break Err(remote
                .closed_reason()
                .unwrap_or_else(|| error("transport", "remote connection closed")));
        }
    };
    remote.close();
    result
}

async fn termination() {
    #[cfg(unix)]
    {
        if let Ok(mut term) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn bounded_reader_rejects_oversize_before_accumulating_it() {
        let bytes = vec![b'x'; MAX_LINE + 1];
        assert!(line(&mut BufReader::new(bytes.as_slice())).await.is_err());
        let mut reader = BufReader::new(b"{}\n[]\n".as_slice());
        assert_eq!(line(&mut reader).await.unwrap().unwrap(), b"{}\n");
        assert_eq!(line(&mut reader).await.unwrap().unwrap(), b"[]\n");
        assert!(line(&mut reader).await.unwrap().is_none());
    }
}

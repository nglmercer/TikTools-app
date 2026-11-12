//! Typed API command dispatch through the selected client.
use super::*;

pub async fn execute(client: &TikToolsClient, command: Command) -> Result<Output, ClientError> {
    match command {
        Command::Discover { json } => {
            let discovered = client.rpc_discover().await?;
            Ok(Output::new("api-discover", result_value(discovered)?).json_override(json))
        }
        Command::Schema { method, json } => {
            let meta = client.rpc_schema(SchemaParams { method }).await?;
            Ok(Output::new("api-schema", result_value(meta)?).json_override(json))
        }
        Command::Call {
            method,
            params,
            timeout_secs,
            metadata,
            include_meta,
            dry_run,
            confirm,
            secrets_visible,
            explain,
            json,
        } => {
            execute_call(
                client,
                CallArgs {
                    method: &method,
                    params,
                    timeout_secs,
                    metadata,
                    include_meta,
                    dry_run,
                    confirm,
                    secrets_visible,
                    explain,
                    json,
                },
            )
            .await
        }
        Command::Events {
            topics,
            timeout_secs,
            max_events,
        } => {
            stream_events(client, &topics, timeout_secs, max_events).await?;
            Ok(Output::written())
        }
        Command::Verify {
            coverage,
            smoke,
            timeout_secs,
            json,
        } => execute_verify(client, coverage, smoke, timeout_secs, json).await,
    }
}

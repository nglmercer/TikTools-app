//! Live schema validation, destructive-call confirmation and redacted call output.
use super::*;

pub(super) async fn execute_call(
    client: &TikToolsClient,
    args: CallArgs<'_>,
) -> Result<Output, ClientError> {
    // Resolve the method live (unknown names fail here) and validate the
    // params against the registry's own typed shape before executing.
    let meta = client
        .rpc_schema(SchemaParams {
            method: args.method.to_owned(),
        })
        .await?;
    validate_params(args.method, &args.params)?;
    if let Some(metadata) = &args.metadata {
        if metadata.command_id.is_some() && !meta.durable_commands {
            return Err(ClientError::new(
                "unsupported_remote",
                "this method does not support durable commands",
            ));
        }
        if metadata.expected_revision.is_some()
            && meta.concurrency
                != Some(tiktools_control_api::registry::ConcurrencyStrategy::CompareAndSet)
        {
            return Err(ClientError::new(
                "invalid_params",
                "this method does not accept expectedRevision",
            ));
        }
        if metadata.command_id.is_some()
            && meta.concurrency
                == Some(tiktools_control_api::registry::ConcurrencyStrategy::CompareAndSet)
            && metadata.expected_revision.is_none()
        {
            return Err(ClientError::new(
                "invalid_params",
                "compare-and-set commands require expectedRevision",
            ));
        }
    }
    if args.explain {
        explain_call(&meta, &args.params, args.metadata.as_ref());
    }
    if args.dry_run {
        return Ok(Output::new(
            "api-call",
            json!({"ok": true, "dryRun": true, "method": args.method}),
        )
        .json_override(args.json));
    }
    if meta.destructive && !args.confirm {
        return Err(ClientError::new(
            "confirmation_required",
            format!(
                "refusing to run destructive method `{}` without --confirm",
                args.method
            ),
        ));
    }
    let result = match args.timeout_secs {
        Some(secs) => {
            match tokio::time::timeout(
                Duration::from_secs(secs),
                client.call_value_with_meta(args.method, &args.params, args.metadata.clone()),
            )
            .await
            {
                Ok(outcome) => outcome?,
                Err(_) => {
                    let mut error = ClientError::new(
                        "timeout",
                        format!(
                            "call timed out after {secs}s; accepted operation may still complete"
                        ),
                    );
                    if let Some(id) = args
                        .metadata
                        .as_ref()
                        .and_then(|meta| meta.command_id.as_ref())
                    {
                        error.data = Some(json!({"commandId":id,"state":"unknown"}));
                    }
                    return Err(error);
                }
            }
        }
        None => {
            client
                .call_value_with_meta(args.method, &args.params, args.metadata)
                .await?
        }
    };
    let result = if args.include_meta {
        json!({"result":result.result,"meta":result.meta})
    } else {
        result.result
    };
    Ok(
        Output::new("api-call", redact_output(result, args.secrets_visible))
            .json_override(args.json),
    )
}

/// Prints the call plan to stderr (stdout stays pure result). Params are
/// always redacted here: `--explain` must never leak secrets.
fn explain_call(
    meta: &tiktools_client::MethodMeta,
    params: &Value,
    metadata: Option<&RequestMetadata>,
) {
    if let Some(metadata) = metadata {
        eprintln!(
            "metadata: {}",
            serde_json::to_string(metadata).unwrap_or_default()
        );
    }
    let mut flags = Vec::new();
    if meta.side_effect {
        flags.push("writes");
    }
    if meta.destructive {
        flags.push("destructive");
    }
    if meta.requires_desktop {
        flags.push("desktop-only");
    }
    eprintln!("method: {}", meta.name);
    eprintln!("description: {}", meta.description);
    eprintln!(
        "flags: {}",
        if flags.is_empty() {
            "read-only".to_owned()
        } else {
            flags.join(", ")
        }
    );
    eprintln!(
        "params: {}",
        serde_json::to_string_pretty(&redact_output(params.clone(), false)).unwrap_or_default()
    );
}

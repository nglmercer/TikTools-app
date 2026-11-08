//! Approved scripts and emitted events retain the original saved authority.
use super::context::ExecutionContext;
use crate::*;

#[cfg(feature = "persistence")]
use super::{approved_effects::complete_external_effect, external_effects::ExternalStart};
#[cfg(feature = "persistence")]
use crate::services::effects::EffectOutcome;

#[cfg(feature = "persistence")]
const MAX_EMIT_BYTES: usize = 64 * 1024;

impl AppCore {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn execute_approved_emit_action(
        self: &Arc<Self>,
        action: &Value,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        #[cfg(feature = "persistence")]
        {
            self.with_core_action(action, context, |_| Ok(()))?;
            let topic = config
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("overlay.alert");
            let topic = normalize_emit_type(topic)?;
            let data = config.get("data").cloned().unwrap_or_else(|| json!({}));
            if !data.is_object() {
                return Err("Emitted data must be an object.".into());
            }
            let data = render_emitted_data(&data, event)?;
            let context = context.effect_child_slot("emitted-output", 0)?;
            self.emit_approved_event(action, &topic, data, event, &context, test)
                .await?;
            let summary = if test {
                format!("would emit {topic}")
            } else {
                format!("emit {topic}")
            };
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            Ok(summary)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (action, config, event, context, logs, test);
            Err("delegated emits require persistence".into())
        }
    }

    #[cfg(feature = "persistence")]
    pub(super) async fn emit_approved_event(
        self: &Arc<Self>,
        action: &Value,
        topic: &str,
        data: Value,
        source: &Value,
        context: &ExecutionContext,
        test: bool,
    ) -> Result<(), String> {
        if !crate::collaboration::valid_emit_topic(topic)
            || !data.is_object()
            || data.to_string().len() > MAX_EMIT_BYTES
        {
            return Err("Emitted topic or payload is invalid.".into());
        }
        let check = {
            let topic = topic.to_owned();
            move |identities: &[crate::collaboration::ExecutionIdentity]| {
                if identities
                    .iter()
                    .any(|identity| !identity.grant().allows_emit_topic(&topic))
                {
                    Err("Emitted topic is outside execution scope.".into())
                } else {
                    Ok(())
                }
            }
        };
        self.with_core_action(action, context, &check)?;
        let child = context.child().map_err(str::to_owned)?;
        if test {
            return Ok(());
        }
        let started = self
            .start_external_effect_owned(
                action.clone(),
                context.clone(),
                crate::control::EffectKind::Emit,
                json!({"topic":topic,"data":data}),
                check.clone(),
            )
            .await?;
        let dispatch = match started {
            ExternalStart::Retained(receipt) => {
                return Err(format!(
                    "Emitted effect already retained ({:?}); it was not replayed.",
                    receipt.state
                ))
            }
            ExternalStart::Dispatched(dispatch) => dispatch,
        };
        self.with_core_action(action, context, &check)?;
        // This direct requested type deliberately has no plugin-Owner stamp.
        // The existing Owner emitter keeps its legacy plugin.emit envelope.
        let mut payload = json!({"id":format!("scoped-emit-{}",self.next_sequence()),
            "type":topic,"timestamp":now_millis(),"data":data});
        for key in ["creator", "user"] {
            if let Some(value) = source.get(key) {
                payload[key] = value.clone();
            }
        }
        let outcome =
            Box::pin(self.publish_scoped_automation_event(AutomationEvent::new(payload, child)))
                .await;
        complete_external_effect(
            dispatch,
            if outcome.is_ok() {
                EffectOutcome::Succeeded
            } else {
                EffectOutcome::Unknown
            },
        )
        .await?;
        self.with_core_action(action, context, check)?;
        outcome
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn execute_approved_script_action(
        self: &Arc<Self>,
        action: &Value,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        #[cfg(feature = "persistence")]
        {
            self.with_core_action(action, context, |_| Ok(()))?;
            let source = config
                .get("source")
                .and_then(Value::as_str)
                .ok_or("Script action has no source.")?
                .to_owned();
            let script_context = context.clone().mark_script_flow();
            let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let _cancel_on_drop = CancelScriptOnDrop(cancelled.clone());
            let permit = self
                .automation_state
                .scoped_script_slots
                .clone()
                .try_acquire_owned()
                .map_err(|_| {
                    "Scoped script evaluation is busy; no effect was accepted.".to_owned()
                })?;
            let core = self.clone();
            let input = event.clone();
            let retained = script_context.clone();
            let result = tokio::task::spawn_blocking(move || {
                // A dropped waiter cannot release capacity while its VM is
                // still unwinding or checking cancellation on this thread.
                let _permit = permit;
                let authority = core.clone();
                core.automation
                    .evaluate_scoped_script(&source, &input, move || {
                        cancelled.load(Ordering::Acquire)
                            || authority.is_shutdown()
                            || authority.automation_dispatch_guard(&retained).is_err()
                    })
            })
            .await
            .map_err(|_| "Scoped script execution is unavailable.".to_owned())??;
            self.with_core_action(action, &script_context, |_| Ok(()))?;
            let mut count = 0usize;
            for key in ["emit", tiktools_plugin_api::AUDIO_PLAY_INTENT, "fetch"] {
                count = count.saturating_add(
                    result
                        .get(key)
                        .map(|value| {
                            if value.is_array() {
                                value.as_array().map_or(0, Vec::len)
                            } else if value.is_null() {
                                0
                            } else {
                                1
                            }
                        })
                        .unwrap_or(0),
                );
            }
            if count > 64 {
                return Err("Script returned too many effect intents.".into());
            }
            // Freeze the complete evaluated plan before any returned effect.
            // Re-evaluation after restart must not move an old side effect to
            // a new ordinal by adding/reordering dynamic intents.
            let script_dispatch = if test {
                None
            } else {
                match self
                    .start_external_effect_owned(
                        action.clone(),
                        script_context.clone(),
                        crate::control::EffectKind::Script,
                        json!({"input":event,"result":result}),
                        |_| Ok(()),
                    )
                    .await?
                {
                    ExternalStart::Retained(receipt) => {
                        return Err(format!(
                        "Script invocation already retained ({:?}); its intents were not replayed.",
                        receipt.state
                    ))
                    }
                    ExternalStart::Dispatched(dispatch) => Some(dispatch),
                }
            };
            self.with_core_action(action, &script_context, |_| Ok(()))?;
            // Logs are also a final scoped sink; the evaluator never traces
            // guest console values on an Owner-authorized callback path.
            self.with_core_action(action, &script_context, |_| {
                for log in result
                    .get("log")
                    .into_iter()
                    .flat_map(as_values)
                    .filter_map(Value::as_str)
                {
                    if logs.len() < 40 {
                        logs.push(log.to_owned());
                    }
                }
                Ok(())
            })?;
            for (ordinal, intent) in result
                .get("emit")
                .into_iter()
                .flat_map(as_values)
                .filter(|value| !value.is_null())
                .enumerate()
            {
                let intent = intent
                    .as_object()
                    .ok_or("Script emit intent must be an object.")?;
                let topic = intent
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or("Script emit intent has no topic.")?;
                let topic = normalize_emit_type(topic)?;
                let data = intent.get("data").cloned().unwrap_or_else(|| json!({}));
                let data = render_emitted_data(&data, event)?;
                let child = script_context.effect_child_slot("script-emit", ordinal as u32)?;
                self.emit_approved_event(action, &topic, data, event, &child, test)
                    .await?;
            }
            for (ordinal, intent) in result
                .get(tiktools_plugin_api::AUDIO_PLAY_INTENT)
                .into_iter()
                .flat_map(as_values)
                .filter(|value| !value.is_null())
                .enumerate()
            {
                let intent = intent
                    .as_object()
                    .ok_or("Script audio intent must be an object.")?;
                let child = script_context.effect_child_slot("script-audio", ordinal as u32)?;
                self.execute_approved_audio_action(action, intent, event, &child, logs, test)
                    .await?;
            }
            for (ordinal, intent) in result
                .get("fetch")
                .into_iter()
                .flat_map(as_values)
                .filter(|value| !value.is_null())
                .enumerate()
            {
                let mut intent = intent
                    .as_object()
                    .ok_or("Script HTTP intent must be an object.")?
                    .clone();
                if let Some(topic) = result.get("emitResponseAs") {
                    intent.insert("emitResponseAs".into(), topic.clone());
                }
                let child = script_context.effect_child_slot("script-http", ordinal as u32)?;
                self.execute_approved_http_action(action, &intent, event, &child, logs, test)
                    .await?;
            }
            self.with_core_action(action, &script_context, |_| Ok(()))?;
            if let Some(dispatch) = script_dispatch {
                complete_external_effect(dispatch, EffectOutcome::Succeeded).await?;
            }
            self.with_core_action(action, &script_context, |_| Ok(()))?;
            Ok(if test {
                format!("would execute {count} scoped script intents")
            } else {
                format!("executed {count} scoped script intents")
            })
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (action, config, event, context, logs, test);
            Err("delegated scripts require persistence".into())
        }
    }
}

#[cfg(feature = "persistence")]
struct CancelScriptOnDrop(Arc<std::sync::atomic::AtomicBool>);
#[cfg(feature = "persistence")]
impl Drop for CancelScriptOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[cfg(feature = "persistence")]
fn render_emitted_data(source: &Value, event: &Value) -> Result<Value, String> {
    fn charge(bytes: usize, budget: &mut usize) -> Result<(), String> {
        *budget = budget
            .checked_sub(bytes)
            .ok_or("Emitted data exceeds its byte budget.")?;
        Ok(())
    }
    fn render(
        value: &Value,
        event: &Value,
        depth: usize,
        nodes: &mut usize,
        budget: &mut usize,
    ) -> Result<Value, String> {
        *nodes += 1;
        if depth > 32 || *nodes > 8192 {
            return Err("Emitted data exceeds its structural budget.".into());
        }
        Ok(match value {
            Value::String(text) => {
                let text = crate::services::automation_approval_policy::render_template(
                    text,
                    event,
                    (*budget).max(1),
                )?;
                charge(
                    serde_json::to_string(&text)
                        .map_err(|_| "Invalid emitted text.")?
                        .len(),
                    budget,
                )?;
                Value::String(text)
            }
            Value::Array(values) => {
                charge(2 + values.len().saturating_sub(1), budget)?;
                Value::Array(
                    values
                        .iter()
                        .map(|value| render(value, event, depth + 1, nodes, budget))
                        .collect::<Result<_, _>>()?,
                )
            }
            Value::Object(values) => {
                charge(2 + values.len().saturating_sub(1), budget)?;
                let mut output = serde_json::Map::new();
                for (key, value) in values {
                    charge(
                        serde_json::to_string(key)
                            .map_err(|_| "Invalid emitted field.")?
                            .len()
                            + 1,
                        budget,
                    )?;
                    output.insert(key.clone(), render(value, event, depth + 1, nodes, budget)?);
                }
                Value::Object(output)
            }
            _ => {
                charge(value.to_string().len(), budget)?;
                value.clone()
            }
        })
    }
    if !source.is_object() || source.to_string().len() > MAX_EMIT_BYTES {
        return Err("Emitted data exceeds its byte budget.".into());
    }
    let mut budget = MAX_EMIT_BYTES;
    render(source, event, 0, &mut 0, &mut budget)
}

#[cfg(all(test, feature = "persistence"))]
mod tests {
    use super::*;
    #[test]
    fn nested_emit_rendering_enforces_one_aggregate_serialized_budget() {
        let repeated = Value::Array(
            (0..32)
                .map(|_| Value::String("{{data.comment}}".into()))
                .collect(),
        );
        let source = json!({"comment":repeated});
        assert!(
            render_emitted_data(&source, &json!({"data":{"comment":"x".repeat(4096)}})).is_err()
        );
        let source = json!({"comment":{"nested":"{{data.comment}}"},"count":2});
        let output = render_emitted_data(&source, &json!({"data":{"comment":"hello"}})).unwrap();
        assert_eq!(output, json!({"comment":{"nested":"hello"},"count":2}));
        assert!(output.to_string().len() <= MAX_EMIT_BYTES);
    }
}

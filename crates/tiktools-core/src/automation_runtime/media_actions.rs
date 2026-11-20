//! Local media actions (audio playback) for automations and scripts.

use super::context::ExecutionContext;
use crate::*;

impl AppCore {
    pub(crate) async fn execute_audio_action(
        self: &Arc<Self>,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        globals: &std::collections::BTreeMap<String, String>,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
        self.execution_context_guard(context)?;
        let configured = config
            .get("fileRef")
            .or_else(|| config.get("file"))
            .or_else(|| config.get("filePath"))
            .or_else(|| config.get("path"))
            .ok_or_else(|| "Audio action has no file reference.".to_owned())?;
        let raw_path = configured
            .get("path")
            .and_then(Value::as_str)
            .or_else(|| configured.as_str())
            .ok_or_else(|| "Audio file reference must contain a path.".to_owned())?;
        let rendered_path = render_template(raw_path, event, globals);
        let file = crate::services::audio_file_ref_from_config(
            &rendered_path,
            self.db.paths().data.as_path(),
        )
        .map_err(|error| error.to_string())?;
        let volume = number_value(config.get("volume"))
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        if !volume.is_finite() {
            return Err("Audio volume must be finite.".to_owned());
        }
        let overlap = match config
            .get("overlap")
            .and_then(Value::as_str)
            .unwrap_or("allow")
        {
            "restart" => tiktools_plugin_api::AudioOverlap::Restart,
            "drop" => tiktools_plugin_api::AudioOverlap::Drop,
            _ => tiktools_plugin_api::AudioOverlap::Allow,
        };
        if test {
            let summary = format!("would play {}", file.name);
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            return Ok(summary);
        }
        self.execution_context_guard(context)?;
        // Playback belongs to an installed, enabled audio provider plugin when one
        // exists. The host media backend is only the fallback for hosts without one.
        if let Some(action_type) = self.audio_provider() {
            let overlap_name = match overlap {
                tiktools_plugin_api::AudioOverlap::Allow => "allow",
                tiktools_plugin_api::AudioOverlap::Restart => "restart",
                tiktools_plugin_api::AudioOverlap::Drop => "drop",
            };
            let file_value = serde_json::to_value(&file)
                .map_err(|error| format!("could not encode audio file: {error}"))?;
            let action = serde_json::json!({
                "typeId": action_type,
                "config": {
                    "fileRef": file_value,
                    "volume": volume,
                    "overlap": overlap_name,
                },
            });
            // Boxed: a provider result may itself return audio intents, which re-enter
            // this function. The indirection keeps the async call graph finite.
            let output = Box::pin(self.execute_plugin_action_output(
                &action_type,
                &action,
                event,
                context,
                logs,
                false,
            ))
            .await?;
            let played = output.result.clone().and_then(|value| {
                serde_json::from_value::<tiktools_plugin_api::AudioPlaybackResult>(value).ok()
            });
            let summary = match played {
                Some(result) => audio_outcome_summary(&file.name, &result),
                None => output.summary,
            };
            tracing::info!(target: "tiktools::automation", file = %file.path, "audio action completed by provider plugin");
            if logs.len() < 40 {
                logs.push(summary.clone());
            }
            return Ok(summary);
        }
        let result = self
            .play_audio(
                file.clone(),
                tiktools_plugin_api::AudioPlayOptions {
                    volume: volume as f32,
                    overlap,
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        let summary = if result.played {
            format!("played {}", file.name)
        } else {
            format!(
                "skipped {}{}",
                file.name,
                result
                    .reason
                    .as_deref()
                    .map(|reason| format!(" ({reason})"))
                    .unwrap_or_default()
            )
        };
        tracing::info!(target: "tiktools::automation", file = %file.path, played = result.played, "audio action completed");
        if logs.len() < 40 {
            logs.push(summary.clone());
        }
        Ok(summary)
    }
}

/// Human summary of a playback outcome, shared by provider and fallback paths.
fn audio_outcome_summary(name: &str, result: &tiktools_plugin_api::AudioPlaybackResult) -> String {
    if result.played {
        format!("played {name}")
    } else {
        format!(
            "skipped {name}{}",
            result
                .reason
                .as_deref()
                .map(|reason| format!(" ({reason})"))
                .unwrap_or_default()
        )
    }
}

impl AppCore {
    /// The action type of the installed, enabled plugin that provides
    /// `audio.play`. Ties are broken by plugin id so routing is deterministic.
    pub(crate) fn audio_provider(&self) -> Option<String> {
        let mut providers: Vec<(String, String)> = self
            .plugins
            .list()
            .into_iter()
            .filter(|plugin| self.plugin_ready(&plugin.manifest.id))
            .filter_map(|plugin| {
                plugin
                    .manifest
                    .capability_provider(tiktools_plugin_api::CAPABILITY_AUDIO_PLAY)
                    .map(|action| (plugin.manifest.id.clone(), action.to_owned()))
            })
            .collect();
        providers.sort();
        providers.into_iter().next().map(|(_, action)| action)
    }
}

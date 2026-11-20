//! Local media actions (audio playback) for automations and scripts.
//!
//! Playback is never performed by the host. The installed audio provider plugin
//! plays the file; this module validates the request and routes it there.

use super::context::ExecutionContext;
use crate::*;

/// Outcome of an audio request: the summary shown to the user and whether the
/// provider reported that sound actually started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AudioOutcome {
    pub(crate) summary: String,
    pub(crate) played: bool,
}

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
        let outcome = self
            .play_with_audio_provider(&file, volume, overlap, event, context, logs)
            .await?;
        if logs.len() < 40 {
            logs.push(outcome.summary.clone());
        }
        Ok(outcome.summary)
    }

    /// Plays a validated file through the installed audio provider plugin.
    /// Fails with a clear message when no provider is installed and ready.
    pub(crate) async fn play_with_audio_provider(
        self: &Arc<Self>,
        file: &MediaFileRef,
        volume: f64,
        overlap: tiktools_plugin_api::AudioOverlap,
        event: &Value,
        context: &ExecutionContext,
        logs: &mut Vec<String>,
    ) -> Result<AudioOutcome, String> {
        let Some(action_type) = self.audio_provider() else {
            return Err(NO_AUDIO_PROVIDER.to_owned());
        };
        let overlap_name = match overlap {
            tiktools_plugin_api::AudioOverlap::Allow => "allow",
            tiktools_plugin_api::AudioOverlap::Restart => "restart",
            tiktools_plugin_api::AudioOverlap::Drop => "drop",
        };
        let file_value = serde_json::to_value(file)
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
        // this path. The indirection keeps the async call graph finite.
        let output = Box::pin(self.execute_plugin_action_output(
            &action_type,
            &action,
            event,
            context,
            logs,
            false,
        ))
        .await?;
        let played = output
            .result
            .clone()
            .and_then(|value| {
                serde_json::from_value::<tiktools_plugin_api::AudioPlaybackResult>(value).ok()
            });
        let outcome = match played {
            Some(result) => AudioOutcome {
                summary: audio_outcome_summary(&file.name, &result),
                played: result.played,
            },
            // A provider that returns no structured outcome is trusted for its summary.
            None => AudioOutcome {
                summary: output.summary,
                played: true,
            },
        };
        tracing::info!(
            target: "tiktools::automation",
            file = %file.path,
            played = outcome.played,
            "audio action completed by provider plugin"
        );
        Ok(outcome)
    }
}

/// Message shown whenever audio is requested and no provider can play it.
pub(crate) const NO_AUDIO_PROVIDER: &str =
    "No audio provider is installed. Install an audio provider plugin, such as the TikTools audio plugin.";

/// Human summary of a playback outcome.
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

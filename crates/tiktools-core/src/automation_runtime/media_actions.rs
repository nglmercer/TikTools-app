//! Local media actions (audio playback) for automations and scripts.

use crate::*;

impl AppCore {
    pub(crate) async fn execute_audio_action(
        self: &Arc<Self>,
        config: &serde_json::Map<String, Value>,
        event: &Value,
        logs: &mut Vec<String>,
        test: bool,
    ) -> Result<String, String> {
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
        let rendered_path = render_template(raw_path, event);
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

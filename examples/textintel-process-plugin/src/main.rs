#![forbid(unsafe_code)]
// Release plugin executables must not allocate a console on Windows; the
// host launches them with CREATE_NO_WINDOW and talks over piped stdio.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use tiktools_plugin_sdk::prelude::*;
use tiktools_textintel_plugin::TextIntelProcessor;

#[derive(Default)]
struct TextIntelPlugin {
    processor: TextIntelProcessor,
}

impl Plugin for TextIntelPlugin {
    fn enrich(
        &mut self,
        _context: &PluginContext,
        request: EventEnrichmentRequest,
    ) -> PluginResult<EventEnrichmentResult> {
        let comment = request
            .event
            .pointer("/data/comment")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let nickname = request
            .event
            .pointer("/user/nickname")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let unique_id = request
            .event
            .pointer("/user/uniqueId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let mut logs = Vec::new();
        let (annotations, views) = self.processor.enrich_texts(
            &request.settings,
            comment,
            nickname,
            unique_id,
            &mut logs,
        )?;
        Ok(EventEnrichmentResult {
            annotations,
            views,
            logs,
        })
    }
}

tiktools_process_plugin!(TextIntelPlugin);

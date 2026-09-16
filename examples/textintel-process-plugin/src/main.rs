#![forbid(unsafe_code)]
// Release plugin executables must not allocate a console on Windows; the
// host launches them with CREATE_NO_WINDOW and talks over piped stdio.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use tiktools_plugin_sdk::prelude::*;
use tiktools_textintel_plugin::{
    annotate_event, build_engine, BoundedCache, EngineMode, TextIntelSettings,
};

struct TextIntelPlugin {
    engine: Option<(EngineMode, textintel::TextIntelligence)>,
    settings_digest: Option<String>,
    comment_cache: BoundedCache,
    name_cache: BoundedCache,
}

impl Default for TextIntelPlugin {
    fn default() -> Self {
        Self {
            engine: None,
            settings_digest: None,
            comment_cache: BoundedCache::new(64),
            name_cache: BoundedCache::new(256),
        }
    }
}

impl TextIntelPlugin {
    /// Builds the engine lazily on first use and rebuilds only when the
    /// engine mode changes. Settings otherwise arrive per request because
    /// the host delivers them inside each enrich call.
    fn ensure_engine(&mut self, mode: EngineMode) -> PluginResult<()> {
        let rebuild = self
            .engine
            .as_ref()
            .is_none_or(|(current, _)| *current != mode);
        if rebuild {
            self.engine = Some((mode, build_engine(mode)?));
        }
        Ok(())
    }
}

impl Plugin for TextIntelPlugin {
    fn enrich(
        &mut self,
        _context: &PluginContext,
        request: EventEnrichmentRequest,
    ) -> PluginResult<EventEnrichmentResult> {
        let settings = TextIntelSettings::from_value(&request.settings);
        let digest = settings.digest();
        if self.settings_digest.as_deref() != Some(digest.as_str()) {
            self.comment_cache.clear();
            self.name_cache.clear();
            self.settings_digest = Some(digest);
        }
        self.ensure_engine(settings.engine_mode)?;
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
        // Disjoint field borrows: the engine is immutable from here on.
        let engine = &self.engine.as_ref().expect("engine was just built").1;
        let (annotations, views) = annotate_event(
            engine,
            &settings,
            comment,
            nickname,
            unique_id,
            &mut self.comment_cache,
            &mut self.name_cache,
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

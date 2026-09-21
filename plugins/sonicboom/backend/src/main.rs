#![forbid(unsafe_code)]
// Release plugin executables must not allocate a console on Windows; the
// host launches them with CREATE_NO_WINDOW and talks over piped stdio.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

//! SonicBoom TTS plugin backend.
//!
//! The backend owns speech: it observes `live.ui-event` chat through the
//! SDK event hook, applies the speech policy (comment triggers, user
//! eligibility, affordability, replay dedup), and posts eligible lines to
//! the configured SonicBoom server for playback. The declarative
//! `speak` / `set-output-device` actions and the voice/output option
//! sources run in the host through the automation HTTP engine — no
//! process call, no foreign code — so this binary only implements the
//! observer half. Settings come from the host settings file (see
//! `config`); the isolated UI edits the same object through the broker.
//!
//! Two host-mediated effects stay out of reach by design: points cannot
//! be charged from a process backend (no host call exists for it), so
//! affordability is checked against the delivered totals but never
//! deducted; and speech status reaches the UI only through the shared
//! logs it already renders, not through a backend event channel.

mod config;
mod policy;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use policy::{Author, Deduper};
use tiktools_plugin_sdk::prelude::*;

const DEDUP_WINDOW_MS: u64 = 1500;
const DEDUP_MAX_ENTRIES: usize = 500;
const VOICES_TTL_SECS: u64 = 300;
const SPEAK_TIMEOUT_SECS: u64 = 15;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

struct SonicBoom {
    http: reqwest::blocking::Client,
    settings: config::ConfigCache,
    deduper: Deduper,
    voices: Vec<String>,
    voices_fetched_at_ms: u64,
    /// Time-seeded xorshift state for random voice picks (no rng dep for
    /// a cosmetic shuffle).
    rng: u64,
}

impl Default for SonicBoom {
    fn default() -> Self {
        Self {
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(SPEAK_TIMEOUT_SECS))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
            settings: config::ConfigCache::new(),
            deduper: Deduper::new(DEDUP_WINDOW_MS, DEDUP_MAX_ENTRIES),
            voices: Vec::new(),
            voices_fetched_at_ms: 0,
            rng: now_millis().max(1),
        }
    }
}

impl SonicBoom {
    fn next_index(&mut self, len: usize) -> usize {
        // xorshift64*: good enough for voice shuffling.
        let mut x = self.rng.max(1);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as usize) % len.max(1)
    }

    /// Refreshes the cached voice list when the policy needs it (random
    /// picks or an empty default) and the cache is stale. Failures keep
    /// the previous list: speech still proceeds with the configured
    /// default, and the server error names a truly unknown voice.
    fn ensure_voices(&mut self, server_url: &str, token: &str, needed: bool) {
        if !needed {
            return;
        }
        let now = now_millis();
        if now.saturating_sub(self.voices_fetched_at_ms) < VOICES_TTL_SECS * 1000
            && !self.voices.is_empty()
        {
            return;
        }
        let url = format!("{server_url}/v1/voices");
        let mut request = self.http.get(&url);
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        let fetched: Option<Vec<String>> = request
            .send()
            .ok()
            .filter(|response| response.status().is_success())
            .and_then(|response| response.json::<serde_json::Value>().ok())
            .map(voice_values);
        if let Some(voices) = fetched {
            self.voices = voices;
            self.voices_fetched_at_ms = now;
        }
    }

    fn speak(
        &self,
        server_url: &str,
        token: &str,
        text: &str,
        voice: &str,
        language: &str,
    ) -> Result<String, String> {
        // Mirrors the declarative speak action: POST
        // `/api/tts/play?voice=…&lang=…` with a text/plain body. The
        // observer never interrupts (`play_now` stays a manual control).
        let url = format!(
            "{server_url}/api/tts/play?voice={}&lang={}",
            encode_param(voice),
            encode_param(language)
        );
        let mut request = self
            .http
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(text.to_owned());
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        let response = request.send().map_err(|error| {
            // Never log the token: only the outcome and status travel.
            format!("speech request failed: {error}")
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("speech server answered {status}"));
        }
        Ok(format!("spoke {} chars as {voice}", text.chars().count()))
    }
}

/// Percent-encodes one query parameter value (unreserved set passes
/// through; everything else becomes `%XX`). The workspace reqwest build
/// omits the `query` serializer, so the backend encodes by hand.
fn encode_param(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// Extracts voice value strings from a `/v1/voices` document, mirroring
/// the host's default option-item mapping (bare strings pass through;
/// objects read `id`, then `value`).
fn voice_values(body: serde_json::Value) -> Vec<String> {
    let items = body.as_array().cloned().unwrap_or_default();
    let mut voices = Vec::new();
    for item in &items {
        if let Some(text) = item.as_str() {
            if !text.trim().is_empty() {
                voices.push(text.trim().to_owned());
            }
            continue;
        }
        let value = item
            .get("id")
            .or_else(|| item.get("value"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(value) = value {
            voices.push(value.to_owned());
        }
    }
    voices
}

impl Plugin for SonicBoom {
    fn event(&mut self, _context: &PluginContext, event: DomainEventEnvelope) -> PluginResult<()> {
        if event.topic != "live.ui-event" {
            return Ok(());
        }
        let Some(chat) = event.data.get("event") else {
            return Ok(());
        };
        if chat.get("kind").and_then(serde_json::Value::as_str) != Some("chat") {
            return Ok(());
        }
        let text = chat
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if text.trim().is_empty() {
            return Ok(());
        }
        let author = Author {
            handle: chat
                .get("author")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("viewer")
                .to_owned(),
            // Delivered totals gate affordability; role flags are absent
            // on this topic, so role-gated rules correctly stay closed.
            points: chat.get("points").and_then(serde_json::Value::as_f64),
            roles: policy::AuthorRoles::default(),
        };
        let config = self.settings.get().clone();
        if !config.policy.enabled {
            return Ok(());
        }
        let voices_needed =
            config.policy.random_voice || config.policy.default_voice.trim().is_empty();
        self.ensure_voices(&config.server_url, &config.api_token, voices_needed);
        let voices = self.voices.clone();
        let decision = policy::decide(text, &author, &config.policy, &voices, |len| {
            self.next_index(len)
        });
        if !decision.speak {
            return Ok(());
        }
        if !self.deduper.claim(
            &policy::fingerprint(&author.handle, &decision.spoken_text),
            now_millis(),
        ) {
            return Ok(());
        }
        match self.speak(
            &config.server_url,
            &config.api_token,
            &decision.spoken_text,
            &decision.voice,
            &decision.language,
        ) {
            Ok(summary) => eprintln!("sonicboom: {summary}"),
            Err(error) => eprintln!("sonicboom: {error}"),
        }
        Ok(())
    }
}

tiktools_process_plugin!(SonicBoom);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_encoding_escapes_reserved_characters() {
        assert_eq!(encode_param("M1"), "M1");
        assert_eq!(encode_param("en-US"), "en-US");
        assert_eq!(encode_param("a b&c=d"), "a%20b%26c%3Dd");
    }

    #[test]
    fn voice_extraction_mirrors_host_option_mapping() {
        let body = serde_json::json!([
            "M1",
            {"id": "F1", "name": "Female 1"},
            {"value": "M2"},
            {"name": "nameless"},
            "",
            42,
        ]);
        assert_eq!(
            voice_values(body),
            vec!["M1".to_owned(), "F1".to_owned(), "M2".to_owned()]
        );
    }

    #[test]
    fn package_manifest_matches_the_backend_contract() {
        // The backend, the manifest, and the UI entry evolve together:
        // this test pins the package contract (webview UI, event
        // subscription, process entry) so drift fails loudly.
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugin.json");
        let raw = std::fs::read_to_string(&manifest_path).expect("plugin.json must exist");
        let manifest =
            tiktools_plugin_sdk::tiktools_plugin_api::manifest::PluginManifest::from_json_str(&raw)
                .expect("plugin.json must parse");
        assert_eq!(manifest.id, "sonicboom.server");
        assert!(
            manifest
                .event_subscriptions
                .iter()
                .any(|subscription| subscription == "live.ui-event"),
            "backend observes live.ui-event"
        );
        let ui = manifest.ui.as_ref().expect("plugin declares its UI");
        assert_eq!(
            ui.mode,
            tiktools_plugin_sdk::tiktools_plugin_api::ui::PluginUiMode::Webview
        );
        let entry = ui.entry.as_deref().expect("webview entry must be set");
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../{entry}"))
                .is_file(),
            "built UI entry must exist at {entry}"
        );
    }
}

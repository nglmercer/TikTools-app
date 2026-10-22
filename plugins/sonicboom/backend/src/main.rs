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
//! deducted; and speech status reaches the UI through the shared,
//! structured stderr lines the host drains and the log panel already
//! renders, not through a backend event channel.
//!
//! Admission contract (Post-refactor):
//!
//! - `event()` performs the cheap, HTTP-free eligibility screen and then
//!   enqueues a self-contained [`SpeechJob`] onto a bounded (64) FIFO. A
//!   full queue is rejected with an exact counter and a rate-limited,
//!   actionable log line; it never blocks ingestion, retries, or hides.
//! - The ordered dispatcher owns catalog lookup, replay dedup and admission
//!   expiry. Supertonic keeps its short synchronous request. Cloned voices
//!   use bounded tasks for idempotent job submission and playback tracking.
//! - Queue admission, inference completion and playback disposition are
//!   separate outcomes. A queued/started response never proves completion.
//! - Disconnect/settings changes cancel outstanding jobs without resetting
//!   message identity dedup. Shutdown joins tracking and retires queued work.
//! - A watchdog flags a stalled worker (`degraded`) after a bounded period
//!   without progression. Recovery is stop/restart only; a stale generation
//!   is cancelled and joined before a replacement starts.

mod config;
mod emotes;
mod playback;
mod policy;
mod sanitizer;
mod transport;

use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use policy::{Author, Deduper};
use tiktools_plugin_sdk::prelude::*;

const DEDUP_WINDOW_MS: u64 = 1500;
const DEDUP_MAX_ENTRIES: usize = 500;
/// True-identity replay suppression is session-scoped and deliberately much
/// longer than the content fallback: a reconnect can replay a message well
/// after the short window without it being a fresh comment.
const EVENT_ID_DEDUP_WINDOW_MS: u64 = 10 * 60_000;
const EVENT_ID_DEDUP_MAX_ENTRIES: usize = 2_000;
const VOICES_TTL_SECS: u64 = 300;
/// Catalog fetch budget: never the synthesis budget; a slow catalog must not
/// consume the speech window.
const VOICE_CATALOG_TIMEOUT_SECS: u64 = 10;
/// Separate connect budget so a black-holed host is distinguishable from a
/// server that accepted but is slow to answer.
const SPEAK_CONNECT_TIMEOUT_SECS: u64 = 5;
/// Hard ceiling for the observer's synchronous request. This is the test seam
/// ceiling and the largest configured provider budget; the effective deadline
/// is the smaller of the provider budget and this value.
const MAX_SPEAK_TIMEOUT_SECS: u64 = 180;
/// Provider-aware synchronous speech budgets, mirrored host-side by the
/// plugin's `timeoutByField` manifest policy and the shared TS
/// `PROVIDER_TTS_BUDGETS_MS`. Supertonic is fast; Pocket clones are slower;
/// Qwen3-GGUF autoregressive cloning is the slowest. These are honest
/// starting hypotheses, not performance guarantees.
const SPEAK_TIMEOUT_SUPERTONIC_SECS: u64 = 15;
const SPEAK_TIMEOUT_POCKET_SECS: u64 = 60;
const SPEAK_TIMEOUT_QWEN_GGUF_SECS: u64 = 180;

/// Effective synchronous speech deadline for one provider. Unknown providers
/// get the conservative Supertonic budget rather than a multi-minute wait.
fn provider_timeout_budget(provider: &str) -> Duration {
    let secs = match provider {
        "pocket-tts" => SPEAK_TIMEOUT_POCKET_SECS,
        "qwen3-gguf" => SPEAK_TIMEOUT_QWEN_GGUF_SECS,
        _ => SPEAK_TIMEOUT_SUPERTONIC_SECS,
    };
    Duration::from_secs(secs)
}
/// Ordinary chat freshness: a job older than this is dropped before it can
/// monopolize the worker. High-value/manual speech is out of the observer's
/// scope (declarative actions bypass this queue entirely).
#[cfg(test)]
const MAX_WAIT_AGE_MS: u64 = 15_000;
const SPEECH_QUEUE_CAPACITY: usize = 64;
/// No progression for this long flips a running worker to `degraded`.
const WATCHDOG_STALL_MS: u64 = 45_000;
const STATUS_LOG_INTERVAL_MS: u64 = 5_000;
const WARN_INTERVAL_MS: u64 = 1_000;
const WATCHDOG_TICK_MS: u64 = 250;

const STATE_IDLE: u8 = 0;
const STATE_RUNNING: u8 = 1;
const STATE_DEGRADED: u8 = 2;
const STATE_STOPPED: u8 = 3;

fn state_name(state: u8) -> &'static str {
    match state {
        STATE_RUNNING => "running",
        STATE_DEGRADED => "degraded",
        STATE_STOPPED => "stopped",
        _ => "idle",
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// Why one speech POST did not complete. Transport errors, HTTP status
/// classes, timeouts and cancellations are distinct so the UI can tell a
/// dead server from a rate limit from a request that was merely slow.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SpeakFailure {
    ConnectTimeout,
    Timeout,
    Disconnected,
    Transport,
    Status(u16),
}

/// A 2xx `POST /api/tts/play`: the line was accepted into the server
/// playback queue. `server_job_id` is the server's queue id when the body
/// parsed, useful for correlating with server logs/queue status. It never
/// proves the line was actually heard.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpeakAccepted {
    server_job_id: Option<String>,
    chars: usize,
    voice: String,
}

impl SpeakFailure {
    fn kind(&self) -> &'static str {
        match self {
            SpeakFailure::ConnectTimeout => "connect_timeout",
            SpeakFailure::Timeout => "timeout",
            SpeakFailure::Disconnected => "disconnected",
            SpeakFailure::Transport => "transport",
            SpeakFailure::Status(408) => "server_http_deadline",
            SpeakFailure::Status(429) => "rate_limited",
            SpeakFailure::Status(code) if *code >= 500 => "server_error",
            SpeakFailure::Status(_) => "client_error",
        }
    }

    fn message(&self) -> String {
        match self {
            SpeakFailure::ConnectTimeout => {
                "could not connect before the connect deadline".to_owned()
            }
            SpeakFailure::Timeout => "server did not answer before the request deadline".to_owned(),
            SpeakFailure::Disconnected => "connection to the server failed".to_owned(),
            SpeakFailure::Transport => "speech request failed in transport".to_owned(),
            SpeakFailure::Status(code) => format!("server answered {code}"),
        }
    }
}

/// Classifies a reqwest error without ever including the URL, token or body.
fn classify_request_error(error: &reqwest::Error) -> SpeakFailure {
    if error.is_timeout() {
        if error.is_connect() {
            SpeakFailure::ConnectTimeout
        } else {
            SpeakFailure::Timeout
        }
    } else if error.is_connect() {
        SpeakFailure::Disconnected
    } else {
        SpeakFailure::Transport
    }
}

/// Exact, payload-free admission and execution counters. Every path through
/// `event()`/`process()` lands in exactly one terminal counter, so totals
/// reconcile (queued = accepted + failed + expired + deduped + filtered +
/// cancelled-of-queued).
#[derive(Default)]
struct SpeechStats {
    observed: AtomicU64,
    eligible: AtomicU64,
    filtered: AtomicU64,
    deduped: AtomicU64,
    queued: AtomicU64,
    rejected: AtomicU64,
    expired: AtomicU64,
    started: AtomicU64,
    accepted: AtomicU64,
    failed: AtomicU64,
    no_voice: AtomicU64,
    generating: AtomicU64,
    completed: AtomicU64,
    playback_queued: AtomicU64,
    playback_started: AtomicU64,
    playback_failed: AtomicU64,
    timed_out: AtomicU64,
    provider_unavailable: AtomicU64,
    cancelled: AtomicU64,
}

/// Shared worker liveness: state, generation fence, monotonic progression
/// and a bounded queue-depth gauge the mpsc sender cannot expose directly.
struct SpeechRuntime {
    state: AtomicU8,
    generation: u64,
    /// Milliseconds since the process epoch; 0 means "no work yet".
    last_progress_ms: AtomicU64,
    current_job_age_ms: AtomicU64,
    queue_depth: AtomicI64,
    last_status_log_ms: AtomicU64,
    last_reject_log_ms: AtomicU64,
    diagnostic: std::sync::Mutex<serde_json::Value>,
    catalog_invalidation: AtomicU64,
    audio_durations: playback::Durations,
}

impl SpeechRuntime {
    fn diagnostic(&self, stage: &str, result: &str) {
        let mut diagnostic = self.diagnostic.lock().unwrap_or_else(|e| e.into_inner());
        diagnostic["stage"] = stage.into();
        diagnostic["lastResult"] = result.into();
    }
    fn new(generation: u64) -> Self {
        Self {
            state: AtomicU8::new(STATE_IDLE),
            generation,
            last_progress_ms: AtomicU64::new(0),
            current_job_age_ms: AtomicU64::new(0),
            queue_depth: AtomicI64::new(0),
            last_status_log_ms: AtomicU64::new(0),
            last_reject_log_ms: AtomicU64::new(0),
            diagnostic: std::sync::Mutex::new(serde_json::json!({"stage":"idle"})),
            catalog_invalidation: AtomicU64::new(0),
            audio_durations: Default::default(),
        }
    }
}

fn watchdog_should_degrade(now_ms: u64, last_progress_ms: u64, stall_ms: u64) -> bool {
    last_progress_ms != 0 && now_ms.saturating_sub(last_progress_ms) > stall_ms
}

/// Session-scoped suppression of upstream message identities. Unlike the
/// content fingerprint fallback this keeps a stable ID distinct from an
/// identical-text duplicate, so two real viewers both speak while a
/// reconnect replay of the same ID is dropped. Bounded and time-evicted.
struct IdDeduper {
    window_ms: u64,
    max_entries: usize,
    seen: std::collections::HashMap<(u64, String), u64>,
}

impl IdDeduper {
    fn new(window_ms: u64, max_entries: usize) -> Self {
        Self {
            window_ms,
            max_entries: max_entries.max(1),
            seen: std::collections::HashMap::new(),
        }
    }

    /// `true` when this identity is new (speak), `false` for a replay.
    fn claim(&mut self, generation: u64, id: &str, now_ms: u64) -> bool {
        let key = (generation, id.to_owned());
        if let Some(last) = self.seen.get(&key) {
            if now_ms.saturating_sub(*last) < self.window_ms {
                return false;
            }
        }
        self.seen.insert(key, now_ms);
        if self.seen.len() > self.max_entries {
            if let Some(oldest) = self
                .seen
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(key, _)| key.clone())
            {
                self.seen.remove(&oldest);
            }
        }
        true
    }
}

struct SpeechEngine {
    http: reqwest::Client,
    deduper: Deduper,
    event_ids: IdDeduper,
    voices: Vec<String>,
    authorized_voices: Vec<String>,
    voices_fetched_at_ms: u64,
    catalog_retry_ms: u64,
    catalog_failures: u32,
    last_empty_diagnostic_at: u64,
    catalog_generation: u64,
    catalog_key: Option<(String, u64, String, String)>,
    /// Time-seeded xorshift state for random voice picks (no rng dep for
    /// a cosmetic shuffle).
    rng: u64,
    request_timeout: Duration,
    stats: Arc<SpeechStats>,
    runtime: Arc<SpeechRuntime>,
    epoch: Instant,
    instance_id: u64,
    tracking: tokio::task::JoinSet<()>,
    tracking_cancel: tokio::sync::watch::Sender<bool>,
}

impl Default for SpeechEngine {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(SPEAK_CONNECT_TIMEOUT_SECS))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            deduper: Deduper::new(DEDUP_WINDOW_MS, DEDUP_MAX_ENTRIES),
            event_ids: IdDeduper::new(EVENT_ID_DEDUP_WINDOW_MS, EVENT_ID_DEDUP_MAX_ENTRIES),
            voices: Vec::new(),
            authorized_voices: Vec::new(),
            voices_fetched_at_ms: 0,
            catalog_retry_ms: 0,
            catalog_failures: 0,
            last_empty_diagnostic_at: 0,
            catalog_generation: 0,
            catalog_key: None,
            rng: now_millis().max(1),
            request_timeout: Duration::from_secs(MAX_SPEAK_TIMEOUT_SECS),
            stats: Arc::new(SpeechStats::default()),
            runtime: Arc::new(SpeechRuntime::new(1)),
            epoch: Instant::now(),
            instance_id: now_millis(),
            tracking: tokio::task::JoinSet::new(),
            tracking_cancel: tokio::sync::watch::channel(false).0,
        }
    }
}

impl SpeechEngine {
    fn new(stats: Arc<SpeechStats>, runtime: Arc<SpeechRuntime>, epoch: Instant) -> Self {
        Self {
            stats,
            runtime,
            epoch,
            ..Self::default()
        }
    }

    /// Test seam: the total speech deadline without rebuilding the client
    /// (connect timeout stays at its configured default).
    #[cfg(test)]
    fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    fn mark_progress(&self) {
        self.runtime
            .last_progress_ms
            .store(self.epoch.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    /// One rate-limited aggregate snapshot. Contains no payloads.
    fn log_status_if_due(&self) {
        let now = now_millis();
        let last = self.runtime.last_status_log_ms.load(Ordering::Relaxed);
        if now.saturating_sub(last) < STATUS_LOG_INTERVAL_MS {
            return;
        }
        if self
            .runtime
            .last_status_log_ms
            .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let s = &self.stats;
        eprintln!(
            "sonicboom: status worker={} generation={} queue_depth={} observed={} eligible={} filtered={} deduped={} queued={} started={} accepted={} failed={} expired={} rejected={} no_voice={} cancelled={}",
            state_name(self.runtime.state.load(Ordering::Relaxed)),
            self.runtime.generation,
            self.runtime.queue_depth.load(Ordering::Relaxed).max(0),
            s.observed.load(Ordering::Relaxed),
            s.eligible.load(Ordering::Relaxed),
            s.filtered.load(Ordering::Relaxed),
            s.deduped.load(Ordering::Relaxed),
            s.queued.load(Ordering::Relaxed),
            s.started.load(Ordering::Relaxed),
            s.accepted.load(Ordering::Relaxed),
            s.failed.load(Ordering::Relaxed),
            s.expired.load(Ordering::Relaxed),
            s.rejected.load(Ordering::Relaxed),
            s.no_voice.load(Ordering::Relaxed),
            s.cancelled.load(Ordering::Relaxed),
        );
    }

    fn next_index(&mut self, len: usize) -> usize {
        // xorshift64*: good enough for voice shuffling.
        let mut x = self.rng.max(1);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as usize) % len.max(1)
    }

    /// Server, credential identity, provider and UI revision scope both positive and
    /// empty catalogs. Explicit reload invalidates immediately; TTL is a fallback.
    #[cfg(test)]
    async fn ensure_voices(&mut self, config: &config::BackendConfig) {
        self.ensure_voices_for(config, &config.policy.provider_id)
            .await;
    }

    async fn ensure_voices_for(&mut self, config: &config::BackendConfig, provider: &str) {
        use std::hash::{Hash, Hasher};
        let mut identity = std::collections::hash_map::DefaultHasher::new();
        config.api_token.hash(&mut identity);
        let key = (
            config.server_url.clone(),
            identity.finish(),
            config.policy.provider_id.clone(),
            config.policy.voice_catalog_revision.clone(),
        );
        let generation = self.runtime.catalog_invalidation.load(Ordering::Relaxed);
        if self.catalog_key.as_ref() != Some(&key) || generation != self.catalog_generation {
            self.catalog_generation = generation;
            self.voices.clear();
            self.authorized_voices.clear();
            self.voices_fetched_at_ms = 0;
            self.catalog_retry_ms = 0;
            self.catalog_failures = 0;
            self.catalog_key = Some(key);
        }
        let server_url = &config.server_url;
        let token = &config.api_token;
        let now = now_millis();
        if now.saturating_sub(self.voices_fetched_at_ms) < self.catalog_retry_ms
            && self.voices_fetched_at_ms != 0
        {
            let prefix = format!("{provider}:");
            self.voices = self
                .authorized_voices
                .iter()
                .filter(|voice| voice.starts_with(&prefix))
                .cloned()
                .collect();
            if self.catalog_failures > 0
                || !self.voices.is_empty()
                || now.saturating_sub(self.voices_fetched_at_ms) < config.empty_catalog_retry_ms
            {
                return;
            }
        }
        let url = format!("{server_url}/api/tts/plugin/voices");
        let mut request = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(VOICE_CATALOG_TIMEOUT_SECS));
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        self.voices_fetched_at_ms = now;
        let fetched = match request.send().await {
            Ok(response) if response.status().is_success() => response
                .json::<serde_json::Value>()
                .await
                .ok()
                .map(voice_values),
            Ok(response) => {
                eprintln!(
                    "sonicboom: voice catalog server answered {}",
                    response.status()
                );
                None
            }
            Err(error) => {
                if error.is_timeout() {
                    self.stats.timed_out.fetch_add(1, Ordering::Relaxed);
                }
                eprintln!(
                    "sonicboom: voice catalog request failed stage={}",
                    classify_request_error(&error).kind()
                );
                None
            }
        };
        self.voices.clear();
        self.authorized_voices.clear();
        if let Some(voices) = fetched {
            self.authorized_voices = voices.clone();
            let prefix = format!("{provider}:");
            self.voices = voices
                .into_iter()
                .filter(|v| v.starts_with(&prefix))
                .collect();
            self.voices_fetched_at_ms = now;
            self.catalog_failures = 0;
            self.catalog_retry_ms = if self.authorized_voices.is_empty() {
                config.empty_catalog_retry_ms
            } else {
                VOICES_TTL_SECS * 1000
            };
        } else {
            self.catalog_failures = self.catalog_failures.saturating_add(1);
            self.catalog_retry_ms = (1000_u64 << self.catalog_failures.min(5)).min(30_000);
        }
        // Backoff starts after completion, including slow failed requests.
        self.voices_fetched_at_ms = now_millis();
    }

    async fn speak(
        &self,
        server_url: &str,
        token: &str,
        text: &str,
        voice: &str,
        provider: &str,
        language: &str,
    ) -> Result<SpeakAccepted, SpeakFailure> {
        // Mirrors the declarative speak action: POST
        // `/api/tts/play?voice=…&lang=…` with a text/plain body. The
        // observer never interrupts (`play_now` stays a manual control).
        let url = format!(
            "{server_url}/api/tts/play?provider={}&voice={}&lang={}",
            encode_param(provider),
            encode_param(voice),
            encode_param(language)
        );
        let mut request = self
            .http
            .post(&url)
            .timeout(provider_timeout_budget(provider).min(self.request_timeout))
            .header("Content-Type", "text/plain")
            .body(text.to_owned());
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        // No retry: a timed-out POST may already have synthesized audio, and
        // the endpoint exposes no idempotency key. A truthful failure beats a
        // duplicated spoken line.
        let response = request
            .send()
            .await
            .map_err(|error| classify_request_error(&error))?;
        let status = response.status();
        if !status.is_success() {
            return Err(SpeakFailure::Status(status.as_u16()));
        }
        // `/api/tts/play` returns `{"success":true,"message":…,"id":…}`. The
        // id correlates with the server playback queue; a 2xx still only
        // proves the line was accepted/queued, never that it was heard. A
        // missing/malformed body is accepted without an id, not failed.
        let server_job_id = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|body| {
                body.get("id")
                    .and_then(serde_json::Value::as_str)
                    .filter(|id| !id.is_empty() && id.len() <= 128)
                    .map(str::to_owned)
            });
        Ok(SpeakAccepted {
            server_job_id,
            chars: text.chars().count(),
            voice: voice.to_owned(),
        })
    }

    /// One full job. Counters are the single source of truth for the
    /// terminal accounting; the caller only owns cancellation and liveness.
    async fn process(&mut self, job: Box<SpeechJob>) {
        let SpeechJob {
            job_id,
            upstream_event_id,
            session_generation,
            admitted,
            admitted_at_ms,
            deadline,
            text,
            author,
            config,
        } = *job;
        self.runtime
            .current_job_age_ms
            .store(admitted.elapsed().as_millis() as u64, Ordering::Relaxed);
        self.mark_progress();
        let event_ref = upstream_event_id.as_deref().unwrap_or("-");

        // Drop stale work before paying for a voice-catalog request.
        if Instant::now() >= deadline {
            self.stats.expired.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "sonicboom: job {job_id} expired stage=admission age_ms={} event={event_ref}",
                admitted.elapsed().as_millis()
            );
            self.log_status_if_due();
            return;
        }

        {
            let mut diagnostic = self
                .runtime
                .diagnostic
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            diagnostic["provider"] = config.policy.provider_id.clone().into();
            diagnostic["voice"] = config.policy.default_voice.clone().into();
        }
        let selected = policy::settings_for_author(&config.policy, &author);
        self.ensure_voices_for(&config, &selected.provider_id).await;
        let mut config = config;
        config.policy = selected;
        if self.voices.is_empty() {
            if self.last_empty_diagnostic_at != self.voices_fetched_at_ms {
                self.last_empty_diagnostic_at = self.voices_fetched_at_ms;
                let response = self
                    .http
                    .get(format!("{}/api/tts/plugin/providers", config.server_url))
                    .bearer_auth(&config.api_token)
                    .timeout(Duration::from_secs(VOICE_CATALOG_TIMEOUT_SECS))
                    .send()
                    .await;
                let state = match response {
                    Ok(response) if response.status().is_success() => response
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|value| {
                            value
                                .as_array()
                                .and_then(|items| {
                                    items
                                        .iter()
                                        .find(|item| item["id"] == config.policy.provider_id)
                                })
                                .cloned()
                        })
                        .and_then(|provider| provider["state"].as_str().map(str::to_owned)),
                    _ => None,
                };
                let reason = match state.as_deref() {
                    Some("ready") => "Provider ready; no authorized compatible voice profiles. Create a profile with this API token and reload voices.",
                    Some(_) => { self.stats.provider_unavailable.fetch_add(1, Ordering::Relaxed); "Provider not ready; load its selected model and retry." },
                    None => "Provider readiness unknown; check server connection and token authorization.",
                };
                self.runtime.diagnostic("catalog", reason);
                eprintln!("sonicboom: job {job_id} failed stage=catalog detail={reason}");
            }
            self.stats.no_voice.fetch_add(1, Ordering::Relaxed);
            self.stats.failed.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "sonicboom: job {job_id} failed reason=no_voices provider={} session={session_generation} event={event_ref}",
                config.policy.provider_id
            );
            self.log_status_if_due();
            return;
        }
        let voices = self.voices.clone();
        let decision = policy::decide(&text, &author, &config.policy, &voices, |len| {
            self.next_index(len)
        });
        if !decision.speak {
            self.stats.filtered.fetch_add(1, Ordering::Relaxed);
            self.log_status_if_due();
            return;
        }
        // Freshness again immediately before dispatch: a slow catalog fetch
        // may have consumed the whole waiting budget.
        if Instant::now() >= deadline {
            self.stats.expired.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "sonicboom: job {job_id} expired stage=dispatch age_ms={} event={event_ref}",
                admitted.elapsed().as_millis()
            );
            self.log_status_if_due();
            return;
        }
        // Primary key is the upstream identity; the content fingerprint is
        // only a fallback for synthetic events that carry no ID, so two
        // distinct messages with identical text both speak.
        let fresh = match upstream_event_id.as_deref() {
            Some(id) => self.event_ids.claim(session_generation, id, now_millis()),
            None => self.deduper.claim(
                &policy::fingerprint(&author.handle, &decision.spoken_text),
                admitted_at_ms,
            ),
        };
        if !fresh {
            self.stats.deduped.fetch_add(1, Ordering::Relaxed);
            self.log_status_if_due();
            return;
        }
        if matches!(
            config.policy.provider_id.as_str(),
            "pocket-tts" | "qwen3-gguf"
        ) {
            // Keep inference reservations bounded locally; SonicBoom remains
            // authoritative and can reject admission with 429.
            while self.tracking.try_join_next().is_some() {}
            if self.tracking.len() >= config.max_inflight {
                self.stats.rejected.fetch_add(1, Ordering::Relaxed);
                eprintln!("sonicboom: job {job_id} rejected stage=inference_admission reason=local_capacity");
                return;
            }
            let client = transport::JobsClient {
                http: self.http.clone(),
                server: config.server_url.clone(),
                token: config.api_token.clone(),
            };
            let key = format!(
                "tiktools-{}-{session_generation}-{job_id}",
                self.instance_id
            );
            let diagnostic_runtime = Arc::clone(&self.runtime);
            let stats = Arc::clone(&self.stats);
            let cancelled = self.tracking_cancel.subscribe();
            let budget = Duration::from_millis(if config.policy.provider_id == "pocket-tts" {
                config.generation_timeout_ms.min(120_000)
            } else {
                config.generation_timeout_ms
            });
            let event_ref = event_ref.to_owned();
            // Submission and polling share a bounded task. Dropping the
            // dispatch future cannot abandon an accepted server job: shutdown
            // joins this task, recovering the ID through idempotent retry.
            self.tracking.spawn(async move {
                let id = match client.submit(&config.policy.provider_id, &decision.voice,
                    &decision.language, &decision.spoken_text, &key).await {
                    Ok(id) => id,
                    Err(error) => {
                        stats.failed.fetch_add(1, Ordering::Relaxed);
                        if error == transport::Failure::Http(503) { stats.provider_unavailable.fetch_add(1, Ordering::Relaxed); }
                        diagnostic_runtime.catalog_invalidation.fetch_add(1, Ordering::Relaxed);
                        diagnostic_runtime.diagnostic("submission", error.guidance());
                        eprintln!("sonicboom: job {job_id} failed stage=submission reason={error:?} event={event_ref}");
                        return;
                    }
                };
                stats.started.fetch_add(1, Ordering::Relaxed);
                diagnostic_runtime.diagnostic("submitted", "Synthesis accepted; playback pending.");
                eprintln!("sonicboom: job {job_id} submitted synthesis_id={id} event={event_ref}");
                let mut previous_stage = String::new();
                let mut generating_seen = false;
                match client.track_with_duration(&id, budget, cancelled, |stage| {
                    if stage != previous_stage {
                        previous_stage = stage.to_owned();
                        if !generating_seen && !matches!(stage, "queued" | "completed" | "failed" | "cancelled") {
                            generating_seen = true; stats.generating.fetch_add(1, Ordering::Relaxed); }
                        if stage == "completed" { stats.completed.fetch_add(1, Ordering::Relaxed); }
                        diagnostic_runtime.diagnostic(stage, "Playback disposition pending.");
                    }
                }).await {
                    Ok((state, queue_id, duration)) => {
                        if let (Some(id), Some(seconds)) = (&queue_id, duration) {
                            let mut known = diagnostic_runtime.audio_durations.lock().unwrap_or_else(|e| e.into_inner());
                            if known.len() >= 128 { if let Some(key) = known.keys().next().cloned() { known.remove(&key); } }
                            known.insert((config.server_url.clone(), config.api_token.clone(), id.clone()), seconds);
                        }
                        diagnostic_runtime.diagnostic("playback", &format!("{state}; completion unverified"));
                        if state == "queued" { stats.playback_queued.fetch_add(1, Ordering::Relaxed); }
                        if state == "started" { stats.playback_started.fetch_add(1, Ordering::Relaxed); }
                        stats.accepted.fetch_add(1, Ordering::Relaxed);
                        eprintln!("sonicboom: job {job_id} synthesis_id={id} playback={state} queue_id={}", queue_id.as_deref().unwrap_or("-"));
                    }
                    Err(transport::Failure::Cancelled) => {
                        stats.cancelled.fetch_add(1, Ordering::Relaxed);
                        diagnostic_runtime.diagnostic("cancelled", transport::Failure::Cancelled.guidance());
                        eprintln!("sonicboom: job {job_id} synthesis_id={id} cancelled stage=tracking");
                    }
                    Err(error) => {
                        if error == transport::Failure::Playback { stats.playback_failed.fetch_add(1, Ordering::Relaxed); }
                        if matches!(error, transport::Failure::Deadline | transport::Failure::RequestTimeout | transport::Failure::InferenceDeadline) { stats.timed_out.fetch_add(1, Ordering::Relaxed); }
                        diagnostic_runtime.catalog_invalidation.fetch_add(1, Ordering::Relaxed);
                        diagnostic_runtime.diagnostic("failed", error.guidance());
                        stats.failed.fetch_add(1, Ordering::Relaxed);
                        eprintln!("sonicboom: job {job_id} synthesis_id={id} failed stage={error:?}");
                    }
                }
            });
            self.mark_progress();
            return;
        }
        self.stats.started.fetch_add(1, Ordering::Relaxed);
        self.mark_progress();
        match self
            .speak(
                &config.server_url,
                &config.api_token,
                &decision.spoken_text,
                &decision.voice,
                &config.policy.provider_id,
                &decision.language,
            )
            .await
        {
            Ok(accepted) => {
                self.runtime.diagnostic(
                    "playback",
                    "Synchronous request accepted; audible completion unverified.",
                );
                self.stats.playback_queued.fetch_add(1, Ordering::Relaxed);
                self.stats.accepted.fetch_add(1, Ordering::Relaxed);
                match accepted.server_job_id.as_deref() {
                    Some(server_id) => eprintln!(
                        "sonicboom: job {job_id} accepted server_id={server_id} {} chars as {} event={event_ref}",
                        accepted.chars, accepted.voice
                    ),
                    None => eprintln!(
                        "sonicboom: job {job_id} accepted server_id=- {} chars as {} event={event_ref}",
                        accepted.chars, accepted.voice
                    ),
                }
            }
            Err(failure) => {
                if matches!(
                    failure,
                    SpeakFailure::Timeout
                        | SpeakFailure::ConnectTimeout
                        | SpeakFailure::Status(408)
                ) {
                    self.stats.timed_out.fetch_add(1, Ordering::Relaxed);
                }
                if failure == SpeakFailure::Status(503) {
                    self.stats
                        .provider_unavailable
                        .fetch_add(1, Ordering::Relaxed);
                }
                self.runtime.diagnostic(failure.kind(), &failure.message());
                self.stats.failed.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "sonicboom: job {job_id} failed reason={} detail={} event={event_ref}",
                    failure.kind(),
                    failure.message()
                );
            }
        }
        self.mark_progress();
        self.log_status_if_due();
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

/// Extracts voice value strings from the plugin catalog, mirroring
/// the host's default option-item mapping (bare strings pass through;
/// objects read `id`, then `value`).
fn voice_values(body: serde_json::Value) -> Vec<String> {
    let items = body.as_array().cloned().unwrap_or_default();
    let mut voices = Vec::new();
    for item in items.iter().take(500) {
        if item.get("disabled").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
        }
        if let Some(text) = item.as_str() {
            if !text.trim().is_empty() && text.len() <= 256 {
                voices.push(text.trim().to_owned());
            }
            continue;
        }
        let value = item
            .get("id")
            .or_else(|| item.get("value"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 256);
        if let Some(value) = value {
            voices.push(value.to_owned());
        }
    }
    voices
}

/// Stable upstream identity for a chat line, when one exists. Used for
/// traceability now and true-identity dedup later; it is an opaque
/// identifier, never chat content.
fn extract_msg_id(chat: &serde_json::Value) -> Option<String> {
    match chat.get("msgId") {
        Some(serde_json::Value::String(text)) if !text.is_empty() && text.len() <= 128 => {
            Some(text.clone())
        }
        Some(serde_json::Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

/// One admitted unit of work, self-contained so settings reloads never
/// mutate an in-flight job and a cancel/restart can retire it cleanly.
struct SpeechJob {
    job_id: u64,
    upstream_event_id: Option<String>,
    session_generation: u64,
    /// Monotonic admission instant for elapsed-age and freshness decisions.
    admitted: Instant,
    /// Wall-clock admission ms for the replay-dedup window.
    admitted_at_ms: u64,
    deadline: Instant,
    text: String,
    author: Author,
    config: config::BackendConfig,
}

struct SonicBoom {
    playback: Option<playback::PlaybackMonitor>,
    settings: config::ConfigCache,
    stats: Arc<SpeechStats>,
    runtime: Arc<SpeechRuntime>,
    next_job_id: u64,
    jobs: Option<tokio::sync::mpsc::Sender<Box<SpeechJob>>>,
    cancel: tokio::sync::watch::Sender<bool>,
    disconnect: tokio::sync::watch::Sender<u64>,
    last_diagnostic: serde_json::Value,
    worker: Option<std::thread::JoinHandle<()>>,
    watchdog: Option<std::thread::JoinHandle<()>>,
    /// Dropping the sender wakes the watchdog immediately (disconnected).
    watchdog_wake: Option<std::sync::mpsc::Sender<()>>,
}

impl Default for SonicBoom {
    fn default() -> Self {
        let epoch = Instant::now();
        let generation = 1;
        let stats = Arc::new(SpeechStats::default());
        let runtime = Arc::new(SpeechRuntime::new(generation));
        let (jobs, mut receiver) = tokio::sync::mpsc::channel(SPEECH_QUEUE_CAPACITY);
        let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
        let (disconnect, mut disconnected) = tokio::sync::watch::channel(0_u64);
        let (watchdog_wake, watchdog_rx) = std::sync::mpsc::channel::<()>();

        let worker_stats = Arc::clone(&stats);
        let worker_runtime = Arc::clone(&runtime);
        let worker = std::thread::Builder::new()
            .name("sonicboom-speech".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("speech runtime");
                runtime.block_on(async move {
                    let mut engine = SpeechEngine::new(
                        Arc::clone(&worker_stats),
                        Arc::clone(&worker_runtime),
                        epoch,
                    );
                    loop {
                        let job: Box<SpeechJob> = tokio::select! {
                            biased;
                            _ = cancelled.changed() => break,
                            _ = disconnected.changed() => {
                                let _ = engine.tracking_cancel.send(true);
                                while engine.tracking.join_next().await.is_some() {}
                                engine.tracking_cancel = tokio::sync::watch::channel(false).0;
                                while receiver.try_recv().is_ok() {
                                    worker_runtime.queue_depth.fetch_sub(1, Ordering::Relaxed);
                                    worker_stats.cancelled.fetch_add(1, Ordering::Relaxed);
                                }
                                continue;
                            },
                            job = receiver.recv() => match job { Some(job) => job, None => break },
                        };
                        worker_runtime.queue_depth.fetch_sub(1, Ordering::Relaxed);
                        worker_runtime.state.store(STATE_RUNNING, Ordering::Relaxed);
                        let age = job.admitted.elapsed().as_millis() as u64;
                        worker_runtime
                            .current_job_age_ms
                            .store(age, Ordering::Relaxed);
                        tokio::select! {
                            biased;
                            _ = cancelled.changed() => {
                                worker_stats.cancelled.fetch_add(1, Ordering::Relaxed);
                                break;
                            },
                            _ = disconnected.changed() => {
                                worker_stats.cancelled.fetch_add(1, Ordering::Relaxed);
                                let _ = engine.tracking_cancel.send(true);
                                while engine.tracking.join_next().await.is_some() {}
                                engine.tracking_cancel = tokio::sync::watch::channel(false).0;
                                while receiver.try_recv().is_ok() {
                                    worker_runtime.queue_depth.fetch_sub(1, Ordering::Relaxed);
                                    worker_stats.cancelled.fetch_add(1, Ordering::Relaxed);
                                }
                            },
                            () = engine.process(job) => {}
                        }
                        worker_runtime.state.store(STATE_IDLE, Ordering::Relaxed);
                    }
                    let _ = engine.tracking_cancel.send(true);
                    while engine.tracking.join_next().await.is_some() {}
                    worker_runtime.state.store(STATE_STOPPED, Ordering::Relaxed);
                    receiver.close();
                    // Everything still queued at shutdown is retired with an
                    // exact count; nothing is silently dropped.
                    let pending = receiver.len();
                    worker_runtime.queue_depth.store(0, Ordering::Relaxed);
                    worker_stats
                        .cancelled
                        .fetch_add(pending as u64, Ordering::Relaxed);
                    if pending > 0 {
                        eprintln!("sonicboom: cancelled {pending} queued speech jobs");
                    }
                });
            })
            .expect("speech worker");

        let watchdog_runtime = Arc::clone(&runtime);
        let watchdog = std::thread::Builder::new()
            .name("sonicboom-watchdog".into())
            .spawn(move || {
                loop {
                    // A disconnected sender (stop) ends the loop immediately;
                    // otherwise the tick bounds how quickly a stall is seen.
                    match watchdog_rx.recv_timeout(Duration::from_millis(WATCHDOG_TICK_MS)) {
                        Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    let now = epoch.elapsed().as_millis() as u64;
                    let last = watchdog_runtime.last_progress_ms.load(Ordering::Relaxed);
                    if watchdog_should_degrade(now, last, WATCHDOG_STALL_MS)
                        && watchdog_runtime.state.load(Ordering::Relaxed) == STATE_RUNNING
                        && watchdog_runtime
                            .state
                            .compare_exchange(
                                STATE_RUNNING,
                                STATE_DEGRADED,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            )
                            .is_ok()
                    {
                        eprintln!(
                            "sonicboom: watchdog flagged stalled worker age_ms={} stall_ms={} generation={}",
                            watchdog_runtime.current_job_age_ms.load(Ordering::Relaxed),
                            WATCHDOG_STALL_MS,
                            watchdog_runtime.generation
                        );
                    }
                }
            })
            .ok();

        Self {
            playback: None,
            settings: config::ConfigCache::new(),
            stats,
            runtime,
            next_job_id: 1,
            jobs: Some(jobs),
            cancel,
            disconnect,
            last_diagnostic: serde_json::Value::Null,
            worker: Some(worker),
            watchdog,
            watchdog_wake: Some(watchdog_wake),
        }
    }
}

impl SonicBoom {
    /// HTTP-free eligibility screen plus bounded FIFO admission. Kept
    /// separate from `event()` so admission accounting is unit-testable
    /// without a settings file or process environment.
    /// Returns `false` only when the worker sender is gone (plugin
    /// unavailable), so the caller can surface a truthful error. Filtering
    /// and rejection are accounted internally and return `true`.
    fn admit(
        &mut self,
        text: &str,
        author: Author,
        config: config::BackendConfig,
        upstream_event_id: Option<String>,
    ) -> bool {
        if !config.policy.enabled {
            return true;
        }
        if !policy::decide(text, &author, &config.policy, &[], |_| 0).speak {
            self.stats.filtered.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        self.stats.eligible.fetch_add(1, Ordering::Relaxed);
        if self.runtime.queue_depth.load(Ordering::Relaxed) >= config.queue_capacity as i64 {
            self.stats.rejected.fetch_add(1, Ordering::Relaxed);
            eprintln!("sonicboom: speech rejected reason=configured_queue_capacity");
            return true;
        }
        let admitted = Instant::now();
        let max_wait_age_ms = config.max_wait_age_ms;
        let job = Box::new(SpeechJob {
            job_id: self.next_job_id,
            upstream_event_id,
            session_generation: self.runtime.generation,
            admitted,
            admitted_at_ms: now_millis(),
            deadline: admitted + Duration::from_millis(max_wait_age_ms),
            text: text.to_owned(),
            author,
            config,
        });
        self.next_job_id = self.next_job_id.wrapping_add(1);
        match self.jobs.as_ref().map(|jobs| jobs.try_send(job)) {
            Some(Ok(())) => {
                self.stats.queued.fetch_add(1, Ordering::Relaxed);
                self.runtime.queue_depth.fetch_add(1, Ordering::Relaxed);
                true
            }
            Some(Err(tokio::sync::mpsc::error::TrySendError::Full(_))) => {
                // Truthful rejection: counted, rate-limited, actionable. The
                // event dispatch stays non-blocking and never retries.
                self.stats.rejected.fetch_add(1, Ordering::Relaxed);
                let now = now_millis();
                let last = self.runtime.last_reject_log_ms.load(Ordering::Relaxed);
                if now.saturating_sub(last) >= WARN_INTERVAL_MS
                    && self
                        .runtime
                        .last_reject_log_ms
                        .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                {
                    eprintln!(
                        "sonicboom: speech rejected reason=queue_full capacity={SPEECH_QUEUE_CAPACITY} queue_depth={} total_rejected={} generation={}",
                        self.runtime.queue_depth.load(Ordering::Relaxed).max(0),
                        self.stats.rejected.load(Ordering::Relaxed),
                        self.runtime.generation
                    );
                }
                true
            }
            // Sender gone: the worker stopped. Report unavailable so the
            // host records a delivery failure instead of a false success.
            _ => false,
        }
    }

    fn stop(&mut self) {
        let _ = self.cancel.send(true);
        self.jobs.take();
        // Dropping the wake sender ends the watchdog loop immediately.
        self.watchdog_wake.take();
        // Join the worker before the watchdog so a stalled HTTP future is
        // dropped, then join the watchdog.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
        self.runtime.state.store(STATE_STOPPED, Ordering::Relaxed);
        let s = &self.stats;
        eprintln!(
            "sonicboom: totals worker={} generation={} observed={} eligible={} filtered={} deduped={} queued={} started={} accepted={} failed={} expired={} rejected={} no_voice={} cancelled={}",
            state_name(self.runtime.state.load(Ordering::Relaxed)),
            self.runtime.generation,
            s.observed.load(Ordering::Relaxed),
            s.eligible.load(Ordering::Relaxed),
            s.filtered.load(Ordering::Relaxed),
            s.deduped.load(Ordering::Relaxed),
            s.queued.load(Ordering::Relaxed),
            s.started.load(Ordering::Relaxed),
            s.accepted.load(Ordering::Relaxed),
            s.failed.load(Ordering::Relaxed),
            s.expired.load(Ordering::Relaxed),
            s.rejected.load(Ordering::Relaxed),
            s.no_voice.load(Ordering::Relaxed),
            s.cancelled.load(Ordering::Relaxed),
        );
    }
}

impl Drop for SonicBoom {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Plugin for SonicBoom {
    fn action(&mut self, _context: &PluginContext, call: ActionCall) -> PluginResult<ActionResult> {
        if call.action_type() != Some("sonicboom.server.playback-status") {
            return Err(PluginError::unsupported(
                call.action_type().unwrap_or("unknown"),
            ));
        }
        let config = self.settings.get().clone();
        let snapshot = self
            .playback
            .get_or_insert_with(|| {
                playback::PlaybackMonitor::new(Arc::clone(&self.runtime.audio_durations))
            })
            .refresh(config);
        Ok(ActionResult::summary(snapshot.to_string()))
    }

    fn poll(&mut self, _context: &PluginContext) -> PluginResult<PollResult> {
        let mut diagnostic = self
            .runtime
            .diagnostic
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        diagnostic["queueDepth"] = self
            .runtime
            .queue_depth
            .load(Ordering::Relaxed)
            .max(0)
            .into();
        diagnostic["connection"] = state_name(self.runtime.state.load(Ordering::Relaxed)).into();
        let stats = &self.stats;
        diagnostic["counters"] = serde_json::json!({
            "observed":stats.observed.load(Ordering::Relaxed), "filtered":stats.filtered.load(Ordering::Relaxed),
            "expired":stats.expired.load(Ordering::Relaxed), "missingVoices":stats.no_voice.load(Ordering::Relaxed),
            "submitted":stats.started.load(Ordering::Relaxed), "playbackAdmitted":stats.accepted.load(Ordering::Relaxed),
            "failed":stats.failed.load(Ordering::Relaxed), "cancelled":stats.cancelled.load(Ordering::Relaxed),
            "saturation":stats.rejected.load(Ordering::Relaxed), "duplicates":stats.deduped.load(Ordering::Relaxed),
            "running":stats.generating.load(Ordering::Relaxed), "completed":stats.completed.load(Ordering::Relaxed),
            "playbackQueued":stats.playback_queued.load(Ordering::Relaxed), "playbackStarted":stats.playback_started.load(Ordering::Relaxed),
            "playbackFailed":stats.playback_failed.load(Ordering::Relaxed), "timedOut":stats.timed_out.load(Ordering::Relaxed),
            "providerUnavailable":stats.provider_unavailable.load(Ordering::Relaxed)
        });
        if self.last_diagnostic == diagnostic {
            return Ok(PollResult::default());
        }
        self.last_diagnostic = diagnostic.clone();
        Ok(PollResult::default().event(PluginEvent::new("sonicboom.speech-status", diagnostic)?))
    }

    fn shutdown(&mut self, _context: &PluginContext) -> PluginResult<()> {
        self.stop();
        Ok(())
    }

    fn event(&mut self, _context: &PluginContext, event: DomainEventEnvelope) -> PluginResult<()> {
        if event.topic == "live.disconnected"
            || (event.topic == "plugin.settings-changed"
                && event
                    .data
                    .get("pluginId")
                    .and_then(serde_json::Value::as_str)
                    == Some("sonicboom.server"))
        {
            self.disconnect
                .send_modify(|generation| *generation = generation.wrapping_add(1));
            return Ok(());
        }
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
        self.stats.observed.fetch_add(1, Ordering::Relaxed);
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
        // Eligibility needs no HTTP. Voice choice and replay dedup run in
        // the ordered worker with the admission timestamp.
        let upstream_event_id = extract_msg_id(chat);
        if !self.admit(text, author, config, upstream_event_id) {
            return Err(PluginError::other("speech worker stopped"));
        }
        Ok(())
    }
}

tiktools_process_plugin!(SonicBoom);

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_policy() -> policy::TtsSettings {
        policy::TtsSettings {
            enabled: true,
            default_voice: "M1".into(),
            ..Default::default()
        }
    }

    fn backend_config(server_url: String) -> config::BackendConfig {
        config::BackendConfig {
            server_url,
            policy: enabled_policy(),
            ..Default::default()
        }
    }

    fn author() -> Author {
        Author {
            handle: "viewer".into(),
            points: None,
            roles: Default::default(),
        }
    }

    fn speech_job(server_url: String) -> Box<SpeechJob> {
        job_with_id(None, "test", server_url)
    }

    fn job_with_id(
        upstream_event_id: Option<&str>,
        text: &str,
        server_url: String,
    ) -> Box<SpeechJob> {
        let admitted = Instant::now();
        Box::new(SpeechJob {
            job_id: 1,
            upstream_event_id: upstream_event_id.map(str::to_owned),
            session_generation: 1,
            admitted,
            admitted_at_ms: now_millis(),
            deadline: admitted + Duration::from_millis(MAX_WAIT_AGE_MS),
            text: text.into(),
            author: author(),
            config: backend_config(server_url),
        })
    }

    /// A stub that answers `requests` connections: every `GET
    /// /api/tts/plugin/voices` gets a catalog, every `POST /api/tts/play`
    /// gets `play_status_line`. Bodies are not consumed: the request line is
    /// enough to route.
    fn serve_voices_and_plays(
        listener: std::net::TcpListener,
        requests: usize,
        play_status_line: &'static str,
    ) -> std::thread::JoinHandle<()> {
        use std::io::{Read, Write};
        std::thread::spawn(move || {
            for _ in 0..requests {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut raw = Vec::new();
                let mut chunk = [0_u8; 1024];
                loop {
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => {
                            raw.extend_from_slice(&chunk[..read]);
                            if raw.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let text = String::from_utf8_lossy(&raw);
                let line = text.lines().next().unwrap_or_default();
                if line.starts_with("GET /api/tts/plugin/voices") {
                    let payload = r#"[{"id":"supertonic:M1"}]"#;
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .as_bytes(),
                    );
                } else if line.starts_with("POST /api/tts/play") {
                    let payload = r#"{"success":true,"message":"Added to queue","id":"srv-1"}"#;
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 {play_status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .as_bytes(),
                    );
                } else {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                }
            }
        })
    }

    #[test]
    fn slow_http_does_not_block_admission_and_shutdown_cancels_full_queue() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (started, ready) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0; 4096];
            assert!(stream.read(&mut buffer).unwrap() > 0);
            started.send(()).unwrap();
            // Keep the HTTP response unavailable until the test has stopped
            // the plugin; this exercises cancellation rather than a timeout.
            let _ = released.recv_timeout(Duration::from_secs(5));
        });
        let mut plugin = SonicBoom::default();
        assert!(plugin
            .jobs
            .as_ref()
            .unwrap()
            .try_send(speech_job(url.clone()))
            .is_ok());
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        for _ in 0..SPEECH_QUEUE_CAPACITY {
            assert!(plugin
                .jobs
                .as_ref()
                .unwrap()
                .try_send(speech_job(url.clone()))
                .is_ok());
        }
        assert!(matches!(
            plugin.jobs.as_ref().unwrap().try_send(speech_job(url)),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_))
        ));
        let start = std::time::Instant::now();
        plugin.stop();
        assert!(start.elapsed() < Duration::from_secs(1));
        plugin.stop();
        let _ = release.send(());
        server.join().unwrap();
    }

    #[test]
    fn burst_reports_bounded_queue_with_exact_accounting() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (started, ready) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0; 4096];
            assert!(stream.read(&mut buffer).unwrap() > 0);
            started.send(()).unwrap();
            let _ = released.recv_timeout(Duration::from_secs(10));
        });
        let mut plugin = SonicBoom::default();
        let config = backend_config(url);
        // 1,000 eligible events against a server that never answers. The
        // worker occupies one slot; the FIFO holds the rest; the surplus is
        // rejected. Memory stays bounded and every event is accounted.
        for _ in 0..1_000 {
            plugin.admit("hello world", author(), config.clone(), None);
        }
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        let queued = plugin.stats.queued.load(Ordering::Relaxed);
        let rejected = plugin.stats.rejected.load(Ordering::Relaxed);
        assert_eq!(queued + rejected, 1_000, "no event is silently lost");
        assert!(
            queued <= (SPEECH_QUEUE_CAPACITY as u64) + 1,
            "queue stays bounded (one in flight): queued={queued}"
        );
        assert!(rejected > 0, "surplus must be rejected, not buffered");
        assert_eq!(
            plugin.runtime.queue_depth.load(Ordering::Relaxed),
            queued as i64 - 1
        );
        plugin.stop();
        let _ = release.send(());
        server.join().unwrap();
    }

    #[tokio::test]
    async fn expired_job_never_reaches_the_server() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let mut engine = SpeechEngine::default();
        let stats = Arc::clone(&engine.stats);
        let past = Instant::now() - Duration::from_secs(5);
        let job = Box::new(SpeechJob {
            job_id: 7,
            upstream_event_id: Some("m-1".into()),
            session_generation: 1,
            admitted: past,
            admitted_at_ms: now_millis() - 5_000,
            deadline: past,
            text: "stale".into(),
            author: author(),
            config: backend_config(url),
        });
        engine.process(job).await;
        assert_eq!(stats.expired.load(Ordering::Relaxed), 1);
        assert_eq!(stats.started.load(Ordering::Relaxed), 0);
        assert!(listener.accept().is_err(), "no HTTP request may be sent");
    }

    #[tokio::test]
    async fn server_error_status_is_a_failure_not_a_success() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = serve_voices_and_plays(listener, 2, "500 Internal Server Error");
        let mut engine = SpeechEngine::default();
        let stats = Arc::clone(&engine.stats);
        engine.process(speech_job(url)).await;
        assert_eq!(stats.started.load(Ordering::Relaxed), 1);
        assert_eq!(stats.accepted.load(Ordering::Relaxed), 0);
        assert_eq!(stats.failed.load(Ordering::Relaxed), 1);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn accepted_response_counts_as_accepted() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = serve_voices_and_plays(listener, 2, "200 OK");
        let mut engine = SpeechEngine::default();
        let stats = Arc::clone(&engine.stats);
        engine.process(speech_job(url)).await;
        assert_eq!(stats.started.load(Ordering::Relaxed), 1);
        assert_eq!(stats.accepted.load(Ordering::Relaxed), 1);
        assert_eq!(stats.failed.load(Ordering::Relaxed), 0);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn accepted_speak_exposes_server_queue_id_for_correlation() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = serve_voices_and_plays(listener, 1, "200 OK");
        let engine = SpeechEngine::default();
        let accepted = engine
            .speak(&url, "", "hello", "supertonic:M1", "supertonic", "en")
            .await
            .unwrap();
        assert_eq!(accepted.server_job_id.as_deref(), Some("srv-1"));
        assert_eq!(accepted.chars, 5);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn accepted_speak_without_json_body_is_still_accepted() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = [0_u8; 1024];
            let _ = stream.read(&mut raw);
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        });
        let engine = SpeechEngine::default();
        let accepted = engine
            .speak(&url, "", "hello", "supertonic:M1", "supertonic", "en")
            .await
            .unwrap();
        assert_eq!(accepted.server_job_id, None);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn distinct_message_ids_with_identical_text_both_speak_and_replay_is_suppressed() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        // One voices lookup (cached after the first job) plus exactly two
        // speech POSTs. A replay of the first ID must not open a third.
        let server = serve_voices_and_plays(listener, 3, "200 OK");
        let mut engine = SpeechEngine::default();
        let stats = Arc::clone(&engine.stats);
        engine
            .process(job_with_id(Some("m-1"), "hello", url.clone()))
            .await;
        engine
            .process(job_with_id(Some("m-2"), "hello", url.clone()))
            .await;
        assert_eq!(stats.accepted.load(Ordering::Relaxed), 2);
        assert_eq!(stats.deduped.load(Ordering::Relaxed), 0);
        // Replay of an already-seen upstream identity is dropped.
        engine.process(job_with_id(Some("m-1"), "hello", url)).await;
        assert_eq!(stats.accepted.load(Ordering::Relaxed), 2);
        assert_eq!(stats.deduped.load(Ordering::Relaxed), 1);
        server.join().unwrap();
    }

    #[test]
    fn id_deduper_separates_identities_sessions_and_windows() {
        let mut deduper = IdDeduper::new(10_000, 10);
        assert!(deduper.claim(1, "a", 1_000));
        assert!(!deduper.claim(1, "a", 5_000));
        // Same id under a different session generation is a fresh identity.
        assert!(deduper.claim(2, "a", 5_000));
        // Window expiry allows the identity again.
        assert!(deduper.claim(1, "a", 12_000));
        assert!(!deduper.claim(1, "a", 12_500));
    }

    #[test]
    fn id_deduper_evicts_the_oldest_at_capacity() {
        let mut deduper = IdDeduper::new(1_000_000, 2);
        assert!(deduper.claim(1, "a", 1_000));
        assert!(deduper.claim(1, "b", 2_000));
        // Capacity is 2: inserting "c" evicts the oldest ("a").
        assert!(deduper.claim(1, "c", 3_000));
        assert!(deduper.claim(1, "a", 4_000));
        // "c" is still retained, so its replay is suppressed.
        assert!(!deduper.claim(1, "c", 4_100));
    }

    #[tokio::test]
    async fn slow_server_times_out_without_counting_success() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut raw = Vec::new();
                let mut chunk = [0_u8; 1024];
                loop {
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => {
                            raw.extend_from_slice(&chunk[..read]);
                            if raw.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                if index == 0 {
                    let payload = r#"[{"id":"supertonic:M1"}]"#;
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                            payload.len()
                        )
                        .as_bytes(),
                    );
                } else {
                    // Hold the connection past the request deadline.
                    std::thread::sleep(Duration::from_millis(1_000));
                }
            }
        });
        let mut engine = SpeechEngine::default().with_request_timeout(Duration::from_millis(200));
        let stats = Arc::clone(&engine.stats);
        engine.process(speech_job(url)).await;
        assert_eq!(stats.started.load(Ordering::Relaxed), 1);
        assert_eq!(stats.accepted.load(Ordering::Relaxed), 0);
        assert_eq!(stats.failed.load(Ordering::Relaxed), 1);
        server.join().unwrap();
    }

    #[test]
    fn watchdog_flags_only_a_running_stall_after_progress() {
        assert!(!watchdog_should_degrade(1_000, 0, 500));
        assert!(!watchdog_should_degrade(1_400, 1_000, 500));
        assert!(watchdog_should_degrade(1_600, 1_000, 500));
    }

    #[test]
    fn message_ids_extract_from_strings_and_numbers_without_content() {
        assert_eq!(
            extract_msg_id(&serde_json::json!({"msgId": "abc"})),
            Some("abc".to_owned())
        );
        assert_eq!(
            extract_msg_id(&serde_json::json!({"msgId": 42})),
            Some("42".to_owned())
        );
        assert_eq!(extract_msg_id(&serde_json::json!({})), None);
        assert_eq!(extract_msg_id(&serde_json::json!({"msgId": ""})), None);
    }

    #[test]
    fn repeated_speech_worker_creation_and_teardown() {
        for _ in 0..128 {
            let mut plugin = SonicBoom::default();
            plugin.stop();
            assert!(plugin.worker.is_none());
            assert!(plugin.watchdog.is_none());
            assert!(plugin.jobs.is_none());
        }
    }

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
    #[test]
    fn provider_budgets_are_bounded_and_unknown_providers_stay_conservative() {
        // Provider-aware synchronous budgets: Supertonic is fast, Pocket clones
        // are slower, Qwen GGUF autoregressive cloning is the slowest.
        assert_eq!(
            provider_timeout_budget("supertonic"),
            Duration::from_secs(15)
        );
        assert_eq!(
            provider_timeout_budget("pocket-tts"),
            Duration::from_secs(60)
        );
        assert_eq!(
            provider_timeout_budget("qwen3-gguf"),
            Duration::from_secs(180)
        );
        // Unknown and empty providers never get a multi-minute wait.
        assert_eq!(provider_timeout_budget(""), Duration::from_secs(15));
        assert_eq!(
            provider_timeout_budget("mystery-tts"),
            Duration::from_secs(15)
        );
        // The engine ceiling always bounds the effective deadline, so a larger
        // future table entry still saturates at the hard cap.
        let engine = SpeechEngine::default();
        for provider in ["supertonic", "pocket-tts", "qwen3-gguf", "unknown"] {
            let effective = provider_timeout_budget(provider).min(engine.request_timeout);
            assert!(effective <= Duration::from_secs(MAX_SPEAK_TIMEOUT_SECS));
        }
    }

    #[tokio::test]
    async fn catalog_cache_tracks_revision_provider_and_auth_even_with_explicit_default() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (report, received) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            for (code, body) in [
                (
                    "200 OK",
                    r#"[{"id":"supertonic:M1"},{"id":"pocket-tts:old"}]"#,
                ),
                ("200 OK", r#"[{"id":"supertonic:F1"}]"#),
                (
                    "200 OK",
                    r#"[{"id":"pocket-tts:new"},{"id":"qwen3-gguf:foreign"}]"#,
                ),
                ("403 Forbidden", "{}"),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut raw = [0_u8; 4096];
                let n = stream.read(&mut raw).unwrap();
                let line = String::from_utf8_lossy(&raw[..n])
                    .lines()
                    .next()
                    .unwrap()
                    .to_owned();
                report.send(line).unwrap();
                write!(stream, "HTTP/1.1 {code}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        let mut engine = SpeechEngine::default();
        let mut config = config::BackendConfig {
            server_url: format!("http://{address}"),
            ..Default::default()
        };
        config.policy.default_voice = "M1".into();
        engine.ensure_voices(&config).await;
        assert_eq!(engine.voices, ["supertonic:M1"]);
        engine.ensure_voices_for(&config, "pocket-tts").await;
        assert_eq!(engine.voices, ["pocket-tts:old"]);
        engine.ensure_voices(&config).await; // Provider overrides share the authenticated catalog.
        assert_eq!(engine.voices, ["supertonic:M1"]);
        assert_eq!(received.try_iter().count(), 1);
        config.policy.voice_catalog_revision = "new-ui-reload".into();
        engine.ensure_voices(&config).await;
        assert_eq!(engine.voices, ["supertonic:F1"]);
        config.policy.provider_id = "pocket-tts".into();
        engine.ensure_voices(&config).await;
        assert_eq!(engine.voices, ["pocket-tts:new"]);
        config.api_token = "test-other-tenant".into();
        engine.ensure_voices(&config).await;
        assert!(engine.voices.is_empty());
        assert!(!format!("{:?}", engine.catalog_key).contains(&config.api_token));
        engine.ensure_voices(&config).await; // Empty/error catalogs are cached too.
        server.join().unwrap();
        assert_eq!(
            received.try_iter().collect::<Vec<_>>(),
            vec!["GET /api/tts/plugin/voices HTTP/1.1"; 3]
        );
    }
    #[tokio::test]
    async fn empty_catalog_retries_quickly_and_failures_back_off() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (status, body) in [
                ("200 OK", "[]"),
                ("503 Unavailable", "{}"),
                ("200 OK", r#"[{"id":"qwen3-gguf:p"}]"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        let mut config = config::BackendConfig {
            server_url: format!("http://{address}"),
            ..Default::default()
        };
        config.policy.provider_id = "qwen3-gguf".into();
        let mut engine = SpeechEngine::default();
        engine.ensure_voices(&config).await;
        assert!(engine.voices.is_empty());
        assert_eq!(engine.catalog_retry_ms, 2000);
        engine.ensure_voices(&config).await; // Burst uses the negative cache.
        assert_eq!(engine.catalog_failures, 0);
        engine.voices_fetched_at_ms = 0;
        engine.ensure_voices(&config).await;
        assert_eq!(engine.catalog_failures, 1);
        assert!(engine.catalog_retry_ms <= 30_000);
        engine.voices_fetched_at_ms = 0;
        engine.ensure_voices(&config).await;
        assert_eq!(engine.voices, ["qwen3-gguf:p"]);
        assert_eq!(engine.catalog_failures, 0);
        assert_eq!(engine.catalog_retry_ms, VOICES_TTL_SECS * 1000);
        server.join().unwrap();
    }
}

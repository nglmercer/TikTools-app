//! TikTools global-hotkey backends.
//!
//! Layout:
//!
//! ```text
//! hotkeys/
//!   mod.rs         this file: plugin wiring, shared queues, actions
//!   event.rs       KeyState, key normalization, sequence history
//!   detect.rs      Linux session detection (Wayland vs X11 vs unknown)
//!   status.rs      backend abstraction, capabilities, listener status
//!   shortcuts.rs   portal chord model + `hotkey.bind` parsing
//!   platform.rs    per-OS startup (Windows/macOS native, Linux plan)
//!   rdev_backend.rs rdev listener (Windows/macOS/X11) with failure reports
//!   diagnostics.rs `hotkey.diagnostics` rendering
//!   linux.rs       Linux orchestration (portal + evdev + X11 selection)
//!   linux_portal.rs XDG Desktop Portal GlobalShortcuts backend (ashpd)
//!   linux_evdev.rs  raw `/dev/input/event*` backend for keys/sequences
//! ```
//!
//! The published `hotkey.pressed` contract is unchanged (`key`, `modifiers`,
//! `sequence`); backends only add optional `backend`/`session`/`shortcut_id`
//! fields. Listener health is exposed as `hotkey.status` poll events and the
//! `hotkey.status` / `hotkey.diagnostics` actions so a dead listener can
//! never look alive.

pub mod detect;
pub mod diagnostics;
pub mod event;
pub mod platform;
pub mod rdev_backend;
pub mod shortcuts;
pub mod state;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub mod linux_evdev;
#[cfg(target_os = "linux")]
pub mod linux_portal;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use tiktools_plugin_sdk::prelude::*;
use tiktools_plugin_sdk::POLL_MAX_EVENTS_PER_RESPONSE;

use self::event::{KeyState, MAX_PENDING_EVENTS};
use self::shortcuts::{parse_bind_config, sequences_needed_from_config, shortcuts_from_env, Chord};
use self::state::{capabilities, BackendReport, SharedStatus, SharedStatusHandle};

/// One normalized key press waiting for the host `poll` tick.
#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub key: String,
    pub modifiers: String,
    pub sequence: String,
    pub backend: String,
    pub shortcut_id: Option<String>,
}

pub type PendingQueue = Arc<Mutex<VecDeque<PendingEvent>>>;
pub type KeyStateHandle = Arc<Mutex<KeyState>>;
/// Cumulative producer-side overflow count. Bumped whenever the bounded
/// pending queue drops the oldest event; reported through `hotkey.status`
/// data, poll logs, and diagnostics so overflow is never silent.
pub type DroppedCounter = Arc<Mutex<u64>>;

/// Chords registered with the portal backend plus the raw-input opt-in.
/// Bumped on every change so the portal thread can re-bind without restart.
#[derive(Debug, Default)]
pub struct SharedConfig {
    pub bindings: Vec<Chord>,
    pub sequences_needed: bool,
    pub generation: u64,
}

impl SharedConfig {
    pub fn initial() -> Self {
        // The host sends the persisted Behavior projection as soon as the
        // plugin is started. Keep startup quiet until that first bind call:
        // otherwise a chord-only user could briefly get raw-input prompts
        // before the host has had a chance to disable sequences.
        let bindings = shortcuts_from_env();
        Self {
            bindings,
            sequences_needed: false,
            generation: 0,
        }
    }

    pub fn snapshot(&self) -> (Vec<Chord>, bool, u64) {
        (
            self.bindings.clone(),
            self.sequences_needed,
            self.generation,
        )
    }

    /// True when this exact press is a portal-registered chord. The evdev
    /// backend yields those presses to the portal (which owns them) whenever
    /// the portal backend is running, so one physical press never emits two
    /// `hotkey.pressed` events. Linux-only (evdev/portal live there).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn is_portal_chord(&self, key: &str, modifiers: &str) -> bool {
        self.bindings
            .iter()
            .any(|chord| chord.key == key && chord.modifiers == modifiers)
    }

    pub fn apply_bind_config(&mut self, config: &serde_json::Value) -> Vec<String> {
        let (chords, warnings) = parse_bind_config(config);
        let sequences_needed = sequences_needed_from_config(config);
        let changed = chords != self.bindings || sequences_needed != self.sequences_needed;
        if changed {
            self.bindings = chords;
            self.sequences_needed = sequences_needed;
            self.generation = self.generation.saturating_add(1);
        }
        warnings
    }
}

pub type SharedConfigHandle = Arc<Mutex<SharedConfig>>;

/// Everything a backend thread needs. Cloned per thread; all state is shared.
#[derive(Clone)]
pub struct BackendHandles {
    pub key_state: KeyStateHandle,
    pub pending: PendingQueue,
    pub dropped: DroppedCounter,
    pub shared: SharedStatusHandle,
    /// Only consulted by Linux backends (portal bindings, evdev gating).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub config: SharedConfigHandle,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Records a press in shared state and queues the resulting event.
/// Returns true when an event was queued.
pub fn emit_press(
    handles: &BackendHandles,
    key: &str,
    pressed: bool,
    backend: &str,
    shortcut_id: Option<String>,
    modifiers_override: Option<&str>,
) -> bool {
    let record = handles
        .key_state
        .lock()
        .ok()
        .and_then(|mut state| state.apply(key, pressed, now_ms()));
    if !pressed {
        return false;
    }
    let Some(record) = record else {
        return false;
    };
    let mut queue = match handles.pending.lock() {
        Ok(queue) => queue,
        Err(_) => return false,
    };
    queue.push_back(PendingEvent {
        key: record.key,
        modifiers: modifiers_override.unwrap_or(&record.modifiers).to_owned(),
        sequence: record.sequence,
        backend: backend.to_owned(),
        shortcut_id,
    });
    let mut overflowed = 0u64;
    while queue.len() > MAX_PENDING_EVENTS {
        queue.pop_front();
        overflowed += 1;
    }
    drop(queue);
    if overflowed > 0 {
        if let Ok(mut dropped) = handles.dropped.lock() {
            *dropped = dropped.saturating_add(overflowed);
        }
    }
    true
}

/// Drains at most one protocol batch from the pending queue, in order.
/// Remainder stays queued for the next tick: events are never removed
/// from the producer queue when the consumer would discard them.
pub fn drain_batch(queue: &PendingQueue) -> Vec<PendingEvent> {
    queue
        .lock()
        .map(|mut queue| {
            let take = queue.len().min(POLL_MAX_EVENTS_PER_RESPONSE);
            queue.drain(..take).collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub struct HotkeyPlugin {
    pending: PendingQueue,
    dropped: DroppedCounter,
    last_reported_dropped: u64,
    /// Retained for the Linux late-evdev start; other platforms drive the
    /// listener purely through the spawned thread's cloned handles.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    key_state: KeyStateHandle,
    shared: SharedStatusHandle,
    config: SharedConfigHandle,
    #[cfg(target_os = "linux")]
    session: detect::LinuxSession,
    #[cfg(target_os = "linux")]
    evdev_started: bool,
    #[cfg(target_os = "linux")]
    evdev_allowed_late: bool,
}

impl Default for HotkeyPlugin {
    fn default() -> Self {
        let key_state = Arc::new(Mutex::new(KeyState::default()));
        let pending = Arc::new(Mutex::new(VecDeque::new()));
        let dropped = Arc::new(Mutex::new(0u64));
        let shared = Arc::new(Mutex::new(SharedStatus::default()));
        let config = Arc::new(Mutex::new(SharedConfig::initial()));
        let handles = BackendHandles {
            key_state: Arc::clone(&key_state),
            pending: Arc::clone(&pending),
            dropped: Arc::clone(&dropped),
            shared: Arc::clone(&shared),
            config: Arc::clone(&config),
        };
        #[cfg(target_os = "linux")]
        let startup = platform::start_platform_listeners(&handles);
        #[cfg(not(target_os = "linux"))]
        platform::start_platform_listeners(&handles);
        Self {
            pending,
            dropped,
            last_reported_dropped: 0,
            key_state,
            shared,
            config,
            #[cfg(target_os = "linux")]
            session: startup.session,
            #[cfg(target_os = "linux")]
            evdev_started: startup.evdev_started,
            #[cfg(target_os = "linux")]
            evdev_allowed_late: startup.evdev_allowed_late,
        }
    }
}

impl Plugin for HotkeyPlugin {
    fn action(&mut self, _context: &PluginContext, call: ActionCall) -> PluginResult<ActionResult> {
        match call.action_type().unwrap_or_default() {
            "hotkey.bind" | "hotkey_bind" | "bind" => Ok(self.action_bind(&call)),
            "hotkey.diagnostics" | "hotkey_diagnostics" | "diagnostics" => {
                Ok(self.action_diagnostics())
            }
            "hotkey.status" | "hotkey_status" | "status" => Ok(self.action_status()),
            "" => Ok(ActionResult::summary(
                "hotkey listener has no default action; configure hotkey.pressed events instead",
            )),
            other => Err(PluginError::unsupported(format!(
                "unknown hotkey action {other:?}; use hotkey.bind, hotkey.status, or hotkey.diagnostics"
            ))),
        }
    }

    fn poll(&mut self, _context: &PluginContext) -> PluginResult<PollResult> {
        #[cfg(target_os = "linux")]
        self.maybe_start_evdev_late();
        let mut result = PollResult::default();
        let events = drain_batch(&self.pending);
        for event in events {
            let mut data = serde_json::Map::new();
            data.insert("key".to_owned(), json!(event.key));
            data.insert("modifiers".to_owned(), json!(event.modifiers));
            data.insert("sequence".to_owned(), json!(event.sequence));
            data.insert("backend".to_owned(), json!(event.backend));
            if let Some(shortcut_id) = event.shortcut_id {
                data.insert("shortcut_id".to_owned(), json!(shortcut_id));
            }
            #[cfg(target_os = "linux")]
            data.insert("session".to_owned(), json!(self.session.as_str()));
            result = result.event(PluginEvent::new(
                "hotkey.pressed",
                serde_json::Value::Object(data),
            )?);
        }
        let dropped = self.dropped.lock().map(|count| *count).unwrap_or(0);
        let pending = self.pending.lock().map(|queue| queue.len()).unwrap_or(0);
        // Fresh overflow this tick: one log line plus a status re-emit with
        // the new counters, even when no backend changed state. Counts
        // only — never key contents.
        let fresh_overflow = dropped != self.last_reported_dropped;
        if fresh_overflow {
            result = result.log(format!(
                "hotkey event queue overflowed: {dropped} event(s) dropped in total, {pending} pending"
            ));
            self.last_reported_dropped = dropped;
        }
        let (status_change, snapshot) = self
            .shared
            .lock()
            .map(|mut shared| {
                let change = shared.take_status_change();
                let snapshot = shared.backends.clone();
                (change, snapshot)
            })
            .unwrap_or((None, Vec::new()));
        if status_change.is_some() || (fresh_overflow && !snapshot.is_empty()) {
            let reports = status_change.unwrap_or(snapshot);
            result = result.event(status_event(&self.shared, &reports, dropped, pending)?);
        }
        Ok(result)
    }
}

fn status_event(
    shared: &SharedStatusHandle,
    reports: &[BackendReport],
    dropped_events: u64,
    pending_events: usize,
) -> PluginResult<PluginEvent> {
    let (platform, session) = shared
        .lock()
        .map(|shared| (shared.platform.clone(), shared.session.clone()))
        .unwrap_or_default();
    PluginEvent::new(
        "hotkey.status",
        json!({
            "platform": platform,
            "session": session,
            "droppedEvents": dropped_events,
            "pendingEvents": pending_events,
            "backends": reports.iter().map(|report| {
                let caps = capabilities(report.backend);
                json!({
                    "backend": report.backend.name(),
                    "state": report.state.as_str(),
                    "detail": report.detail,
                    "summary": report.status_line(),
                    "capabilities": {
                        "globalChords": caps.global_chords,
                        "arbitraryKeys": caps.arbitrary_keys,
                        "sequences": caps.sequences,
                        "keyRelease": caps.key_release,
                    },
                })
            }).collect::<Vec<_>>(),
        }),
    )
}

impl HotkeyPlugin {
    fn action_bind(&mut self, call: &ActionCall) -> ActionResult {
        let config = call.config();
        let (warnings, bindings, sequences_needed) = match self.config.lock() {
            Ok(mut shared) => {
                let warnings = shared.apply_bind_config(&serde_json::Value::Object(config.clone()));
                let snapshot = (shared.bindings.clone(), shared.sequences_needed);
                (warnings, snapshot.0, snapshot.1)
            }
            Err(_) => (
                vec!["configuration lock unavailable".to_owned()],
                Vec::new(),
                true,
            ),
        };
        let mut result = ActionResult::summary(format!(
            "portal shortcuts updated: {} bound, sequences {}",
            bindings.len(),
            if sequences_needed {
                "enabled"
            } else {
                "disabled"
            }
        ));
        for warning in warnings {
            result = result.log(warning);
        }
        for chord in &bindings {
            result = result.log(format!(
                "bound {} ({})",
                chord.description(),
                chord.shortcut_id()
            ));
        }
        result
    }

    fn action_status(&mut self) -> ActionResult {
        let summary = self
            .shared
            .lock()
            .map(|shared| {
                let (_, overall) = state::overall_status(&shared.backends);
                overall
            })
            .unwrap_or_else(|_| "Global Hotkeys: status unavailable".to_owned());
        ActionResult::summary(summary)
    }

    fn action_diagnostics(&mut self) -> ActionResult {
        // `from_env` reads Linux session variables; elsewhere it yields an
        // empty snapshot, which the renderer prints as `(unset)`.
        let env = detect::EnvSnapshot::from_env();
        let (report, backend_count) = self
            .shared
            .lock()
            .map(|shared| {
                let (bindings, sequences_needed, _) = self
                    .config
                    .lock()
                    .map(|config| config.snapshot())
                    .unwrap_or_default();
                (
                    diagnostics::render_diagnostics(&shared, &env, &bindings, sequences_needed),
                    shared.backends.len(),
                )
            })
            .unwrap_or_else(|_| ("diagnostics unavailable".to_owned(), 0));
        let mut result = ActionResult::summary(format!(
            "hotkey diagnostics ready ({backend_count} backend entries); see logs"
        ));
        for line in report.lines().map(str::to_owned) {
            result = result.log(line);
        }
        let dropped = self.dropped.lock().map(|count| *count).unwrap_or(0);
        let pending = self.pending.lock().map(|queue| queue.len()).unwrap_or(0);
        result = result.log(format!(
            "Queue:\n  dropped events (overflow): {dropped}\n  pending events: {pending}"
        ));
        result
    }

    #[cfg(target_os = "linux")]
    fn maybe_start_evdev_late(&mut self) {
        if self.evdev_started || !self.evdev_allowed_late {
            return;
        }
        let sequences_needed = self
            .config
            .lock()
            .map(|config| config.sequences_needed)
            .unwrap_or(false);
        if !sequences_needed {
            return;
        }
        let handles = BackendHandles {
            key_state: Arc::clone(&self.key_state),
            pending: Arc::clone(&self.pending),
            dropped: Arc::clone(&self.dropped),
            shared: Arc::clone(&self.shared),
            config: Arc::clone(&self.config),
        };
        linux_evdev::spawn_evdev_listener(handles);
        self.evdev_started = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn startup_waits_for_host_projection_before_enabling_raw_input() {
        assert!(!SharedConfig::initial().sequences_needed);
    }

    fn test_handles() -> BackendHandles {
        BackendHandles {
            key_state: Arc::new(Mutex::new(KeyState::default())),
            pending: Arc::new(Mutex::new(VecDeque::new())),
            dropped: Arc::new(Mutex::new(0)),
            shared: Arc::new(Mutex::new(SharedStatus::default())),
            config: Arc::new(Mutex::new(SharedConfig::initial())),
        }
    }

    fn press(handles: &BackendHandles, key: &str) {
        assert!(emit_press(handles, key, true, "rdev", None, None));
        handles
            .key_state
            .lock()
            .unwrap()
            .apply(key, false, now_ms());
    }

    #[test]
    fn poll_drains_at_most_one_protocol_batch_in_order() {
        let handles = test_handles();
        for index in 0..40 {
            press(&handles, &format!("k{index}"));
        }
        assert_eq!(handles.pending.lock().unwrap().len(), 40);
        // 40 queued → three polls of 16/16/8, order preserved, none lost.
        let first = drain_batch(&handles.pending);
        assert_eq!(first.len(), POLL_MAX_EVENTS_PER_RESPONSE);
        assert_eq!(first[0].key, "k0");
        assert_eq!(first[15].key, "k15");
        assert_eq!(handles.pending.lock().unwrap().len(), 24);
        let second = drain_batch(&handles.pending);
        assert_eq!(second.len(), POLL_MAX_EVENTS_PER_RESPONSE);
        assert_eq!(second[0].key, "k16");
        assert_eq!(handles.pending.lock().unwrap().len(), 8);
        let third = drain_batch(&handles.pending);
        assert_eq!(third.len(), 8);
        assert_eq!(third[7].key, "k39");
        assert!(handles.pending.lock().unwrap().is_empty());
    }

    #[test]
    fn queue_overflow_drops_oldest_and_counts_drops() {
        let handles = test_handles();
        for index in 0..(MAX_PENDING_EVENTS + 6) {
            press(&handles, &format!("k{index}"));
        }
        assert_eq!(handles.pending.lock().unwrap().len(), MAX_PENDING_EVENTS);
        assert_eq!(*handles.dropped.lock().unwrap(), 6);
        // Oldest evicted, newest retained.
        assert_eq!(handles.pending.lock().unwrap()[0].key, "k6");
        // Drains still respect the protocol batch after overflow.
        assert_eq!(
            drain_batch(&handles.pending).len(),
            POLL_MAX_EVENTS_PER_RESPONSE
        );
    }

    #[test]
    fn modifiers_normalize_to_canonical_chord_order() {
        let handles = test_handles();
        let mut state = handles.key_state.lock().unwrap();
        for modifier in ["meta", "alt", "shift", "ctrl"] {
            assert!(state.apply(modifier, true, 1).is_none());
        }
        let record = state.apply("k", true, 2).expect("press records");
        assert_eq!(record.key, "k");
        assert_eq!(record.modifiers, "ctrl+shift+alt+meta");
        assert_eq!(record.sequence, "k");
    }
}

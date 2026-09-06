//! Backend abstraction, capability model, and explicit listener status.
//!
//! The plugin never polls silently after a listener failure: every backend
//! thread reports its lifecycle into [`SharedStatus`], and the host observes
//! it through `hotkey.status` poll events plus the `hotkey.diagnostics` and
//! `hotkey.status` actions. UIs can render lines such as
//! `Global Hotkeys: active via XDG Desktop Portal` directly from that state.

use std::sync::Mutex;

use super::detect::LinuxSession;

/// The concrete listener serving keyboard input on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyBackend {
    /// rdev listener on Windows (`Win32`) or macOS (Accessibility-gated).
    /// Only constructed on non-Linux builds; kept on all targets so backend
    /// reports and capabilities stay comparable across platforms.
    #[allow(dead_code)]
    RdevNative,
    /// rdev/X11 listener (`XOpenDisplay`). Constructed on Linux builds.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    RdevX11,
    /// XDG Desktop Portal GlobalShortcuts (compositor-authorized chords).
    /// Constructed on Linux builds.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Portal,
    /// Raw Linux input devices (`/dev/input/event*`). Constructed on Linux.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Evdev,
}

impl HotkeyBackend {
    pub fn name(self) -> &'static str {
        match self {
            HotkeyBackend::RdevNative => "rdev",
            HotkeyBackend::RdevX11 => "x11",
            HotkeyBackend::Portal => "portal",
            HotkeyBackend::Evdev => "evdev",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            HotkeyBackend::RdevNative => "native listener",
            HotkeyBackend::RdevX11 => "X11",
            HotkeyBackend::Portal => "XDG Desktop Portal",
            HotkeyBackend::Evdev => "raw input (evdev)",
        }
    }
}

/// What a backend can observe. The portal sees only compositor-authorized
/// chords; raw listeners see every key, releases, and sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyBackendCapabilities {
    pub global_chords: bool,
    pub arbitrary_keys: bool,
    pub sequences: bool,
    pub key_release: bool,
}

pub fn capabilities(backend: HotkeyBackend) -> HotkeyBackendCapabilities {
    match backend {
        HotkeyBackend::RdevNative | HotkeyBackend::RdevX11 | HotkeyBackend::Evdev => {
            HotkeyBackendCapabilities {
                global_chords: true,
                arbitrary_keys: true,
                sequences: true,
                key_release: true,
            }
        }
        HotkeyBackend::Portal => HotkeyBackendCapabilities {
            global_chords: true,
            arbitrary_keys: false,
            sequences: false,
            key_release: false,
        },
    }
}

/// Lifecycle of one backend thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendRunState {
    Starting,
    Running,
    PermissionRequired,
    Unsupported,
    Failed,
}

impl BackendRunState {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendRunState::Starting => "starting",
            BackendRunState::Running => "active",
            BackendRunState::PermissionRequired => "permission required",
            BackendRunState::Unsupported => "unsupported",
            BackendRunState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackendReport {
    pub backend: HotkeyBackend,
    pub state: BackendRunState,
    pub detail: String,
}

impl BackendReport {
    pub fn new(backend: HotkeyBackend, state: BackendRunState, detail: impl Into<String>) -> Self {
        Self {
            backend,
            state,
            detail: detail.into(),
        }
    }

    /// One-line UI status, e.g. `Global Hotkeys: active via X11`.
    pub fn status_line(&self) -> String {
        if self.detail.is_empty() {
            format!(
                "Global Hotkeys: {} via {}",
                self.state.as_str(),
                self.backend.display_name()
            )
        } else {
            format!(
                "Global Hotkeys: {} via {} ({})",
                self.state.as_str(),
                self.backend.display_name(),
                self.detail
            )
        }
    }
}

/// Process-wide listener state shared between backend threads and the poll
/// loop. `generation` bumps on every mutation so `poll` can emit exactly one
/// `hotkey.status` event per change without spamming the log.
#[derive(Debug, Default)]
pub struct SharedStatus {
    pub platform: String,
    pub session: String,
    pub desktop: String,
    pub backends: Vec<BackendReport>,
    pub generation: u64,
    pub last_emitted_generation: u64,
    /// extra diagnostic notes (device counts, restore tokens, hints).
    pub notes: Vec<String>,
    /// evdev discovery result, kept separate so diagnostics can name it.
    pub evdev_summary: Option<String>,
}

impl SharedStatus {
    pub fn set_platform(&mut self, platform: &str, session: LinuxSession, desktop: &str) {
        self.platform = platform.to_owned();
        self.session = session.as_str().to_owned();
        self.desktop = desktop.to_owned();
        self.bump();
    }

    pub fn upsert_backend(&mut self, report: BackendReport) {
        if let Some(existing) = self
            .backends
            .iter_mut()
            .find(|entry| entry.backend == report.backend)
        {
            if existing.state != report.state || existing.detail != report.detail {
                *existing = report;
                self.bump();
            }
        } else {
            self.backends.push(report);
            self.bump();
        }
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn set_evdev_summary(&mut self, summary: impl Into<String>) {
        let summary = summary.into();
        if self.evdev_summary.as_deref() != Some(summary.as_str()) {
            self.evdev_summary = Some(summary);
            self.bump();
        }
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn set_note(&mut self, note: impl Into<String>) {
        let note = note.into();
        if !self.notes.contains(&note) {
            self.notes.push(note);
            self.bump();
        }
    }

    /// Drains a pending status change for `poll`. Returns `None` when the
    /// host already saw the latest state.
    pub fn take_status_change(&mut self) -> Option<Vec<BackendReport>> {
        if self.last_emitted_generation == self.generation {
            return None;
        }
        self.last_emitted_generation = self.generation;
        Some(self.backends.clone())
    }

    fn bump(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }
}

/// Which Linux backends to run. Computed from pure inputs so it is unit
/// testable without touching D-Bus or `/dev/input`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxBackendPlan {
    /// Always attempted first on Wayland (cheap probe, authoritative UX).
    pub portal: bool,
    /// Raw input for arbitrary keys/sequences. On Wayland this needs an
    /// explicit device grant; on X11 it is a fallback for a dead display.
    pub evdev: bool,
    /// rdev/X11 listener. Never selected for native Wayland sessions, even
    /// when `DISPLAY` is present (XWayland trap).
    pub x11: bool,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn plan_linux_backend(
    session: LinuxSession,
    sequences_needed: bool,
    x11_display_present: bool,
) -> LinuxBackendPlan {
    match session {
        LinuxSession::Wayland => LinuxBackendPlan {
            portal: true,
            // evdev is the only path to arbitrary keys/sequences on
            // Wayland; when nothing needs it we stay quiet and avoid a
            // permission warning for chord-only users.
            evdev: sequences_needed,
            x11: false,
        },
        LinuxSession::X11 => LinuxBackendPlan {
            portal: false,
            // Without a display socket the X11 listener cannot start; offer
            // evdev as the fallback instead of polling forever.
            evdev: !x11_display_present,
            x11: true,
        },
        LinuxSession::Unknown => LinuxBackendPlan {
            portal: true,
            evdev: sequences_needed,
            x11: x11_display_present,
        },
    }
}

/// Overall health for the diagnostics header: the first running backend wins;
/// otherwise the most actionable non-running state is surfaced.
pub fn overall_status(backends: &[BackendReport]) -> (BackendRunState, String) {
    if let Some(running) = backends
        .iter()
        .find(|report| report.state == BackendRunState::Running)
    {
        return (BackendRunState::Running, running.status_line());
    }
    for state in [
        BackendRunState::PermissionRequired,
        BackendRunState::Failed,
        BackendRunState::Unsupported,
        BackendRunState::Starting,
    ] {
        if let Some(report) = backends.iter().find(|report| report.state == state) {
            return (state, report.status_line());
        }
    }
    (
        BackendRunState::Starting,
        "Global Hotkeys: starting".to_owned(),
    )
}

pub type SharedStatusHandle = std::sync::Arc<Mutex<SharedStatus>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::detect::LinuxSession;

    #[test]
    fn wayland_never_selects_x11() {
        // Even with an XWayland DISPLAY present, X11 must not be selected.
        let plan = plan_linux_backend(LinuxSession::Wayland, true, true);
        assert!(plan.portal);
        assert!(plan.evdev);
        assert!(!plan.x11);

        let chords_only = plan_linux_backend(LinuxSession::Wayland, false, true);
        assert!(chords_only.portal);
        assert!(!chords_only.evdev);
        assert!(!chords_only.x11);
    }

    #[test]
    fn x11_uses_rdev_with_evdev_only_as_display_fallback() {
        let healthy = plan_linux_backend(LinuxSession::X11, true, true);
        assert!(healthy.x11);
        assert!(!healthy.evdev);
        assert!(!healthy.portal);

        let no_display = plan_linux_backend(LinuxSession::X11, false, false);
        assert!(no_display.x11);
        assert!(no_display.evdev);
    }

    #[test]
    fn unknown_session_probes_everything_available() {
        let plan = plan_linux_backend(LinuxSession::Unknown, true, true);
        assert!(plan.portal && plan.evdev && plan.x11);
        let headless = plan_linux_backend(LinuxSession::Unknown, false, false);
        assert!(headless.portal && !headless.evdev && !headless.x11);
    }

    #[test]
    fn portal_lacks_arbitrary_key_capabilities() {
        let portal = capabilities(HotkeyBackend::Portal);
        assert!(portal.global_chords);
        assert!(!portal.arbitrary_keys);
        assert!(!portal.sequences);
        assert!(!portal.key_release);

        let x11 = capabilities(HotkeyBackend::RdevX11);
        assert!(x11.global_chords && x11.arbitrary_keys && x11.sequences && x11.key_release);
    }

    #[test]
    fn status_changes_are_emitted_once() {
        let mut status = SharedStatus::default();
        assert!(status.take_status_change().is_none());
        status.upsert_backend(BackendReport::new(
            HotkeyBackend::Portal,
            BackendRunState::Running,
            "3 shortcuts bound",
        ));
        assert!(status.take_status_change().is_some());
        assert!(status.take_status_change().is_none());
        // Re-reporting identical state does not re-emit.
        status.upsert_backend(BackendReport::new(
            HotkeyBackend::Portal,
            BackendRunState::Running,
            "3 shortcuts bound",
        ));
        assert!(status.take_status_change().is_none());
    }

    #[test]
    fn failures_are_visible_instead_of_silent() {
        let reports = vec![
            BackendReport::new(
                HotkeyBackend::Portal,
                BackendRunState::Unsupported,
                "GlobalShortcuts unavailable",
            ),
            BackendReport::new(
                HotkeyBackend::Evdev,
                BackendRunState::PermissionRequired,
                "no readable /dev/input/event* devices",
            ),
        ];
        let (state, line) = overall_status(&reports);
        assert_eq!(state, BackendRunState::PermissionRequired);
        assert!(line.contains("permission required"), "{line}");
    }
}

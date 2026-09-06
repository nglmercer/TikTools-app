//! rdev listener used on Windows, macOS, and Linux X11.
//!
//! The failure mode this fixes: `rdev::listen()` returns `Err` (missing
//! `DISPLAY`, no Accessibility grant, dead X server) and the old plugin kept
//! polling forever with no events. The error is now classified into
//! [`BackendRunState`] and reported through [`SharedStatus`].

use rdev::{listen, EventType};

use super::event::key_name;
use super::state::{BackendReport, BackendRunState, HotkeyBackend, SharedStatusHandle};
use super::{emit_press, BackendHandles};

/// Starts the rdev listener on the current thread. Returns only when the
/// listener exits; the terminal status is always recorded first.
pub fn run_rdev_listener(backend: HotkeyBackend, handles: &BackendHandles) {
    // A reconnect must never inherit a modifier held before the outage.
    if let Ok(mut guard) = handles.key_state.lock() {
        guard.reset();
    }
    set_running(&handles.shared, backend);
    let callback_handles = handles.clone();
    let backend_name = backend.name();
    let failed = listen(move |event| {
        let (key, pressed) = match event.event_type {
            EventType::KeyPress(key) => (key, true),
            EventType::KeyRelease(key) => (key, false),
            _ => return,
        };
        let name = key_name(&key);
        emit_press(&callback_handles, &name, pressed, backend_name, None, None);
    });
    if let Err(error) = failed {
        let report = classify_rdev_error(backend, &error);
        handles
            .shared
            .lock()
            .map(|mut guard| guard.upsert_backend(report))
            .ok();
    } else {
        // `listen` returning Ok means the hook was torn down from outside.
        handles
            .shared
            .lock()
            .map(|mut guard| {
                guard.upsert_backend(BackendReport::new(
                    backend,
                    BackendRunState::Failed,
                    "listener exited unexpectedly; restart TikTools to reattach",
                ));
            })
            .ok();
    }
}

fn set_running(shared: &SharedStatusHandle, backend: HotkeyBackend) {
    shared
        .lock()
        .map(|mut guard| {
            guard.upsert_backend(BackendReport::new(
                backend,
                BackendRunState::Running,
                "listener attached",
            ));
        })
        .ok();
}

/// Classifies an rdev startup failure so the UI can show an actionable line
/// instead of `hotkey listener stopped: ...` on stderr.
pub fn classify_rdev_error(backend: HotkeyBackend, error: &rdev::ListenError) -> BackendReport {
    classify_rdev_error_text(backend, &format!("{error:?}"))
}

fn classify_rdev_error_text(backend: HotkeyBackend, text: &str) -> BackendReport {
    let lowered = text.to_ascii_lowercase();
    // The X11 backend surfaces a missing display (XOpenDisplay(NULL)
    // failed because DISPLAY/XAUTHORITY never reached the plugin).
    if lowered.contains("display") || lowered.contains("xopendisplay") {
        return BackendReport::new(
            backend,
            BackendRunState::Failed,
            format!("X11 display unavailable ({text}); check DISPLAY/XAUTHORITY forwarding"),
        );
    }
    if lowered.contains("permission")
        || lowered.contains("accessibility")
        || lowered.contains("granted")
    {
        return BackendReport::new(
            backend,
            BackendRunState::PermissionRequired,
            format!("OS input permission required ({text})"),
        );
    }
    BackendReport::new(backend, BackendRunState::Failed, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_display_is_a_failure_not_silence() {
        // The rdev error type is non-exhaustive, so the classifier works
        // off its debug rendering; feed the documented variant shape.
        let report = classify_rdev_error_text(HotkeyBackend::RdevX11, "MissingDisplayError");
        assert_eq!(report.state, BackendRunState::Failed);
        assert!(report.detail.contains("DISPLAY"), "{}", report.detail);
        assert!(
            report.status_line().contains("via X11"),
            "{}",
            report.status_line()
        );
    }

    #[test]
    fn permission_denied_asks_for_a_grant() {
        let report = classify_rdev_error_text(
            HotkeyBackend::RdevNative,
            "EventTapError (accessibility permission required)",
        );
        assert_eq!(report.state, BackendRunState::PermissionRequired);
    }
}

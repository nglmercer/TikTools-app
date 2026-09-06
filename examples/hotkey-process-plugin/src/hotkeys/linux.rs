//! Linux orchestration: session detection, backend selection, startup.
//!
//! ```text
//! Wayland: portal (chords) + evdev (keys/sequences, permission-gated)
//! X11:     rdev/X11 (+ evdev fallback when DISPLAY is missing)
//! Unknown: probe portal, evdev when sequences are wanted, X11 when DISPLAY exists
//! ```
//!
//! XWayland (`DISPLAY` set inside a Wayland session) never selects X11.

use super::detect::{desktop_label, detect_session, is_xwayland_display, EnvSnapshot};
use super::platform::{spawn_rdev, PlatformStartup};
use super::state::{plan_linux_backend, BackendReport, BackendRunState, HotkeyBackend};
use super::BackendHandles;

pub fn start_linux_listeners(handles: &BackendHandles) -> PlatformStartup {
    let env = EnvSnapshot::from_env();
    let session = detect_session(&env);
    let desktop = desktop_label(&env);
    let display_present = env
        .display
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let sequences_needed = handles
        .config
        .lock()
        .map(|config| config.sequences_needed)
        .unwrap_or(true);

    handles
        .shared
        .lock()
        .map(|mut shared| {
            shared.set_platform("linux", session, &desktop);
            if is_xwayland_display(&env) {
                shared.set_note(
                    "DISPLAY comes from XWayland; the X11 backend is skipped in favor of the portal",
                );
            }
        })
        .ok();

    let plan = plan_linux_backend(session, sequences_needed, display_present);

    if plan.x11 {
        mark_starting(handles, HotkeyBackend::RdevX11, "attaching X11 listener");
        spawn_rdev(HotkeyBackend::RdevX11, handles);
    }
    if plan.portal {
        super::linux_portal::spawn_portal_listener(handles.clone());
    }
    let mut evdev_started = false;
    if plan.evdev {
        super::linux_evdev::spawn_evdev_listener(handles.clone());
        evdev_started = true;
    }

    PlatformStartup {
        session,
        evdev_started,
        // On native X11 the rdev listener already covers sequences, so a
        // late opt-in needs no new backend. Everywhere else the poll loop
        // may start evdev once sequences are requested.
        evdev_allowed_late: session != super::detect::LinuxSession::X11,
    }
}

fn mark_starting(handles: &BackendHandles, backend: HotkeyBackend, detail: &str) {
    handles
        .shared
        .lock()
        .map(|mut shared| {
            shared.upsert_backend(BackendReport::new(
                backend,
                BackendRunState::Starting,
                detail,
            ));
        })
        .ok();
}

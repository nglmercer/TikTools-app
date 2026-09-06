//! Per-OS listener startup.
//!
//! Windows and macOS keep the existing rdev listener. Linux follows the
//! runtime plan from [`crate::hotkeys::state::plan_linux_backend`]: portal
//! first on Wayland, rdev/X11 on X11, evdev for sequences/fallback — never
//! X11 inside a Wayland session just because `DISPLAY` exists.

#[cfg(not(target_os = "linux"))]
use super::detect::LinuxSession;
#[cfg(target_os = "linux")]
use super::state::HotkeyBackend;
#[cfg(not(target_os = "linux"))]
use super::state::{BackendReport, BackendRunState, HotkeyBackend};
use super::BackendHandles;

/// Outcome of listener startup, consumed by the poll loop for late starts.
pub struct PlatformStartup {
    #[cfg(target_os = "linux")]
    pub session: super::detect::LinuxSession,
    #[cfg(target_os = "linux")]
    pub evdev_started: bool,
    #[cfg(target_os = "linux")]
    pub evdev_allowed_late: bool,
}

#[cfg(not(target_os = "linux"))]
pub fn start_platform_listeners(handles: &BackendHandles) -> PlatformStartup {
    handles
        .shared
        .lock()
        .map(|mut shared| {
            shared.set_platform(std::env::consts::OS, LinuxSession::Unknown, "n/a");
            shared.upsert_backend(BackendReport::new(
                HotkeyBackend::RdevNative,
                BackendRunState::Starting,
                "attaching native listener",
            ));
        })
        .ok();
    spawn_rdev(HotkeyBackend::RdevNative, handles);
    PlatformStartup {}
}

#[cfg(target_os = "linux")]
pub fn start_platform_listeners(handles: &BackendHandles) -> PlatformStartup {
    super::linux::start_linux_listeners(handles)
}

pub fn spawn_rdev(backend: HotkeyBackend, handles: &BackendHandles) {
    let thread_handles = handles.clone();
    std::thread::Builder::new()
        .name(format!("tiktools-hotkey-{}", backend.name()))
        .spawn(move || {
            super::rdev_backend::run_rdev_listener(backend, &thread_handles);
        })
        .ok();
}

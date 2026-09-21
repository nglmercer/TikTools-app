//! Small platform seam for UI-thread initialization and WebView event-loop
//! maintenance. Platform conditionals stay here instead of spreading through
//! the core or IPC layers.

use winit::{
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopBuilder},
    window::Window,
};
use wry::{WebView, WebViewBuilder};

#[cfg(target_os = "linux")]
use std::time::Duration;
use std::time::Instant;

/// How often the Linux event loop wakes while idle. GTK/libappindicator has
/// no source attached to Winit's loop, so both the steady-state wait and the
/// WebView startup wait reuse this interval to keep GTK/WebKit progressing.
#[cfg(target_os = "linux")]
const GTK_PUMP_INTERVAL: Duration = Duration::from_millis(50);

pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        // WebKitGTK/GLX can emit GLXBadWindow while Winit is processing a
        // focus transition. Winit otherwise reports that queued Xlib error at
        // the next IME operation and panics. This is the supported Winit hook
        // for Xlib users such as WebKitGTK and matches Wry's Linux example.
        winit::platform::x11::register_xlib_error_hook(Box::new(|_display, error| {
            let error = error as *mut x11_dl::xlib::XErrorEvent;
            unsafe { (*error).error_code == 170 }
        }));

        // Wry's child-window backend is X11-only. When a desktop exposes both
        // Wayland and X11/XWayland, select the same backend for GTK and Winit
        // so the raw handles remain compatible. Native Wayland needs the
        // GTK-container path kept behind this platform seam.
        if std::env::var_os("DISPLAY").is_some()
            && std::env::var_os("WAYLAND_DISPLAY").is_some()
            && std::env::var_os("GDK_BACKEND").is_none()
        {
            std::env::set_var("GDK_BACKEND", "x11");
        }
        gtk::init()?;
        tracing::debug!(
            display = ?std::env::var_os("DISPLAY"),
            wayland_display = ?std::env::var_os("WAYLAND_DISPLAY"),
            gdk_backend = ?std::env::var_os("GDK_BACKEND"),
            session_type = ?std::env::var_os("XDG_SESSION_TYPE"),
            "Linux desktop display environment"
        );
        if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_none() {
            tracing::warn!(
                "Winit/Wry child WebViews require an X11 display; use DISPLAY/XWayland or the future Tao/GTK seam for native Wayland"
            );
        }
    }
    Ok(())
}

pub fn configure_event_loop<T>(_builder: &mut EventLoopBuilder<T>) {
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("GDK_BACKEND").as_deref() == Some(std::ffi::OsStr::new("x11"))
    {
        use winit::platform::x11::EventLoopBuilderExtX11;

        _builder.with_x11();
        tracing::debug!("using X11/XWayland for the Winit/Wry child WebView");
    }
}

pub fn pump() {
    #[cfg(target_os = "linux")]
    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }
}

pub fn prepare_for_wait(event_loop: &ActiveEventLoop) {
    #[cfg(target_os = "linux")]
    {
        // GTK/libappindicator has no source attached to Winit's event loop.
        // Wake periodically so menu callbacks are dispatched even while no
        // window/X11 event is pending. This keeps the UI responsive without
        // switching the whole desktop loop to a busy Poll cycle.
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + GTK_PUMP_INTERVAL));
    }
    #[cfg(not(target_os = "linux"))]
    event_loop.set_control_flow(ControlFlow::Wait);
}

/// Startup variant of [`prepare_for_wait`]. While the WebView is loading, the
/// loop must still wake periodically on Linux so GTK/WebKit keeps progressing;
/// sleeping until the whole startup deadline would starve the page load and
/// the `frontend-ready` IPC behind it. Other platforms wait for the deadline
/// directly.
pub fn prepare_for_startup_wait(event_loop: &ActiveEventLoop, deadline: Instant) {
    #[cfg(target_os = "linux")]
    {
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_startup_wake(
            Instant::now(),
            deadline,
            GTK_PUMP_INTERVAL,
        )));
    }
    #[cfg(not(target_os = "linux"))]
    {
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
    }
}

/// Pure deadline helper behind [`prepare_for_startup_wait`]: wake at the next
/// pump interval, but never past the startup deadline.
#[allow(dead_code)]
fn next_startup_wake(now: Instant, deadline: Instant, interval: std::time::Duration) -> Instant {
    std::cmp::min(deadline, now + interval)
}

pub fn build_webview(builder: WebViewBuilder<'_>, window: &Window) -> wry::Result<WebView> {
    // `build_as_child` is supported by Wry on Windows, macOS, and Linux/X11.
    // Keeping this call in one platform seam leaves a later Linux Tao/GTK
    // implementation isolated from the core and IPC crates.
    builder.build_as_child(window)
}

/// Round-trips the GDK display after a WebView drop on Linux. Call it on
/// the UI thread, synchronously, between the WebView drop and the return
/// to the event loop (or the Winit window drop).
///
/// Wry builds child WebViews with their own X11 container window, which
/// `WebView::drop` destroys on GDK's connection, while Winit destroys the
/// parent window in `Window::drop` on its own connection. The parent
/// destroy flushes first (still inside Winit's event poll) and recursively
/// kills the not-yet-destroyed container; the container destroy then fails
/// with BadWindow on GDK's display, and Winit's process-global Xlib error
/// hook files that foreign error into Winit's own slot with no display
/// check. The next `check_errors` (IME focus when focus reverts to the
/// surviving window) panics with "Failed to focus input context".
///
/// The round-trip forces the container destroy to execute while Winit's
/// parent destroy is still sitting in its unflushed buffer, so the parent
/// destroy later finds an already-dead child and stays silent. Safe
/// without GTK (headless tests): the gtk-rs bindings panic when touched
/// before `gtk::init`, so uninitialized processes skip the round-trip.
pub fn sync_gtk_display() {
    #[cfg(target_os = "linux")]
    if gtk::is_initialized() {
        if let Some(display) = gtk::gdk::Display::default() {
            display.sync();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_wake_prefers_the_pump_interval_before_the_deadline() {
        let now = Instant::now();
        let deadline = now + std::time::Duration::from_secs(10);
        let wake = next_startup_wake(now, deadline, std::time::Duration::from_millis(50));
        assert!(wake <= now + std::time::Duration::from_millis(50));
        assert!(wake <= deadline);
    }

    #[test]
    fn startup_wake_never_passes_an_imminent_deadline() {
        let now = Instant::now();
        let deadline = now + std::time::Duration::from_millis(20);
        let wake = next_startup_wake(now, deadline, std::time::Duration::from_millis(50));
        assert_eq!(wake, deadline);
    }

    #[test]
    fn gtk_display_sync_is_safe_without_gtk() {
        // The teardown paths call this unconditionally; headless test runs
        // never initialize GTK, so this must be a silent no-op there.
        sync_gtk_display();
    }
}

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
/// The webview is a GTK child window: every keystroke, click and WebKit frame
/// callback only advances inside these iterations, so the interval is also
/// the worst-case input latency and the effective animation cadence. 16ms
/// matches a 60Hz display; the idle cost is one non-blocking iteration per
/// wake, while backlog batches chain immediate wakes regardless (see
/// `flush_webview_batch`), so message throughput is not tied to this value.
#[cfg(target_os = "linux")]
const GTK_PUMP_INTERVAL: Duration = Duration::from_millis(16);

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
        configure_webkit_renderer();
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

/// WebKitGTK renderer policy. WebKitGTK 2.54 composes through DMA-BUF
/// buffers shared with the GPU by default; on drivers whose buffer-sharing
/// path is broken (reported across NVIDIA and uncommon AMD parts) the frames
/// never present, so the window only repaints when a resize or unminimize
/// damages it wholesale. Forcing the shared-memory transport keeps the
/// composited renderer — measured within a few ms of the hardware transport
/// upstream — while dropping the driver-dependent buffer sharing. Never
/// override an explicit operator choice: any of the renderer variables being
/// set means the user already picked a policy. Must run before `gtk::init`
/// so the WebKitWebProcess children inherit it.
#[cfg(target_os = "linux")]
fn configure_webkit_renderer() {
    const RENDERER_VARS: [&str; 4] = [
        "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
        "WEBKIT_DISABLE_DMABUF_RENDERER",
        "WEBKIT_DISABLE_COMPOSITING_MODE",
        "WEBKIT_FORCE_COMPOSITING_MODE",
    ];
    if RENDERER_VARS
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    {
        return;
    }
    std::env::set_var("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1");
    // Empirical on exotic/fragile GPU drivers (e.g. the AMD BC-250 compute
    // card): the GL pipeline composes frames that never fully present —
    // windows repaint only after a resize. Skia CPU rendering bypasses the
    // GPU entirely and measured correct on hardware where the GL path
    // visibly breaks; typical web UI cost is modest either way. Opt out
    // with TIKTOOLS_WEBKIT_GPU=1 on machines with healthy drivers.
    if std::env::var_os("TIKTOOLS_WEBKIT_GPU").is_none() {
        std::env::set_var("WEBKIT_SKIA_ENABLE_CPU_RENDERING", "1");
    }
    tracing::debug!(
        "forcing WebKitGTK shared-memory transport + skia CPU rendering; set TIKTOOLS_WEBKIT_GPU=1 or any WEBKIT_* variable to override"
    );
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

/// Host action for an OS window focus transition. WebKitGTK child windows do
/// not always track window focus on X11: without an explicit re-focus, a
/// field can keep its `:focus` visuals while keyboard focus (caret, typing)
/// never returns to the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFocusAction {
    /// Move keyboard focus into the page.
    FocusWebview,
    /// Keep DOM focus untouched (browser parity); the transition is only logged.
    Preserve,
}

/// Pure policy behind the `WindowEvent::Focused` arms: focus gains move
/// keyboard focus into the WebView, focus losses preserve DOM focus so the
/// user's place (caret, selection, focused field) survives window switches.
pub fn window_focus_action(gained: bool) -> WindowFocusAction {
    if gained {
        WindowFocusAction::FocusWebview
    } else {
        WindowFocusAction::Preserve
    }
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

/// Stop the Wry 0.57 GTK IPC signal before releasing its WebView owner.
/// Wry's script-message-received closure captures a strong WebView; the
/// WebView owns the UserContentManager that owns that closure. GTK widget
/// destroy alone did not retire renderer children in the 100-cycle reproducer.
/// Unregistering a JS handler does not disconnect the GObject signal.
/// Disconnect only that signal, on this private manager, at terminal teardown.
/// This never destroys a native window or changes the child/sync/parent order.
pub(crate) fn prepare_webview_teardown(webview: &WebView) -> u64 {
    #[cfg(target_os = "linux")]
    {
        use gtk::glib::{gobject_ffi, prelude::*, translate::IntoGlib};
        use webkit2gtk::{UserContentManagerExt, WebViewExt};
        use wry::WebViewExtUnix;
        #[cfg(test)]
        if std::env::var_os("TIKTOOLS_NATIVE_SMOKE_RETAIN_IPC").is_some() {
            return 0;
        }
        let native = webview.webview();
        let Some(manager) = native.user_content_manager() else {
            return 0;
        };
        manager.unregister_script_message_handler("ipc");
        let mut disconnected = 0;
        // SAFETY: GTK/Wry owners and manager remain live on their UI thread.
        // Lookup uses the manager's runtime type. Each ID is found on this
        // instance immediately before disconnect, which also releases its
        // closure. No borrowed callback is executing during deferred teardown.
        // Find+disconnect supports GLib versions before 2.78 (unlike matching
        // disconnect with ID alone). The loop is finite over existing handlers.
        unsafe {
            let signal = gobject_ffi::g_signal_lookup(
                c"script-message-received".as_ptr(),
                manager.type_().into_glib(),
            );
            if signal == 0 {
                return 0;
            }
            loop {
                let id = gobject_ffi::g_signal_handler_find(
                    manager.as_ptr().cast(),
                    gobject_ffi::G_SIGNAL_MATCH_ID,
                    signal,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if id == 0 {
                    break;
                }
                gobject_ffi::g_signal_handler_disconnect(manager.as_ptr().cast(), id);
                disconnected += 1;
            }
        }
        DISCONNECTED_IPC_HANDLERS.fetch_add(disconnected, std::sync::atomic::Ordering::Relaxed);
        disconnected
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = webview;
        0
    }
}

static DISCONNECTED_IPC_HANDLERS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
static RETAINED_EXTERNAL_HANDLES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

pub(crate) fn record_external_destroy() {
    RETAINED_EXTERNAL_HANDLES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn native_lifecycle_diagnostics() -> serde_json::Value {
    serde_json::json!({
        "disconnectedIpcHandlers": DISCONNECTED_IPC_HANDLERS.load(std::sync::atomic::Ordering::Relaxed),
        "nativeBrokerAdmissions": crate::event::native_admissions(),
        "retainedExternalHandles": RETAINED_EXTERNAL_HANDLES.load(std::sync::atomic::Ordering::Relaxed),
    })
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
    fn window_focus_gain_moves_keyboard_focus_into_the_webview() {
        assert_eq!(window_focus_action(true), WindowFocusAction::FocusWebview);
    }

    #[test]
    fn window_focus_loss_preserves_dom_focus() {
        assert_eq!(window_focus_action(false), WindowFocusAction::Preserve);
    }

    #[test]
    fn gtk_display_sync_is_safe_without_gtk() {
        // The teardown paths call this unconditionally; headless test runs
        // never initialize GTK, so this must be a silent no-op there.
        sync_gtk_display();
    }
}

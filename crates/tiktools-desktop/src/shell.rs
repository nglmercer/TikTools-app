//! Platform window shell. On Linux the shell window is a native GTK window
//! and the webview embeds as a regular GTK widget inside it, so input,
//! focus, compositing and resizing follow GTK's own paths on both X11 and
//! Wayland — no XWayland child-window translation. Other platforms keep
//! the Winit window with the native child embedding.
//!
//! The Winit event loop still drives application logic on every platform
//! (command delivery, batching ticks); only window ownership differs.

#[cfg(target_os = "linux")]
use gtk::prelude::*;

pub enum ShellWindow {
    #[cfg(target_os = "linux")]
    Gtk {
        #[allow(dead_code)]
        window: gtk::Window,
        content: gtk::Box,
    },
    #[cfg(not(target_os = "linux"))]
    Winit(winit::window::Window),
}

impl ShellWindow {
    pub fn set_visible(&self, visible: bool) {
        match self {
            #[cfg(target_os = "linux")]
            ShellWindow::Gtk { window, .. } => window.set_visible(visible),
            #[cfg(not(target_os = "linux"))]
            ShellWindow::Winit(window) => window.set_visible(visible),
        }
    }

    pub fn set_minimized(&self, minimized: bool) {
        match self {
            #[cfg(target_os = "linux")]
            ShellWindow::Gtk { window, .. } => {
                if minimized {
                    window.iconify();
                } else {
                    window.deiconify();
                }
            }
            #[cfg(not(target_os = "linux"))]
            ShellWindow::Winit(window) => window.set_minimized(minimized),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn gtk_content(&self) -> Option<&gtk::Box> {
        match self {
            ShellWindow::Gtk { content, .. } => Some(content),
        }
    }

    pub fn focus(&self) {
        match self {
            #[cfg(target_os = "linux")]
            ShellWindow::Gtk { window, .. } => window.present(),
            #[cfg(not(target_os = "linux"))]
            ShellWindow::Winit(window) => window.focus_window(),
        }
    }
}

/// Builds the Linux shell window: a GTK window whose single container fills
/// the frame. The webview is added to the box by wry's `build_gtk`; making
/// it expand lets GTK layout resize it with the window, so no manual bounds
/// management exists on this platform.
#[cfg(target_os = "linux")]
pub fn create_gtk_shell_window(title: &str, width: i32, height: i32) -> (gtk::Window, gtk::Box) {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(title);
    window.set_default_size(width, height);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.add(&content);
    // Children start visible inside the hidden window; presenting later
    // shows everything in one compositor commit.
    content.show();
    (window, content)
}

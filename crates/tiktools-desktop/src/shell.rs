//! Platform window shell. On Linux the shell window is a native GTK window
//! and the webview embeds as a regular GTK widget inside it, so input,
//! focus, compositing and resizing follow GTK's own paths on both X11 and
//! Wayland — no XWayland child-window translation. Windows keeps the Winit
//! window with the native child embedding and receives the same frontend-owned
//! menu model through a Win32 menu bar.
//!
//! The Winit event loop still drives application logic on every platform
//! (command delivery, batching ticks); only window ownership differs.

#[cfg(target_os = "linux")]
use gtk::prelude::*;
#[cfg(target_os = "linux")]
use tiktools_core::HostEmitter as _;
#[cfg(target_os = "windows")]
use winit::raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

pub(crate) const TOGGLE_SIDEBAR_MENU_ID: &str = "toggle-sidebar";

pub enum ShellWindow {
    #[cfg(target_os = "linux")]
    Gtk {
        #[allow(dead_code)]
        window: gtk::Window,
        content: gtk::Box,
        menubar: gtk::MenuBar,
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

    #[cfg(target_os = "linux")]
    pub fn gtk_menubar(&self) -> Option<&gtk::MenuBar> {
        match self {
            ShellWindow::Gtk { menubar, .. } => Some(menubar),
        }
    }

    /// The GTK window itself, for transient parenting of native dialogs:
    /// unparented portal dialogs surface behind the shell on focus-stealing-
    /// preventing compositors, which reads exactly like a dead button.
    #[cfg(target_os = "linux")]
    pub fn gtk_window(&self) -> Option<&gtk::Window> {
        match self {
            ShellWindow::Gtk { window, .. } => Some(window),
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

    /// Non-Linux platforms keep the plain Winit window underneath the shell
    /// abstraction; dialog parenting, identity checks and the child-webview
    /// builder all operate on it directly.
    #[cfg(not(target_os = "linux"))]
    pub fn winit_window(&self) -> &winit::window::Window {
        match self {
            ShellWindow::Winit(window) => window,
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn id(&self) -> winit::window::WindowId {
        self.winit_window().id()
    }
}

/// Builds the Linux shell window: a GTK window whose single container fills
/// the frame below a native header bar. The webview is added to the box by
/// wry's `build_gtk`; making it expand lets GTK layout resize it with the
/// window, so no manual bounds management exists on this platform.
///
/// The header bar carries the native menu bar (populated later from the
/// frontend-owned model) and the sidebar toggle; both report activations as
/// `shell-menu-action` host messages — the host never interprets menu
/// semantics, it renders and forwards.
#[cfg(target_os = "linux")]
pub fn create_gtk_shell_window(
    title: &str,
    width: i32,
    height: i32,
    emitter: std::sync::Arc<crate::event::EventLoopEmitter>,
) -> (gtk::Window, gtk::Box, gtk::MenuBar) {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_default_size(width, height);

    let header = gtk::HeaderBar::new();
    header.set_title(Some(title));
    header.set_show_close_button(true);

    let sidebar_button = gtk::Button::new();
    sidebar_button.set_image(Some(&gtk::Image::from_icon_name(
        Some("open-menu-symbolic"),
        gtk::IconSize::Menu,
    )));
    sidebar_button.set_tooltip_text(Some("Toggle sidebar"));
    {
        let emitter = emitter.clone();
        sidebar_button.connect_clicked(move |_| {
            emitter.emit(tiktools_core::ipc::messages::HostMessage::ShellMenuAction {
                id: TOGGLE_SIDEBAR_MENU_ID.to_owned(),
            });
        });
    }
    header.pack_start(&sidebar_button);

    let menubar = gtk::MenuBar::new();
    header.pack_start(&menubar);

    window.set_titlebar(Some(&header));
    header.show_all();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.add(&content);
    // Children start visible inside the hidden window; presenting later
    // shows everything in one compositor commit.
    content.show();
    (window, content, menubar)
}

/// Rebuilds the native menu bar from the frontend-owned model. The wire
/// shape mirrors the frontend's `MenuBarMenu`/`MenuEntry` types minus
/// icons: `[{ id, label, items: [{ type?, id, label?, disabled?, checked?,
/// children? }] }]`. Any malformed entry is skipped — a broken menu model
/// must never take down window chrome.
#[cfg(target_os = "linux")]
pub fn rebuild_native_menu(
    menubar: &gtk::MenuBar,
    model: &serde_json::Value,
    emitter: &std::sync::Arc<crate::event::EventLoopEmitter>,
) {
    for child in menubar.children() {
        menubar.remove(&child);
    }
    let Some(menus) = model.as_array() else {
        return;
    };
    for menu in menus {
        let Some(label) = menu.get("label").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let top = gtk::MenuItem::with_label(label);
        let submenu = gtk::Menu::new();
        if let Some(items) = menu.get("items") {
            append_native_items(&submenu, items, emitter);
        }
        top.set_submenu(Some(&submenu));
        menubar.append(&top);
    }
    menubar.show_all();
}

#[cfg(target_os = "linux")]
fn append_native_items(
    menu: &gtk::Menu,
    items: &serde_json::Value,
    emitter: &std::sync::Arc<crate::event::EventLoopEmitter>,
) {
    let Some(entries) = items.as_array() else {
        return;
    };
    for entry in entries {
        if entry.get("type").and_then(serde_json::Value::as_str) == Some("separator") {
            menu.append(&gtk::SeparatorMenuItem::new());
            continue;
        }
        let (Some(id), Some(label)) = (
            entry.get("id").and_then(serde_json::Value::as_str),
            entry.get("label").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        let checked = entry
            .get("checked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let item: gtk::MenuItem = if checked || entry.get("checked").is_some() {
            let check = gtk::CheckMenuItem::with_label(label);
            check.set_active(checked);
            check.upcast()
        } else {
            gtk::MenuItem::with_label(label)
        };
        if entry
            .get("disabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            item.set_sensitive(false);
        }
        let children = entry.get("children").and_then(serde_json::Value::as_array);
        if children.is_some_and(|list| !list.is_empty()) {
            let submenu = gtk::Menu::new();
            append_native_items(&submenu, entry.get("children").unwrap(), emitter);
            item.set_submenu(Some(&submenu));
        } else {
            let emitter = emitter.clone();
            let id = id.to_owned();
            item.connect_activate(move |_| {
                emitter.emit(tiktools_core::ipc::messages::HostMessage::ShellMenuAction {
                    id: id.clone(),
                });
            });
        }
        menu.append(&item);
    }
}

/// Rebuilds the Windows menu bar from the same frontend-owned wire model as
/// the GTK shell. The returned menu must be stored by the caller: dropping it
/// detaches it from the window. Malformed entries are skipped, matching the
/// Linux policy that window chrome must survive a broken menu model.
#[cfg(target_os = "windows")]
pub fn rebuild_native_menu(
    window: &winit::window::Window,
    model: &serde_json::Value,
) -> Result<muda::Menu, String> {
    let menu = muda::Menu::new();
    // Match the GTK header's sidebar button. A Win32 menu bar cannot carry a
    // standalone button, so the same chrome control is a compact command at
    // the front of the bar and reports the identical opaque frontend action.
    let sidebar = muda::MenuItem::with_id(TOGGLE_SIDEBAR_MENU_ID, "☰", true, None);
    menu.append(&sidebar)
        .map_err(|error| format!("could not append Windows sidebar menu item: {error}"))?;
    if let Some(menus) = model.as_array() {
        for menu_model in menus {
            let Some(label) = menu_model.get("label").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let enabled = !menu_model
                .get("disabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let Some(id) = menu_model.get("id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let top = muda::Submenu::with_id(id, label, enabled);
            if let Some(items) = menu_model
                .get("items")
                .and_then(serde_json::Value::as_array)
            {
                append_windows_items(&top, items);
            }
            menu.append(&top)
                .map_err(|error| format!("could not append Windows shell menu: {error}"))?;
        }
    }
    let hwnd = windows_hwnd(window)?;
    // SAFETY: the raw handle comes from the live Winit window owned by this
    // UI thread, and the menu is stored for the window's lifetime by the caller.
    unsafe { menu.init_for_hwnd(hwnd) }
        .map_err(|error| format!("could not attach Windows shell menu: {error}"))?;
    Ok(menu)
}

#[cfg(target_os = "windows")]
fn windows_hwnd(window: &winit::window::Window) -> Result<isize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("could not read Windows window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        other => Err(format!("expected a Win32 window handle, got {other:?}")),
    }
}

#[cfg(target_os = "windows")]
fn append_windows_items(menu: &muda::Submenu, entries: &[serde_json::Value]) {
    for entry in entries {
        if entry.get("type").and_then(serde_json::Value::as_str) == Some("separator") {
            append_windows_item(menu, &muda::PredefinedMenuItem::separator());
            continue;
        }
        let (Some(id), Some(label)) = (
            entry.get("id").and_then(serde_json::Value::as_str),
            entry.get("label").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        let enabled = !entry
            .get("disabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let children = entry.get("children").and_then(serde_json::Value::as_array);
        if let Some(children) = children.filter(|list| !list.is_empty()) {
            let submenu = muda::Submenu::with_id(id, label, enabled);
            append_windows_items(&submenu, children);
            append_windows_item(menu, &submenu);
        } else if entry.get("checked").is_some() {
            let checked = entry
                .get("checked")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let item = muda::CheckMenuItem::with_id(id, label, enabled, checked, None);
            append_windows_item(menu, &item);
        } else {
            let item = muda::MenuItem::with_id(id, label, enabled, None);
            append_windows_item(menu, &item);
        }
    }
}

#[cfg(target_os = "windows")]
fn append_windows_item(menu: &muda::Submenu, item: &dyn muda::IsMenuItem) {
    if let Err(error) = menu.append(item) {
        tracing::warn!(%error, "could not append Windows shell menu item");
    }
}

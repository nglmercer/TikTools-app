#![forbid(unsafe_code)]
// Release plugin executables must not allocate a console on Windows; the
// host launches them with CREATE_NO_WINDOW and talks over piped stdio.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

//! Global hotkey / key-sequence process plugin for TikTools.
//!
//! A background listener records every global key press (modifiers, rolling
//! key sequence) and the host picks the batch up through its one-second
//! `poll` call, publishing one `hotkey.pressed` event per press. Behaviors
//! match chords with `eq` on `event.data.key`/`event.data.modifiers` and
//! phrases with `contains` on `event.data.sequence`.
//!
//! The executable is still trusted code with the operating-system permissions
//! of the user; a process boundary is not an OS sandbox. Platform notes:
//! macOS needs an Accessibility grant (silently delivers nothing without it),
//! Linux uses X11 on X11 sessions, the XDG Desktop Portal GlobalShortcuts
//! backend for registered chords on Wayland, and an opt-in evdev backend for
//! arbitrary keys/sequences (automatic per-device ACL, never a root app). Windows
//! needs nothing beyond a normal user session.
//!
//! Listener health is explicit: `hotkey.status` poll events plus the
//! `hotkey.status` and `hotkey.diagnostics` actions report which backend is
//! active, which needs permission, and which is unsupported — the plugin
//! never polls silently after a listener failure. On Linux, raw input asks
//! Polkit for a per-device ACL automatically when a Behavior needs it. See
//! `hotkeys/` for the backend layout and `docs/HOTKEYS_LINUX.md` for setup.

mod hotkeys;

use hotkeys::HotkeyPlugin;

tiktools_plugin_sdk::tiktools_process_plugin!(HotkeyPlugin);

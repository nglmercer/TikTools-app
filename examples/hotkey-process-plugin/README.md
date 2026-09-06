# Global hotkey plugin example

A crash-isolated TikTools process plugin that publishes `hotkey.pressed`
events for global shortcuts and key sequences. Behaviors trigger on them
like any other event: match chords with `eq` on `event.data.key` or
`event.data.modifiers`, phrases with `contains` on `event.data.sequence`.

The executable watches the OS keyboard, tracks modifiers plus a rolling
8-key sequence, and answers the host `poll` call with everything observed
since the previous tick. It never sends keystrokes anywhere; it only reports
what was pressed. The implementation uses `tiktools-plugin-sdk` for framing,
typed events, and protocol plumbing; it remains `#![forbid(unsafe_code)]`.

Backend layout (`src/hotkeys/`):

```text
mod.rs         plugin wiring, poll/actions, shared queues
event.rs       KeyState, key normalization, sequence history
detect.rs      Linux session detection (Wayland vs X11 vs unknown)
state.rs       backend abstraction, capabilities, listener status
shortcuts.rs   portal chord model + hotkey.bind parsing
platform.rs    per-OS startup
rdev_backend.rs rdev listener (Windows/macOS/X11) with failure reports
diagnostics.rs hotkey.diagnostics rendering
linux.rs       Linux orchestration
linux_portal.rs XDG Desktop Portal GlobalShortcuts backend (ashpd)
linux_evdev.rs  raw /dev/input backend for keys/sequences
```

Build it outside the application workspace:

```bash
cargo build --release --manifest-path examples/hotkey-process-plugin/Cargo.toml
```

Install a package containing these two files:

```text
hotkeys/
  plugin.json
  tiktools-hotkey-process-plugin
```

The executable must be beside `plugin.json` and have the entry name declared
by the manifest (rename the built binary, adding no extension). After copying
the directory into the user plugin directory, reload the Plugins view or
restart TikTools. No host recompilation or plugin registration is required.

The host accepts only the `events.publish` capability for these events, only
for the `hotkey.pressed` and `hotkey.status` types declared in this manifest,
and stamps identity, depth, and connection context itself.

Example filters on a `hotkey.pressed` event:

```text
event.data.key        eq        k
event.data.modifiers  eq        ctrl
event.data.sequence   contains  g o
```

Testing notes:

- the host starts this plugin automatically on its first poll tick; no
  manual start is needed after install/enable.
- enabled `hotkey.pressed` Behavior filters are synchronized automatically:
  complete `key` + non-empty `modifiers` equality filters become portal
  chords, while bare keys and sequences request raw input. A bare `Key = a`
  trigger therefore causes the plugin to request a per-device Wayland evdev
  ACL automatically through the system authorization dialog.
- the editor Run test button checks filters against the manifest sample
  (`key "k"`) or the most recent live press of the same trigger — not
  against keys pressed while the dialog is open. To verify live behavior,
  save the event, press the keys for real, and watch the Runs list.
- a mismatch names the sample data it tested, for example
  `sample data: {"key":"k",...}`, so a wrong guess reads as a data
  problem instead of a broken trigger.

Plugin actions (call from automations or the plugin inspector):

- `hotkey.bind` with config
  `{"shortcuts": [{"key": "k", "modifiers": "ctrl+shift"}], "sequencesNeeded": true}`
  registers portal chords on Wayland and toggles the raw-input backend.
  `TIKTOOLS_HOTKEY_SHORTCUTS` (same JSON array) seeds the initial set.
- `hotkey.status` returns the one-line listener summary.
- `hotkey.diagnostics` returns the full diagnostics report in its logs.

`hotkey.status` poll events fire whenever a backend starts, fails, or needs
permission, carrying per-backend `backend/state/detail/summary` plus a
capability map (`globalChords`, `arbitraryKeys`, `sequences`, `keyRelease`).

Platform notes:

- Windows: works in a normal user session, no admin needed.
- macOS: grant the process Accessibility access, otherwise the OS silently
  delivers no events and every poll comes back empty (now reported as
  `permission required` instead of silent).
- Linux X11: rdev/X11 listener. The host forwards `DISPLAY`/`XAUTHORITY`
  across the plugin environment boundary, so a healthy X11 session works.
- Linux Wayland: XDG Desktop Portal GlobalShortcuts serves registered
  chords (no device access); arbitrary keys and `sequence` triggers use the
  evdev backend. The plugin automatically requests a narrow per-device ACL
  through Polkit when raw input is needed; `input` group membership or a
  narrow udev rule remain fallbacks, never root and never `chmod 777`. Full
  setup, diagnostics, and the security model live in
  `docs/HOTKEYS_LINUX.md`.

Evaluation note on `rdev::unstable_grab`: the pinned fork has the grab
features commented out, and its grab path still opens an X11 display, so it
cannot serve Wayland either. The evdev backend uses the maintained `evdev`
crate behind the same backend abstraction instead.

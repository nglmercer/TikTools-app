# Linux hotkeys: X11, Wayland, permissions

TikTools global hotkeys work on Linux X11 **and** Wayland. Wayland
intentionally forbids arbitrary global key observation, so two different
capabilities combine on Wayland. This document explains the architecture,
the setup for Arch Linux / CachyOS, the security model, diagnostics, and
known compositor differences.

## Architecture

```text
Behavior config (hotkey.pressed chord filters)
        |
        v  hotkey.bind action / TIKTOOLS_HOTKEY_SHORTCUTS
portal backend (ashpd GlobalShortcuts) ── compositor-authorized chords
        |                                    Ctrl+Shift+K, Alt+F8, ...
        v Activated
hotkey.pressed {key, modifiers, sequence, backend:"portal", shortcut_id}

Physical keyboards ── evdev backend ── arbitrary keys + sequences
        |                              g o, media keys, keypad, ...
        v press/release
hotkey.pressed {key, modifiers, sequence, backend:"evdev"}
```

Backend selection is runtime-based (`src/hotkeys/` in
`examples/hotkey-process-plugin`):

| Session | Chords | Arbitrary keys / sequences |
|---|---|---|
| X11 | rdev/X11 listener | rdev/X11 listener (evdev fallback if `DISPLAY` is missing) |
| Wayland | Portal GlobalShortcuts | evdev backend (permission-gated) |
| Unknown/headless | portal probed, X11 if `DISPLAY` exists | evdev when sequences are wanted |

`DISPLAY` alone never selects X11: Wayland sessions expose it through
XWayland, and the detector prefers `XDG_SESSION_TYPE`, then
`WAYLAND_DISPLAY`, then `DISPLAY`.

Capabilities per backend:

| Backend | chords | arbitrary keys | sequences | key release |
|---|---|---|---|---|
| rdev (Win/macOS/X11), evdev | yes | yes | yes | yes |
| portal | yes | no | no | no |

While the portal backend is running, the evdev backend yields presses that
exactly match a portal-registered chord to the portal, so one physical press
never emits two `hotkey.pressed` events. Sequence state is shared, and it is
reset on backend (re)connect, device re-enumeration, and after 120 s of input
silence (sleep/resume, hotplug, missed releases).

## Wayland chords via the portal (no special permission)

1. Install a portal back-end for your compositor (usually already present):
   - KDE Plasma: `xdg-desktop-portal` + `xdg-desktop-portal-kde`
   - GNOME: `xdg-desktop-portal` + `xdg-desktop-portal-gnome`
   - wlroots (Sway/Hyprland): `xdg-desktop-portal` + `xdg-desktop-portal-wlr`
     (GlobalShortcuts support depends on the back-end version; see
     limitations below).
2. Create an enabled `hotkey.pressed` Behavior. Complete `key eq` plus
   non-empty `modifiers eq` filters are synchronized automatically through
   `hotkey.bind`; bare keys such as `Key = a`, sequences, and other arbitrary
   input keep evdev enabled instead. `hotkey.bind` can still be called
   manually with `{"shortcuts": [{"key": "k", "modifiers": "ctrl+shift"}]}`
   or seeded with `TIKTOOLS_HOTKEY_SHORTCUTS` before starting TikTools.
3. Approve each shortcut when the compositor prompts. KDE Plasma Wayland and
   GNOME Wayland (with a recent portal) show a system dialog; the choice is
   remembered per shortcut.

Chords that have no portal spelling (punctuation beyond single characters,
`compose`, media keys) stay on the evdev backend.

## Sequence triggers on Wayland (raw input permission)

`sequence contains g o` behaviors need the evdev backend, which reads
`/dev/input/event*`. Device nodes are normally `root:input` mode `660`.
When a raw-input Behavior is enabled, TikTools automatically asks Polkit for
a narrow read ACL on the discovered event devices. Approve the system dialog;
the app remains an ordinary user process and the permission applies immediately.
If the system has no Polkit agent or `setfacl`, use one of these fallbacks:

### Option A — active-seat ACL (default on most desktops)

systemd-logind already grants the active local session access. If
`hotkey.diagnostics` reports `readable 0`, continue below.

### Option B — `input` group (Arch / CachyOS fallback)

```bash
sudo usermod -aG input "$USER"
# re-login (group membership applies to new sessions only)
groups | tr ' ' '\n' | grep '^input$'
```

This grants read access to **all** input devices (keyboard and mouse), so
treat it as a sensitive grant. A complete logout/login is required before a
running desktop process receives the new group membership.

### Option C — narrow udev rule (one keyboard, one group)

Find your keyboard:

```bash
ls -l /dev/input/by-id/ | grep -i kbd
udevadm info -a -n /dev/input/eventX | grep -m2 'ATTRS{name}\|ATTRS{idVendor}'
```

Create `/etc/udev/rules.d/70-tiktools-hotkey.rules`:

```udev
# Only this keyboard, readable by the tiktools-input group.
SUBSYSTEM=="input", ENV{ID_INPUT_KEYBOARD}=="1", ATTRS{name}=="YOUR KEYBOARD NAME", GROUP="tiktools-input", MODE="0640"
```

Then:

```bash
sudo groupadd -r tiktools-input
sudo usermod -aG tiktools-input "$USER"
sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=input
# re-login, then check hotkey.diagnostics
```

Verify without running anything as root:

```bash
python3 -c "import os; print([f for f in sorted(os.listdir('/dev/input')) if f.startswith('event')])"
# readable check:
for f in /dev/input/event*; do [ -r "$f" ] && echo "readable: $f"; done | head
```

### Never do this

- `sudo tiktools` / running the desktop app as root.
- `chmod 777 /dev/input/event*` (or any global weakening of `/dev/input`).
- `LD_PRELOAD` tricks or setuid wrappers around the plugin.

## Security analysis

**Portal-authorized shortcuts** disclose the least: the compositor shows the
user exactly which chords TikTools wants, the user approves each one, and
TikTools learns nothing except "approved chord X was pressed". It cannot see
passwords, other windows' input, or mouse movement. This is the only Wayland
path that needs no device permission, and it is why chords are preferred.

**Raw evdev listening** sees every key on the devices it opens, including
input destined for other applications. That is inherent to sequence triggers
(`g o` cannot be recognized without observing `g` and `o` globally). The
exposure is bounded by:

- the plugin runs as your user, never as root; only the user-approved Polkit
  helper applies a per-device read ACL;
- access is granted by the OS (seat ACL, group, udev) and revocable by
  removing that grant;
- the plugin forwards only normalized `(key, pressed)` pairs to the host —
  no device passthrough, no file access, no network;
- diagnostics and logs never print sequences or keystrokes;
- disabling sequences (`"sequencesNeeded": false` in `hotkey.bind`) stops
  the evdev backend entirely.

If sequence triggers are not needed, keep them disabled and stay on the
portal alone.

## Diagnostics

Call the `hotkey.diagnostics` action (logs carry the report) or watch
`hotkey.status` poll events. Example:

```text
TikTools Hotkey Diagnostics

OS:
  linux

Session:
  wayland

Environment:
  XDG_SESSION_TYPE=wayland
  WAYLAND_DISPLAY=wayland-0
  DISPLAY=:0
  XDG_CURRENT_DESKTOP=KDE

Backends:
  XDG Desktop Portal:
    status: active
    detail: 3 shortcuts bound
  raw input (evdev):
    status: permission required
    detail: no readable /dev/input/event* devices; ...
  evdev devices: discovered 3, readable 0

Selected:
  Global Hotkeys: active via XDG Desktop Portal (3 shortcuts bound)

Capabilities:
  registered shortcuts: 3
    - TikTools hotkey Ctrl+Shift+K (ctrl-shift-k)
  arbitrary key sequences: requested (raw input backend)

Recommendation:
  Grant raw-input access to enable sequence triggers: ... See
  docs/HOTKEYS_LINUX.md for the udev/input-group setup.
```

Status lines the UI can show directly: `Global Hotkeys: active via X11`,
`Global Hotkeys: active via XDG Desktop Portal`,
`Global Hotkeys: permission required for raw input (evdev)`,
`Global Hotkeys: X11 listener failed because DISPLAY is unavailable`, and
equivalents for unsupported backends.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| X11: `X11 display unavailable` | plugin lost `DISPLAY`/`XAUTHORITY` | fixed in the loader (forwards desktop env); check `hotkey.diagnostics` env block |
| Wayland chords never fire | shortcuts not registered/approved | save an enabled chord Behavior, then approve the compositor prompt |
| Wayland sequences never fire | evdev permission | approve the automatic Polkit request; input group / udev rule are fallbacks |
| `hotkey.status` shows portal `unsupported` | old `xdg-desktop-portal-wlr` or missing back-end | update portal packages; chords need a back-end with GlobalShortcuts |
| Duplicate events for one press | portal + evdev both emitted | evdev yields bound chords to a running portal; if you see this, report backend names from `event.data.backend` |
| Stuck modifier after sleep | missed release | state expires after 120 s idle and resets on reconnect |

## Known limitations

- GlobalShortcuts availability and approval UX differ per portal back-end.
  KDE (`xdg-desktop-portal-kde`) and GNOME (`xdg-desktop-portal-gnome`)
  support it in recent versions; `xdg-desktop-portal-wlr` support depends on
  version and compositor — the plugin reports `unsupported` with a reason
  instead of pretending.
- The portal cannot observe arbitrary keys, releases, or sequences by
  design; anything beyond registered chords needs evdev.
- Portal preferred triggers are advisory: the compositor may assign a
  different trigger, shown in system settings.
- `hotkey.bind` is the host synchronization point from enabled behavior
  filters to portal bindings. The host sends it asynchronously after Behavior
  changes, so a complete chord is registered without blocking the webview.
- A bare key filter such as `event.data.key eq a` cannot be served by the
  portal. The editor marks it as requiring raw keyboard access on Wayland and
  the plugin automatically requests the evdev/input-device permission when
  that Behavior is enabled.
- Non-US layouts: key names are physical (`KeyK` → `k`) on rdev and kernel
  codes on evdev, so positions stay stable while labels vary; dead keys
  produce no press event on either path.

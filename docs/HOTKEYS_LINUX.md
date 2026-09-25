# Linux hotkeys: X11, Wayland, permissions

Global hotkeys ship as a TypeScript napi-vm plugin
(`examples/hotkey-napi-plugin`) backed by the `rdev-node` native addon.
The guest watches the OS keyboard through one rdev listener, tracks
modifiers plus a rolling 8-key sequence, and answers the host `poll`
call with everything observed since the previous tick as
`hotkey.pressed` events. The host projects enabled Behaviors into the
guest through the `hotkey.bind` action (`shortcuts` chords plus the
`sequencesNeeded` raw-input opt-in).

## Architecture

```text
Behavior config (hotkey.pressed chord filters)
        |
        v  hotkey.bind action {shortcuts, sequencesNeeded}
TypeScript guest (napi-vm, sandboxed JS)
        |
        v  require('rdev-node') — host-authorized native alias
rdev listener ── every key, releases, sequences ── chords too
        |
        v press (poll)
hotkey.pressed {key, modifiers, sequence, backend:"rdev"}
hotkey.status {platform, session, backends:[{backend, state, capabilities}]}
```

Backend selection is capability-based — the guest observes no platform
or session facts by design, so `hotkey.status` reports both as
`unknown`:

| Session | Chords | Arbitrary keys / sequences |
|---|---|---|
| X11 | rdev listener (no special permissions) | rdev listener |
| Wayland | evdev path, fails closed without `/dev/input` read access | same |
| Windows / macOS | rdev listener | rdev listener |

rdev-node picks its backend from the session: X11 sessions use
`rdev::listen` (any desktop user, no extra permissions), Wayland
sessions read the physical devices through evdev. The evdev path needs
persistent read access to `/dev/input/event*` (seat ACL, `input` group,
or a udev rule); without it the listener fails to start and the guest
reports `state: "failed"` with the rdev error verbatim as the backend
detail. The UI surfaces it through the standard `hotkey.status` badge.
There is intentionally no silent fallback: a failed backend must look
failed.

One backend quirk to know: the X11 `rdev` hook cannot restart after a
stop, while the Wayland evdev loop can. A sequences-off `hotkey.bind`
followed by a re-bind on X11 therefore reports the backend `failed`
until the plugin reloads; the guest surfaces this like any other
backend failure.

`hotkey.bind` re-decides the listener on every projection: it runs
while sequences are wanted **or** chords are watched, and stops only
when the plugin is fully unbound with sequences off.

## What the retired process plugin did differently

The previous Rust process plugin (`examples/hotkey-process-plugin`,
removed) additionally served Wayland through two host-side backends:
compositor-authorized chords via the XDG Desktop Portal
GlobalShortcuts interface, and arbitrary keys/sequences via an
evdev backend reading `/dev/input/event*` behind a Polkit / input-group
/ udev permission story.

A napi-vm guest cannot do either: it has no D-Bus, no device, no `fs`,
and no `process.env` access. The portal and evdev paths therefore
moved to the roadmap as **host capability modules** — Rust code in the
loader, granted per plugin from the manifest — instead of guest code.
Until they land, Wayland sessions have no global-hotkey observation;
run TikTools on X11 (or XWayland-hosted X11) for working hotkeys.

## Security analysis

**Raw rdev listening** sees every key while the listener runs,
including input destined for other applications. That is inherent to
global hotkeys and sequence triggers (`g o` cannot be recognized
without observing `g` and `o` globally). The exposure is bounded by:

- the native addon runs with TikTools process privileges and is **not**
  sandboxed; it is authorized explicitly per plugin through the
  `nativeAddons` manifest declaration plus `"trust": "trusted"`
  (see `docs/NAPI_VM_PLUGINS.md`);
- the guest forwards only normalized `(key, pressed)` pairs to the
  host — no device passthrough, no file access, no network;
- diagnostics and logs never print sequences or keystrokes (overflow
  lines carry counts only);
- a fully unbound plugin with sequences off stops the listener
  entirely.

Never run the desktop app as root to "fix" hotkeys, and never weaken
`/dev/input` permissions: the napi-vm build does not read input
devices at all.

## Diagnostics

Call the `hotkey.diagnostics` action (logs carry the report) or watch
`hotkey.status` poll events. Example:

```text
Hotkey diagnostics (napi-vm):
  runtime: napi-vm (TypeScript guest, rdev-node listening)
  backend: rdev state=running
  stop: available
  bindings: 2 chord(s) watched, sequences enabled
  portal: unsupported (compositor chords need the retired process plugin)
  session: unknown (guests observe no platform facts)
Queue:
  dropped events (overflow): 0
  pending events: 0
```

`hotkey.status` carries one backend entry with `state` (`starting`,
`running`, `permission-required`, `unsupported`, `failed`), a
capability map (`globalChords`, `arbitraryKeys`, `sequences`,
`keyRelease`), and queue counters (`droppedEvents`, `pendingEvents`).
The UI needs no update for new states: `running` and
`permission-required` are already recognized.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Wayland: backend `failed`, `/dev/input` denied | evdev path needs raw-input access | grant `/dev/input` read access (seat ACL, `input` group, or udev rule), or run on X11 where no extra permissions are needed |
| X11: `failed` with display error | plugin lost `DISPLAY`/`XAUTHORITY` | check `hotkey.diagnostics`; the loader forwards desktop env |
| Chord-only Behavior never fires | listener stopped | fixed: watched chords keep the listener running (`wantsListening`) |
| `hotkey event queue overflowed` in logs | consumer slower than typist | counts only, no key contents; poll drains 16 events per tick |
| Stuck modifier after sleep | missed release | state expires after 120 s idle and resets on restart |

## Known limitations

- The X11 backend cannot restart its hook after a stop: re-binding
  after a sequences-off `hotkey.bind` reports `failed` until reload.
  The Wayland evdev backend restarts cleanly. Staging an rdev-node as
  old as v1.0.1 additionally loses `stopListener` entirely; the guest
  detects that and says so loudly instead of pretending to stop.
- Pure Wayland sessions need `/dev/input` read access for any
  observation. Compositor-authorized portal chords remain roadmap
  (host capability module).
- Guests observe no platform facts, so status and diagnostics say
  `unknown` where the old plugin named sessions and desktops.
- Non-US layouts: key names are physical (`KeyK` → `k`), so positions
  stay stable while labels vary; dead keys produce no press event.
- `TIKTOOLS_HOTKEY_SHORTCUTS` seeding retired with the process plugin:
  bindings now arrive exclusively through the host `hotkey.bind`
  projection.

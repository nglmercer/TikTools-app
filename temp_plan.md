````markdown
# TikTools — Fix Global Hotkeys End-to-End

Repository:
https://github.com/nglmercer/TikTools-app

Branch:
`remake`

## Goal

Make **Global Hotkeys** reliable across:

```text
OS keyboard
  ↓
Global Hotkeys process plugin
  ↓
validated plugin event
  ↓
AppCore
  ├─ DomainEvent / Control API / IPC / agents
  └─ Automation matching
       ↓
     actions
       ↓
     live run/status UI
````

Hotkey events must never silently disappear, depend on WebView readiness, or execute actions without the UI reflecting them.

---

## 1. Make plugin events first-class DomainEvents

Current problem:

`hotkey.pressed` enters the automation pipeline but `publish_live_domain_event()` only promotes `tiktok.*` events.

Therefore WebView / IPC / CLI / agents cannot observe raw hotkey events.

Add a generic domain event, for example:

```rust
DomainEvent::PluginEvent {
    plugin_id: String,
    event_type: String,
    event: Value,
}
```

Requirements:

* Publish validated plugin events immediately after polling/validation.
* Do this before automation enrichment/execution.
* Include owning plugin id.
* Route through normal JSON-RPC `"event"` notifications.
* Use a stable topic such as:

```text
plugin.event
```

with payload:

```json
{
  "pluginId": "hotkeys",
  "eventType": "hotkey.pressed",
  "event": {
    "type": "hotkey.pressed",
    "data": {}
  }
}
```

Optionally expose specific topic aliases such as:

```text
plugin.hotkeys.hotkey.pressed
```

Do not special-case only hotkeys. Fix this generically for all plugin-declared events.

---

## 2. Fix missing live automation run updates

Current backend emits:

```rust
HostMessage::BehaviorRuns { runs }
```

after real automation execution.

Current frontend `src/web/features/automation.ts` does not consume `behavior-runs`.

Result:

```text
hotkey received
→ behavior matched
→ action executed
→ backend emitted updated runs
→ frontend ignored message
→ UI looks broken
```

Fix frontend subscription:

```ts
control.onPush('behavior-runs', ...)
```

and update:

```ts
behaviorRuns.value
```

Preferred long-term fix:

Add authoritative domain event:

```text
automation.runs.changed
```

or:

```text
automation.run.completed
```

and migrate frontend to `control.onTopic(...)`.

Do not require manual refresh to see a real hotkey-triggered run.

---

## 3. Fix hotkey event loss: 64 queued → only 16 consumed

Current plugin can queue:

```rust
MAX_PENDING_EVENTS = 64
```

Current plugin poll drains everything:

```rust
queue.drain(..)
```

Host later limits:

```rust
MAX_POLLED_EVENTS_PER_TICK = 16
```

This silently destroys every event after the first 16.

Example:

```text
40 queued
→ plugin drains 40
→ host accepts 16
→ 24 permanently lost
```

Fix generically.

Preferred approaches:

```text
A. Plugin poll only drains maximum protocol batch size
OR
B. Protocol supports hasMore / pagination
OR
C. Host accepts complete bounded plugin response
```

Never remove events from the producer queue if the consumer will discard them.

Add tests with >16 hotkey events proving all events are eventually processed in order.

---

## 4. Start plugin polling with AppCore/host, not WebView

Current desktop starts plugin polling from `frontend_ready()`.

This makes spontaneous plugin events depend on Vue/WebView initialization.

Wrong:

```text
WebView ready
  ↓
start plugin poll
```

Required:

```text
AppCore/control host starts
  ↓
start plugin poll
```

Requirements:

* Desktop starts polling as part of host/runtime startup.
* Headless `host --ipc` starts polling.
* Headless stdio host starts polling when appropriate.
* WebView refresh/reload must not stop hotkeys.
* WebView never becomes owner of plugin lifecycle.
* `spawn_plugin_event_poll()` remains idempotent.

Add tests proving `hotkey.pressed` can be processed with no WebView.

---

## 5. Make Global Hotkeys status visible even when healthy

Current Behavior UI only renders hotkey status when:

```ts
hotkeySummary.needsAttention
```

This hides the healthy state.

Always show a compact Global Hotkeys status when the plugin is installed/enabled.

Example:

```text
Global Hotkeys
● Active · Native listener
Last event: Ctrl+K · 2s ago
```

States should clearly distinguish:

```text
Active
Starting
Permission required
Failed
Unsupported
No events received yet
```

Do not make users infer whether the listener exists.

---

## 6. Track last received hotkey event

Add lightweight runtime diagnostics for:

```text
last event timestamp
key
modifiers
backend
sequence
```

Example:

```json
{
  "pluginId": "hotkeys",
  "lastEventAt": 123456789,
  "lastEvent": {
    "key": "k",
    "modifiers": "ctrl",
    "backend": "rdev"
  }
}
```

Expose this through plugin health/status or another read-only diagnostics RPC.

Do not persist keyboard history to disk.

Keep only minimal in-memory diagnostics.

---

## 7. Preserve event ownership metadata

`make_plugin_event()` currently creates the typed event but does not visibly stamp its plugin owner.

Add explicit host-owned metadata, for example:

```json
{
  "type": "hotkey.pressed",
  "source": {
    "kind": "plugin",
    "pluginId": "hotkeys"
  },
  "data": {}
}
```

The plugin must not be allowed to spoof another plugin id.

Host determines ownership from the manifest/polling plugin.

This metadata should survive:

```text
poll
→ DomainEvent
→ automation
→ IPC
→ WebView
```

---

## 8. Keep hotkey event publication independent from automation success

A valid `hotkey.pressed` event must still reach subscribers if:

```text
automation has no matching behavior
automation action fails
automation is overloaded
processor/enrichment fails
```

Correct ordering:

```text
plugin poll
  ↓
validate event
  ↓
publish authoritative PluginEvent
  ↓
record diagnostics
  ↓
run automation independently
```

Do not make control-plane visibility dependent on automation execution.

---

## 9. Improve hotkey polling observability

Add structured logs/metrics around:

```text
plugin started
backend listener active
poll requested
events returned
events accepted
events dropped
event queue overflow
automation matched
automation not matched
action completed
action failed
```

Example fields:

```text
plugin=hotkeys
event_type=hotkey.pressed
backend=rdev
queued=4
polled=4
accepted=4
```

Avoid logging arbitrary key history or sensitive keyboard contents.

---

## 10. Fix queue overflow semantics

Current plugin silently removes oldest events when:

```rust
queue.len() > MAX_PENDING_EVENTS
```

Keep a bounded queue, but make drops observable.

Add counter such as:

```text
droppedHotkeyEvents
```

and report it through:

```text
hotkey.status
plugin health
system.health
```

Log a rate-limited warning when overflow occurs.

Never silently lose events.

---

## 11. Ensure hotkey plugin is available in packaged releases

Current release package creates an empty:

```text
TikTools/plugins/
```

directory.

If Global Hotkeys is an official TikTools feature, package it.

Expected Windows release:

```text
TikTools/
  TikTools.exe
  plugins/
    hotkeys/
      plugin.json
      tiktools-hotkey-process-plugin.exe
```

Equivalent native binary for Linux/macOS.

Requirements:

* Build correct target binary.
* Stage manifest + executable.
* Preserve executable permissions on Unix.
* Validate plugin exists in packaged archive.
* Fail release packaging if required built-in plugin is missing.

If hotkeys intentionally remain optional instead of bundled, make that explicit in UI/docs and provide a real installation path. Do not advertise Global Hotkeys as available when no plugin exists.

---

## 12. Verify Windows native listener path

For Windows:

```text
rdev::listen
→ EventType::KeyPress / KeyRelease
→ key_name()
→ emit_press()
→ pending queue
→ Plugin::poll()
```

Add Windows-focused tests where possible around:

```text
key normalization
modifier normalization
press/release state
auto-repeat suppression
queueing
poll serialization
```

Do not replace `rdev` unless a reproducible listener failure is found.

The current architecture is valid; fix downstream delivery first.

---

## 13. Keep hotkey filter contract consistent

Current event contract is:

```json
{
  "data": {
    "key": "k",
    "modifiers": "ctrl",
    "sequence": "g k",
    "backend": "rdev"
  }
}
```

Therefore behavior filters should be:

```text
event.data.key        eq        k
event.data.modifiers  eq        ctrl
```

Not:

```text
event.data.key        eq        ctrl+k
```

Make editor hints/examples consistent with this contract.

Add regression tests for:

```text
Ctrl+K
Ctrl+Shift+K
bare A
sequence G O
function keys
arrows
numpad keys
```

---

## 14. Make hotkey binding synchronization observable

Current host projects enabled Behavior filters into:

```text
hotkey.bind
```

Add explicit diagnostics showing current synchronized config:

```json
{
  "shortcuts": [
    {
      "key": "k",
      "modifiers": "ctrl"
    }
  ],
  "sequencesNeeded": false,
  "revision": 12
}
```

Expose whether:

```text
desired revision
==
applied revision
```

If synchronization fails, surface the failure in plugin health/UI.

Do not silently continue with stale bindings.

---

## 15. Handle plugin restart/recovery correctly

If hotkey process:

```text
crashes
times out
is killed
returns malformed protocol
listener exits
```

the host should:

```text
mark unhealthy
back off
restart process
re-send desired hotkey.bind configuration
resume polling
```

A restarted process must not come back with empty/stale portal bindings.

Add regression test:

```text
configure Ctrl+K
→ plugin running
→ simulate plugin restart
→ host restarts it
→ host re-sends binding projection
→ Ctrl+K events resume
```

---

## 16. Domain-event topics for automation state

Reduce remaining dependence on legacy HostMessage for this path.

Introduce/migrate topics such as:

```text
plugin.event
plugin.status
automation.run.completed
automation.runs.changed
```

Frontend should eventually use:

```ts
control.onTopic(...)
```

instead of requiring compatibility pushes for core runtime state.

Keep old HostMessage paths temporarily only where compatibility requires them.

---

## 17. Add end-to-end tests

Add tests covering the complete pipeline:

```text
fake hotkey plugin event
→ poll
→ validation
→ PluginEvent domain publication
→ automation matching
→ action execution
→ run recorded
→ subscriber receives event
```

Required regression tests:

1. Plugin starts without WebView.
2. `hotkey.pressed` reaches automation.
3. `hotkey.pressed` reaches DomainEvent subscribers.
4. IPC subscriber receives plugin event.
5. More than 16 queued events are not silently lost.
6. Queue overflow increments drop diagnostics.
7. `behavior-runs` / automation run topic updates frontend state.
8. Healthy hotkey status is available.
9. Disabled hotkey plugin does not publish events.
10. Restarted plugin receives bindings again.
11. Wrong/undeclared plugin event types are rejected.
12. A failed automation action does not suppress the original hotkey event.
13. Headless control host receives hotkeys without WebView.
14. Packaged release contains the required hotkey plugin.

---

## 18. Manual verification

Run development host:

```bash
bun run scripts/start-dev.ts
```

Verify plugin:

```bash
cargo run -p tiktools-cli -- plugin get hotkeys --json
cargo run -p tiktools-cli -- plugin health hotkeys --json
cargo run -p tiktools-cli -- plugin action hotkey.status --live --json
cargo run -p tiktools-cli -- plugin action hotkey.diagnostics --live --json
```

Create enabled behavior:

```text
trigger: hotkey.pressed

event.data.key        eq  k
event.data.modifiers  eq  ctrl
```

Press:

```text
Ctrl+K
```

Verify all of these independently:

```text
native listener received it
plugin queue received it
plugin poll returned it
DomainEvent published it
IPC subscriber received it
automation matched it
configured action executed
Runs UI updated immediately
last-hotkey diagnostics updated
```

---

## Acceptance Criteria

The fix is complete only when:

```text
Global Hotkeys works without WebView readiness.
hotkey.pressed is observable through the control plane.
Automation executes from real key presses.
Runs UI updates immediately after execution.
Healthy/failed listener status is visible.
No 64→16 silent event loss exists.
Queue overflow is observable.
Plugin restart restores bindings.
Headless host receives plugin events.
Release builds include the hotkey plugin if it is an official feature.
No event path depends on legacy UI messages for authoritative delivery.
```

Run:

```bash
bun run lint
bun run typecheck
bun test
bun run build:web

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo check --workspace --locked

bun run scripts/build-plugin.ts --all --debug
```

Do not fix this with hotkey-specific frontend hacks. Fix plugin-event ownership, polling lifecycle, event transport, queue semantics, automation notifications, diagnostics, and packaging at the architecture boundaries.

```
```

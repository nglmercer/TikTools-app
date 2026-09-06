# Automations

Automation data is stored in Rust and edited in the existing Vue UI.
TikTok events enter the core event bus, where points, behavior records,
workflow execution, plugins, and the UI can consume JSON-safe values.

```text
native TikTok event
       │
       ▼
  AppEvent::TikTok
       │
       ├── points service
       ├── behavior/workflow services
       ├── runtime plugins
       └── HostMessage / automation context
```

## Behavior records

An action is a reusable configured action type. An event selects a trigger,
filters, cooldown, and action ids. Existing SQLite tables remain:

```text
behavior_actions
behavior_events
behavior_plugins
```

The host sends a behavior snapshot containing saved records, built-in action
descriptors, runtime plugin descriptors, and translations. The UI renders
descriptor fields and does not execute action code.

Supported trigger values are:

```text
tiktok.chat          tiktok.gift       tiktok.like
tiktok.follow        tiktok.share      tiktok.join
tiktok.social        tiktok.room_stats
tiktok.connected     tiktok.disconnected
points.awarded       plugin.emit
```

Plugins may declare extra trigger types (for example `hotkey.pressed`); they
arrive in the behavior snapshot, appear in the event picker while their
plugin is installed and enabled, and match through the same
trigger/filters/cooldown pipeline. See `docs/PLUGINS.md` for the manifest
contract.

Filters use dotted JSON paths such as `event.data.diamondCount` and
`event.user.uniqueId`. The registry at
`src/automation/contracts/generated/event-registry.generated.ts` is generated
from the Rust contracts in `crates/tiktools-core/src/contracts/` and provides
editor samples, labels, types, and source-field metadata. `AutomationEvent` and
the per-event data types in `src/automation/contracts/generated/` are the only
frontend contract source; consumers must not recreate TikTok event interfaces
in feature code.

Regenerate and verify the checked-in artifacts with:

```bash
bun run contracts:generate
bun run contracts:check
bun run deps:check
```

The host intentionally exposes only fields it emits. For example, user
`avatarUrl`, gift `toUser`, and room-stat `topViewers` are not automation fields
unless the Rust boundary starts emitting them and the generated contracts are
updated together.

## Built-in action types

The Rust host owns the built-in catalog. It includes HTTP, internal event
emission, points, delay, logging, script, and local audio actions. The audio
action stores only a path-backed media reference and revalidates it before
streaming; it never copies the sound into TikTools. Each action declares its required
capability and configuration fields. Plugin actions arrive only from runtime
manifests.

## Scripts

The `core.code` action and the workflow script node execute through the
pure-Rust `napi-vm` adapter. A script receives JSON globals:

```js
event   // normalized automation event
inputs  // node/action inputs
data    // alias for inputs
```

The host enforces source and result size limits plus a loop budget. The VM has
no Node module loader, filesystem, network, process, WebView, or database
handle. Scripts return JSON; privileged work is expressed as host actions and
capabilities.

## Workflows

Workflow graphs stay JSON and retain schema version 1 for database
compatibility. Node definitions are returned by `get-automation-nodes` and are
not compiled into the WebView. The current native catalog covers:

- event trigger;
- compare condition;
- template and script transforms;
- delay and cooldown controls;
- log, HTTP, and points actions.

The editor uses stable node types, versions, ports, and JSON configuration
schemas. Unknown nodes remain visible in saved graphs so a missing runtime can
be diagnosed instead of silently deleting user data.

## Capabilities

Host capabilities are explicit interfaces in core. HTTP, audio, TTS, points,
storage, and native integrations are never exposed as arbitrary WebView
objects. Runtime process/WASM plugins must declare the capability and
permission they request. Trusted native plugins are an OS-level trust boundary
and cannot be sandboxed by a JSON manifest.

## Event context and previews

The core keeps only the most recent normalized event in memory and exposes it
through `get-automation-context`. The WebView uses that value, plus registry
samples, for template suggestions and script-editor previews. Live user data is
not persisted as editor context. `TemplateField` and `CodeEditor` share the
autocomplete controller, token parser, scoring, and `AutocompleteList`.

## Testing

```bash
bun run typecheck
bun run test
cargo test -p tiktools-core
cargo test --workspace
```

The tests cover event-registry path drift, filter behavior, bounded VM
execution, message parsing, SQLite round trips, plugin manifest validation,
framed process messages, and the native TikTok event model.

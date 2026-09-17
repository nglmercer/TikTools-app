# TikTools-app — SonicBoom Output Device Integration

Repository:

```text
https://github.com/nglmercer/TikTools-app
```

Target branch:

```text
remake
```

## Objective

Update the existing declarative SonicBoom integration so TikTools users can select the audio output device exposed by SonicBoom.

Do not implement OS audio-device discovery directly in TikTools.

SonicBoom is responsible for:

```text
enumerating devices
opening devices
switching playback output
tracking selected output
```

TikTools should only consume SonicBoom's HTTP APIs.

The SonicBoom integration is declarative and should remain declarative.

---

# 1. Expected SonicBoom API

Assume SonicBoom exposes:

```text
GET  /api/audio/devices
GET  /api/audio/output
POST /api/audio/output
```

All requests use the existing SonicBoom bearer authentication.

## Device list example

```json
{
  "devices": [
    {
      "id": "default",
      "name": "System Default",
      "is_default": true,
      "is_selected": false
    },
    {
      "id": "CABLE Input (VB-Audio Virtual Cable)",
      "name": "CABLE Input (VB-Audio Virtual Cable)",
      "is_default": false,
      "is_selected": true
    }
  ],
  "selected": "CABLE Input (VB-Audio Virtual Cable)"
}
```

## Device selection request

```http
POST /api/audio/output
Content-Type: application/json
```

```json
{
  "device": "CABLE Input (VB-Audio Virtual Cable)"
}
```

---

# 2. Keep plugin declarative

The existing SonicBoom integration lives around:

```text
examples/sonicboom-server/plugin.json
```

It already uses a dynamic option source for voices:

```json
"optionSources": {
  "voice": {
    "path": "/v1/voices"
  }
}
```

Use the same generic declarative option-source mechanism for audio devices.

Do not add a SonicBoom-specific native Rust or TypeScript audio backend.

---

# 3. Add output-device option source

Add an option source for SonicBoom audio outputs.

Conceptually:

```json
"optionSources": {
  "voice": {
    "path": "/v1/voices"
  },
  "outputDevice": {
    "path": "/api/audio/devices",
    "itemsPath": "devices",
    "valuePath": "id",
    "labelPath": "name"
  }
}
```

Use the exact schema already supported by TikTools.

Do not extend the declarative schema if the existing `itemsPath`, `valuePath`, and `labelPath` functionality is sufficient.

---

# 4. Add SonicBoom setting

Add an output-device setting.

Example:

```json
"outputDevice": {
  "type": "string",
  "title": "Audio output",
  "description": "Audio device used by the SonicBoom server for playback.",
  "default": "default"
}
```

In `uiHints`:

```json
"outputDevice": {
  "optionsFrom": "plugin-action-options:sonicboom.server.speak:outputDevice"
}
```

However, simply saving a local TikTools setting is not sufficient.

Changing this value must call SonicBoom's:

```text
POST /api/audio/output
```

The UI must reflect the actual server state.

---

# 5. Determine the best declarative architecture

Review the current declarative plugin feature set before adding custom functionality.

Preferred order:

1. Reuse existing settings/action/option-source mechanisms.
2. Extend the generic declarative plugin system only if needed.
3. Avoid SonicBoom-specific hardcoded frontend/backend logic.

If settings cannot currently trigger an HTTP mutation when changed, introduce a generic declarative mechanism rather than a SonicBoom-only exception.

For example, a generic settings action concept could allow:

```json
"outputDevice": {
  "type": "string",
  "title": "Audio output"
}
```

with a corresponding generic remote mutation:

```json
"onChange": {
  "http": {
    "method": "POST",
    "path": "/api/audio/output",
    "body": {
      "device": "{{ value }}"
    }
  }
}
```

This is only an architectural example.

Use naming and structures consistent with the existing schema.

If adding generic schema functionality, document and test it thoroughly.

---

# 6. Preferred alternative: explicit plugin action

If generic settings mutation would require excessive framework changes, add a declarative action such as:

```text
sonicboom.server.set-output-device
```

Example conceptual manifest:

```json
{
  "id": "sonicboom.server.set-output-device",
  "version": 1,
  "title": {
    "default": "Set SonicBoom audio output"
  },
  "requiredCapabilities": [
    "http.request"
  ],
  "fields": [
    {
      "key": "device",
      "label": {
        "default": "Audio output"
      },
      "kind": "select",
      "value": "default",
      "optionsFrom": "plugin-action-options:sonicboom.server.set-output-device:device"
    }
  ],
  "http": {
    "method": "POST",
    "path": "/api/audio/output",
    "headers": {
      "Content-Type": "application/json"
    },
    "body": "{\"device\":\"{{ config.device }}\"}"
  },
  "optionSources": {
    "device": {
      "path": "/api/audio/devices",
      "itemsPath": "devices",
      "valuePath": "id",
      "labelPath": "name"
    }
  }
}
```

If there is already a cleaner declarative settings mechanism in the repository, use it instead.

---

# 7. GUI requirements

The SonicBoom settings UI should let the user clearly see and choose:

```text
Audio output
[ System Default                          ▼ ]
```

Possible options:

```text
System Default
Speakers (Realtek(R) Audio)
Headphones
CABLE Input (VB-Audio Virtual Cable)
VoiceMeeter Input
BlackHole 2ch
...
```

Do not hardcode platform-specific device names.

Load them from SonicBoom.

---

# 8. Loading states

The GUI must gracefully handle:

```text
SonicBoom offline
authentication missing
device API unsupported by older SonicBoom
no output device available
network timeout
selected device disappeared
```

Do not break the entire SonicBoom plugin settings page when device enumeration fails.

Show a useful state such as:

```text
Audio devices unavailable
```

and preserve the existing configuration.

---

# 9. Compatibility with older SonicBoom servers

Older SonicBoom versions do not expose:

```text
/api/audio/devices
/api/audio/output
```

The TikTools integration should remain usable with those versions.

If the endpoint returns:

```text
404
```

the UI should hide or disable output selection rather than marking the entire SonicBoom integration invalid.

Voice loading and TTS should continue working.

Do not require the new endpoint for the base `/ready` connection test.

---

# 10. Server source of truth

Do not assume the TikTools saved setting is the true current output.

Whenever practical, query:

```text
GET /api/audio/output
```

and use it as the authoritative state.

This matters because output might have been changed:

```text
through SonicBoom directly
by another client
by another TikTools instance
after device reconnect
```

Avoid showing stale state.

---

# 11. Synchronization

When the user selects a device:

```text
GUI select
   ↓
POST /api/audio/output
   ↓
successful SonicBoom response
   ↓
update local displayed/saved state
```

If the POST fails:

```text
do not pretend the device changed
restore previous selection
show error
```

Do not optimistically persist a state that SonicBoom rejected.

---

# 12. Authentication

The new requests must reuse the existing declarative plugin bearer auth configuration.

Do not implement another token field.

Existing:

```text
apiToken
```

must automatically authenticate:

```text
GET /api/audio/devices
GET /api/audio/output
POST /api/audio/output
```

through the generic declarative HTTP engine.

---

# 13. Rate-limit implications

SonicBoom will raise its default TTS rate limit to approximately:

```text
300 requests / 60 seconds
```

TikTools should still behave responsibly.

Do not implement aggressive automatic retry loops for `429`.

If an HTTP action receives:

```http
429 Too Many Requests
Retry-After: N
```

the generic HTTP/action runtime should preserve or expose the server error appropriately.

If TikTools already has retry handling, make sure it respects `Retry-After` rather than repeatedly hammering SonicBoom.

Do not introduce a SonicBoom-only retry implementation if the generic HTTP engine can handle this.

---

# 14. Preserve TTS queue semantics

Current TTS calls use:

```text
POST /api/tts/play
```

with:

```text
play_now=false
```

by default.

Keep this behavior.

`play_now=false` is important because SonicBoom already owns the playback queue.

TikTools should not implement a second competing playback queue for SonicBoom requests.

Expected architecture:

```text
TikTok events
    ↓
TikTools automation engine
    ↓
POST /api/tts/play?play_now=false
    ↓
SonicBoom inference admission control
    ↓
SonicBoom playback queue
    ↓
selected audio output
```

---

# 15. Interrupt behavior

Keep the existing advanced setting:

```text
Interrupt and play now
```

mapping to:

```text
play_now=true
```

It should remain opt-in.

Normal chat/event TTS should use:

```text
play_now=false
```

so messages play sequentially.

---

# 16. Plugin page improvements

Update the SonicBoom TTS page text to clarify responsibilities.

Suggested copy:

```text
Choose the voice and server-side audio output used for SonicBoom playback.
Audio outputs are discovered from the connected SonicBoom server.
```

If no playback capability exists on the server:

```text
This SonicBoom server does not expose local audio playback controls.
TTS generation remains available.
```

Use existing localization patterns.

Add translations to the relevant locale files.

At minimum:

```text
src/web/i18n-en.ts
src/web/i18n-es.ts
```

Follow the project's current i18n architecture.

---

# 17. Generic declarative framework changes

If extending the declarative plugin framework, update:

```text
docs/DECLARATIVE_PLUGINS.md
schema validation
manifest types/interfaces
Rust parser/model if applicable
frontend rendering
tests
```

Do not add undocumented schema keys.

All new generic functionality must:

```text
validate manifest input
respect host/network restrictions
reuse existing auth
reuse existing timeout restrictions
reuse existing URL/host pinning protections
preserve secret redaction
```

Never let an arbitrary WebView URL bypass the declarative HTTP allowlist.

---

# 18. Tests

Add tests for:

## Option source

```text
/api/audio/devices response maps to device select options
id -> value
name -> label
```

## Compatibility

```text
404 from /api/audio/devices does not break SonicBoom plugin
voice options continue working
TTS actions continue working
```

## Selection

```text
selecting device sends POST /api/audio/output
request body contains correct device id
bearer auth is included
success updates visible state
failure preserves previous selection
```

## Source of truth

```text
GET /api/audio/output populates active selection
external change can be reflected after refresh/reload
```

## Generic declarative framework

If schema functionality is extended, add parser and validation tests for:

```text
valid remote-setting action
invalid path
invalid method
invalid template
secret handling
network restrictions
```

---

# 19. Documentation

Update:

```text
examples/sonicboom-server/plugin.json
docs/DECLARATIVE_PLUGINS.md
relevant user-facing docs
```

Explain:

```text
SonicBoom audio devices are server-side
TikTools discovers devices through SonicBoom
System Default is always a valid logical option
older SonicBoom versions remain supported
```

---

# 20. Validation

Run the repository's normal checks.

Inspect `package.json`, Cargo workspace configuration, and CI before choosing exact commands.

At minimum run the relevant equivalents of:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

and frontend checks such as:

```bash
npm/pnpm/yarn lint
npm/pnpm/yarn test
npm/pnpm/yarn typecheck
```

Use the package manager already used by the repository.

Do not introduce another package manager.

---

# 21. End-to-end acceptance test

The completed system should support this workflow:

```text
1. Start SonicBoom.
2. Start TikTools.
3. Connect the SonicBoom declarative plugin.
4. TikTools loads voices from /v1/voices.
5. TikTools loads outputs from /api/audio/devices.
6. User chooses "CABLE Input (VB-Audio Virtual Cable)".
7. TikTools sends POST /api/audio/output.
8. SonicBoom switches its playback output.
9. TikTok chat event arrives.
10. TikTools sends POST /api/tts/play?play_now=false.
11. SonicBoom generates the audio.
12. SonicBoom queues it.
13. SonicBoom plays it through the selected virtual cable.
14. Additional chat messages remain queued in order.
15. Normal traffic does not hit the old 20 requests/minute limitation.
```

---

# 22. Non-goals

Do not:

```text
enumerate system audio devices directly from TikTools
replace SonicBoom's playback queue
create a second TTS playback queue in TikTools
hardcode Windows-only device names
require the new API for SonicBoom connection health
break older SonicBoom servers
write directly to SonicBoom's .env file
```

---

# 23. Deliverable

Implement the changes rather than only writing a design.

At completion provide:

```text
summary
files changed
manifest/schema changes
UI behavior
backward compatibility behavior
tests executed
remaining limitations
```

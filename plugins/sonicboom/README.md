# SonicBoom plugin: provider selection and voice reload

Requires the SonicBoom API in [SonicBoom PR #6](https://github.com/nglmercer/SonicBoom/pull/6).
The isolated WebView continues to use only `PluginBroker`: no direct HTTP, privileged
bridge imports or host TTS components. Output-device selection, automatic-chat policy,
queue capacity (64), replay deduplication, points and playback remain unchanged.

## Provider and voice selection

The connected server supplies `/api/tts/plugin/providers` and `/api/tts/plugin/voices`
through manifest option sources. Count enabled providers, including enabled providers
that are loading/error/not-installed. Hide the provider select with one enabled
provider; display disabled unavailable rows with multiple providers. An unavailable
selection cannot synthesize and never falls back to another provider.

Voices are qualified (`supertonic:M1`, `pocket-tts:<profile-uuid>`,
`qwen3-gguf:<profile-uuid>`) and every plugin dropdown filters the exact provider prefix.
Switching providers resets voice overrides with a notice, preserving special-user
allow/block policy, speed, pitch and other settings. The tester follows the global
provider. All three speech paths (tester, declarative action and automatic chat) send
the selected provider. SonicBoom validates/strips qualified IDs server-side.

Pocket and Qwen need profiles owned by the current SonicBoom token. Pocket models are
separate by language: select the profile's resident model on the server and use its
language in plugin settings. Qwen uses the existing validated ICL mode. A provider
with no usable authorized profiles shows an empty state and cannot send empty speech.

Old settings without `providerId` use Supertonic. Raw saved Supertonic voices such as
`M1`/`F2` are qualified at selection time; the UI persists qualified IDs after catalog
reconciliation. Existing actions without a provider still use Supertonic. No policy
setting or secondary settings database is introduced. The generic declarative action
editor must use a matching provider/qualified voice; mismatches are rejected by the
server rather than routed elsewhere.

## Reload semantics

**Refresh voices** deduplicates clicks, force-refreshes both host option sources,
reconciles all voice dropdowns, then persists a fresh opaque `tts.voiceCatalogRevision`.
Loading/success/error are accessible and localized in English/Spanish. Transient
errors retain the same connection's last valid catalog; retry remains available.
Loading providers retain saved preferences until a ready catalog can validate them.

The backend compares revision **equality** in its cache key, along with server,
provider and an opaque hash of credential identity. The next queued speech job with
the new settings re-fetches immediately instead of waiting for the 300-second TTL.
Empty/error catalogs are cached too, avoiding a request per chat message; explicit
reload bypasses that cache. Foreign provider voices never enter voice-choice policy.

Host settings changes clear option caches; generation checks reject in-flight results
from old connection settings. The UI checks connection identity before committing
catalogs and dispatching speech. It also subscribes to the existing
`plugin.settings-changed` event to invalidate after external credential changes even
when credentials remain redacted. Its own settings-write echoes are consumed without
creating a reload loop. No tokens enter cache keys or logs in plaintext. Reload does
not reload models, reset audio, switch output devices or interrupt queued playback.

## Text processing (emote codes before speech)

Chat is normalised before synthesis so recognised TikTok emote codes are never
spoken. The pipeline order is unchanged except for one inserted step:

```
enabled → comment/trigger filter → user eligibility
        → text processing → points check → voice
```

Text processing removes known emote shortcodes, then (optionally) Unicode
emoji, then unwanted invisible characters, then collapses whitespace. It runs
after the trigger filters so command matching still sees the original comment,
and before the points check so an emote-only comment is dropped without
charging points or posting a synthesis request.

Only tokens in the verified catalog are removed. Arbitrary bracketed text —
`[debug]`, `[1,2,3]`, `[username]` — is spoken normally; the naive "strip
everything in brackets" approach is explicitly rejected. Zero-width joiners are
preserved, Unicode emoji removal is opt-in (default off), and Spanish,
Japanese, Chinese, Korean and Arabic text is untouched. An operator can switch
emotes from "remove" to "describe" and can choose to keep messages that would
otherwise be emptied.

The **Text Processing** card in the settings UI exposes these options and a
live preview. The preview calls the exact same `sanitizeTtsText` the backend
policy uses, so the preview cannot disagree with production.

Both implementations are pinned by shared fixtures in
`shared/tts/fixtures/sanitizer-cases.json`, replayed by
`shared/tts/sanitizer.test.ts` (TypeScript) and `backend/src/sanitizer.rs`
(Rust). The catalog token list is a hypothesis pending verification against
captured LIVE traffic — see [docs/EMOTES.md](../../docs/EMOTES.md).

## Validation (2026-10-16)

```bash
bun run typecheck:sonicboom-ui
bun run build:sonicboom-ui
bun run lint:plugins
bun run test:plugins
bun run typecheck
bun run build:web
bun run test:sonicboom-ui --output=/tmp/sonicboom-ui-results
cargo test -p sonicboom-backend
cargo test -p tiktools-core option_sources
bun run test:e2e --output=/tmp/tiktools-e2e-results
```

- Plugin unit suite: 37 passed.
- Plugin UI typecheck/build, plugin lint, app typecheck and host web build: passed.
- Isolated Chromium suite: 19 passed (compiled UI, fake native broker and real iframe
  host shim). Includes enabled/ready counts, provider switching, manual provider/profile
  payloads, profile deletion/reload, revision persistence, loading/error/retry, switching
  during refresh, zero voices, Spanish copy and redacted credential-change events.
- Backend: 16 unit tests plus one real-process HTTP observer integration passed. Cache
  tests verify revision/provider/auth invalidation before TTL and no plaintext token key.
- Host option sources: 17 passed, including literal disabled metadata and rejection of
  old-tenant in-flight cache writes. Manifest schema-v3 validation passes in backend tests.
- App-wide E2E: 29 passed, 17 failed: 13 existing screenshot comparisons and four widget
  editor tests. Representative feed screenshot and missing widget Heading failures also
  reproduce at unchanged `origin/remake` (`278ca29`); no assertions or baselines changed.
  The full unchanged baseline also has 29 passes and the identical 17 failures. See
  [machine-readable validation](validation/provider-selection-2026-10-16.json).

Use separate Playwright output directories if running suites concurrently; both default
to `test-results`, which can otherwise cause unrelated artifact-deletion failures.

Real-model audible playback, GPU execution and native OS WebView/hardware-device
verification: **NOT RUN** for this integration. Contract tests use stub audio; this
work makes no new speech-quality claims and changes no inference engines.

## Connect SonicBoom

1. Start SonicBoom and finish its initial setup.
2. In TikTools, open the SonicBoom connection page and select **Connect SonicBoom**.
3. If SonicBoom requires authorization, sign in with its administrator account. TikTools creates and saves the API credential; the administrator password is not stored. No token needs to be copied. Manual entry remains available under **Advanced settings**.

New settings use `http://127.0.0.1:17842`, which avoids ambiguity between IPv4 and IPv6 localhost on Windows. Existing addresses are preserved. If SonicBoom uses another address or port, edit the server URL.

Discovery uses `/api/info` and verifies access through the protected, model-independent provider catalog. Downloading/loading a model does not mean the connection failed. The page refreshes model progress while it is open and reports builds without audio playback separately.

A server already configured with SonicBoom's `local` or `none` authentication mode connects on this computer without a token. TikTools does not disable authentication or change SonicBoom configuration automatically. Remote connections retain the host's network permission, private-network and credential requirements; disabling authentication remotely does not bypass that policy.

Administration responses from current Vue-based servers, older HTML servers and API-only servers are supported. This uses SonicBoom's existing login and token APIs; it does not add a browser-approval pairing endpoint to SonicBoom.

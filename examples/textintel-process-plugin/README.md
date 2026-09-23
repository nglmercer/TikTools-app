# TextIntel process plugin (reference event processor)

A reference pre-filter event processor: it enriches `tiktok.chat` events with
structured text analysis (language, normalization, composition, Unicode,
obfuscation, spam, rebus, spoken/TTS views, nickname phonetics) before
automation filters run. Raw event fields are never modified; derived data is
returned as constrained annotations the host merges under `event.intel`.

This plugin is the first implementation of the generic processor extension
point, not a special case: any plugin can declare `processorTypes`, require
the `events.enrich` capability, and implement `Plugin::enrich`.

## Layout

- `plugin.json` — manifest with `processorTypes` and a host-rendered settings
  schema (the host's `SchemaForm` renders it; no plugin UI ships here).
- `src/main.rs` — thin process adapter delegating to the library processor.
- `src/processor.rs` — engine orchestration: `TextIntelProcessor` owns the
  lazy engine plus generation-scoped analysis caches keyed by full input text.
- `src/mapping.rs` — output rendering: fingerprints become annotations and
  the canonical text views (spoken selection, skip policies, nested
  pronunciation evidence).
- `src/moderation.rs` — chat moderation: configured-term matching over the
  fingerprint views plus the plugin-owned verdict automations filter on.
- `src/settings.rs` — lenient serde settings with per-field fallbacks.
- `src/cache.rs` — the generic FIFO-bounded cache.
- `benches/processor_latency.rs` — dependency-free latency harness driving
  the same processor path the host invokes.

## Engine

`textintel` is pinned to a reviewed upstream revision with the
`production-local-lite` features: fully offline, no network providers. Engine
modes:

- `production-local-lite` (default) — the graceful local preset
  (`TextIntelligence::production_local()`). Lite is the default because the
  latency bench shows `default` mode hitting ~870ms on long repetitive input
  while lite stays near 10ms cold; both stay under ~25ms cold on typical chat.
- `default` — `TextIntelligence::default()`.
- `production-local` — same graceful preset call. This build does not compile
  the heavy transformer/espeak/ANN features, so unavailable pieces degrade
  and are reported by diagnostics instead of failing.

## Settings delivery

The host loads `settings.json` and attaches it to every
`EventEnrichmentRequest.settings`, so settings apply without file access from
any runtime and take effect on the next event. Unknown settings are ignored;
malformed values fall back per field.

## Chat moderation

Every analyzed comment carries a plugin-owned verdict (stable shape; the
host keeps it under the provider namespace, so the stable `IntelComment`
contract is untouched):

```json
{
  "blocked": true,
  "spam": false,
  "spamScore": 0.12,
  "badWords": true,
  "matches": [{"term": "badword", "view": "leet"}],
  "reasons": ["bad-word"]
}
```

Automations filter on `event.intel.providers.textintel.comment.moderation.blocked`
(`is-true`). Both filters are opt-in and default off, so existing
installations never start blocking messages on upgrade.

### Spam moderation

`spam` calculates evidence; `filterSpam` decides whether that evidence
blocks. `detected` in the stable spam evidence follows the same threshold.

```json
{
  "spam": true,
  "filterSpam": true,
  "spamThreshold": 0.75
}
```

### Bad words

```json
{
  "filterBadWords": true,
  "badWords": ["scam", "badword", "spam phrase"]
}
```

Matching runs over TextIntel's normalized and anti-obfuscation views, not
just the raw text: `BADWORD` matches through `casefold`, full-width text
through `nfkc`, `b4dw0rd` through `leet`, `baaaadword` through
`repetition_collapsed`, confusable spellings through the confusable
skeleton, combined obfuscation through the full `normalized` pipeline, and
decoded text through strong `rebus` candidates. Each term reports the first
(least transformed) view that reveals it.

Single-word terms require word boundaries, so `ass` never matches `class`;
phrases match as contiguous spans. The list is sanitized (trimmed,
deduplicated, capped at 500 terms of 128 characters) and never logged.

### TTS

```json
{
  "muteBlockedTts": true
}
```

Blocked chat emits an explicit non-speakable policy view instead of spoken
text:

```json
{
  "text": "",
  "source": "policy",
  "speak": false,
  "reason": "moderation-blocked"
}
```

The policy is emitted even with `ttsCandidate` disabled: omitting the view
would let TTS consumers fall back to the raw blocked comment. Set
`muteBlockedTts` to `false` to keep regular TTS for blocked messages.

### Points

The processor is intentionally side-effect free: it never touches viewer
points. Deductions are automation work over the existing executable action:

```text
trigger: tiktok.chat
filter:  event.intel.providers.textintel.comment.moderation.blocked is true
action:  core.points
  uniqueId: {{ event.user.uniqueId }}
  delta: -10
```

The penalty amount is user-configured per rule (any finite non-zero
number); the points service clamps totals at zero. Create the rule from the
web UI (Behavior → Events → Moderation penalty) or from the CLI:

```sh
tiktools automation moderation-penalty --points -10
```

Both store the `core.points` action plus the `tiktok.chat` event wired to
it. Blocked messages stay available as raw live events; `moderation.blocked`
is the decision signal for downstream consumers (TTS, automations, rewards).

## Concurrency note

Each plugin instance is served by a single host worker thread with a bounded
queue, and sibling processors share it: one slow call delays the rest. Keep
this process fast and never combine slow model preparation with enrichment
here; measure with the bench before raising `timeoutMs`.

## Build, test, bench

```sh
cargo test --manifest-path examples/textintel-process-plugin/Cargo.toml --locked
cargo bench -p tiktools-textintel-plugin --bench processor_latency
```

Install by packaging `plugin.json` with the `tiktools-textintel` binary as
the `entry` executable.

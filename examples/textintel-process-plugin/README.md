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

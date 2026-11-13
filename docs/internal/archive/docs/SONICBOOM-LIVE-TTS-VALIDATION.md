# SonicBoom LIVE TTS integration validation

Validated on 2026-10-17 against TikTools `remake` baseline `b0ee9c0` and SonicBoom `main` baseline `a0a4394`.

## Confirmed defects and changes

- Empty and failed voice catalogs shared the five-minute positive TTL. Valid catalogs retain that TTL; empty catalogs retry after two seconds, and failures back off from two to thirty seconds. Server/token/provider/catalog-revision changes invalidate the cache. Failed asynchronous speech invalidates its catalog as well.
- Automatic cloned speech held the only dispatcher until synchronous inference finished. Pocket and Qwen now use SonicBoom's authenticated jobs API with `profile=<uuid>`, scoped stable idempotency keys, bounded submission retries, bounded polling, and separate synthesis/playback outcomes. Supertonic retains its short synchronous path.
- Selected provider, language and voice could diverge between settings and automation defaults. Optional declarative `defaultsFromSettings` resolves omitted fields from the same host settings snapshot as authentication. Explicit action fields win. The SonicBoom manifest opts in; other integrations retain their behavior.
- An explicitly persisted English language could be replaced by the legacy default language. Explicit nested language now wins. Provider changes save immediately; periodic backend file reads handle unchanged/coarse modification timestamps. Unknown advanced settings survive UI saves.
- Jobs accepted an explicit provider unrelated to the owned profile. SonicBoom's real executor now checks the pairing before decoding reference audio, retaining existing ownership/model/revision/language/clone validation.
- Qwen's 180-second synchronous client budget exceeded the default server HTTP budget of 120 seconds. The synchronous playback route now has provider budgets of 15/60/180 seconds and a structured HTTP-deadline response. Explicit non-default server limits remain caps. Job HTTP requests and generation tracking have separate deadlines.
- Manual testing forced interruption. It now enqueues by default with an explicit interruption checkbox. Existing audio-manager progression, device recovery, and temporary-file ownership are preserved.

Provider-scoped selection and private catalog ownership checks already existed in both repositories and were preserved. The original symptom alone did not prove a Qwen inference defect; no model internals were changed.

## Bounded scheduling and observable outcomes

Advanced settings live under the existing `tts` settings object:

| Setting | Default | Bounds |
| --- | ---: | ---: |
| `emptyCatalogRetryMs` | 2,000 | 250–10,000 |
| `chatQueueCapacity` | 64 | 1–64 |
| `chatMaxWaitMs` | 15,000 | 1,000–30,000 |
| `chatMaxInflight` | 1 | 1–4 |
| `chatGenerationTimeoutMs` | 300,000 | 15,000–600,000 |

Pocket generation tracking is additionally capped at 120 seconds. Qwen's generation budget is independent of its synchronous budget. Admission freshness is checked before catalog lookup and dispatch; it is not extended to accommodate inference. The default single cloned job preserves chat ordering and prevents a server inference backlog. Messages arriving while local inference capacity is occupied are explicitly rejected, counted and logged rather than buffered for delayed playback. Increasing concurrency can reorder completion/playback and should only be done with matching server capacity.

SonicBoom remains authoritative for `max_concurrent`/`max_pending`; 429/503 retries are bounded. LIVE ingestion never waits for inference. Disconnect, settings changes and shutdown cancel tracking and retire queued messages. Message-ID dedup remains resident across reconnects.

Diagnostics use the existing host poll/plugin-event channel, emitting only changed snapshots. The WebView subscribes through PluginBroker; it adds neither privileged HTTP nor a polling loop. Counters and correlation IDs distinguish observation, filtering, expiration, duplicates, saturation, submission, generation, completion, playback admission, failure, timeout and cancellation. Error guidance never includes tokens, spoken text or reference audio.

## Automated checks

| Command | Result |
| --- | --- |
| `bun x vue-tsc --noEmit` | Pass |
| `bun x vue-tsc --noEmit -p plugins-example-tiktools/plugins/sonicboom/ui/tsconfig.json` | Pass |
| `bun scripts/task.ts frontend-lint` | Pass |
| `bun test plugins-example-tiktools/plugins/sonicboom/shared plugins-example-tiktools/plugins/sonicboom/backend plugins-example-tiktools/plugins/sonicboom/ui/src` | 101 pass |
| `bun scripts/task.ts e2e test --config plugins-example-tiktools/plugins/sonicboom/ui/playwright.config.ts` | 20 pass |
| `cargo test -p sonicboom-backend` | 54 unit tests and 3 process integration tests pass |
| `cargo clippy -p sonicboom-backend --all-targets -- -D warnings` | Pass |
| `cargo test -p tiktools-core option_sources` | 17 pass |
| `cargo test -p tiktools-core --features http declarative_http` | 26 pass; includes real HTTP action routing |
| `cargo test -p tiktools-plugin-api` | 72 pass |
| `bun x vite build --config plugins-example-tiktools/plugins/sonicboom/ui/vite.config.ts` | Pass |
| `bun scripts/build-sonicboom-backend.ts` | Pass; packaged executable staged |
| `bun scripts/task.ts e2e test` | 38 pass, 17 fail; all 17 reproduce on unchanged baseline |

Baseline E2E reproduction used a detached worktree at `b0ee9c0`, separate Vite port 3001, and the unchanged screenshot/widget suites: 2 pass, the same 17 fail. Screenshot assertions and widget tests were not changed. Core tests emit existing warnings outside the modified code.

SonicBoom checks use `--no-default-features --features server,playback,pocket-tts,qwen-gguf` (CPU Qwen, no GUI/GPU SDK requirement):

- `cargo test`: 201 library, 250 server, 5 cache-isolation and 1 framing tests pass; 10 opt-in model tests remain ignored by this suite.
- `cargo clippy ... --all-targets -- -D warnings`: pass.
- `cargo build ... --bin SonicBoom`: pass.

The reduced feature run without `qwen-gguf` initially exposed an existing OpenAI-model assertion requiring Qwen. The full CPU-provider feature set passes. An initial placement of profile validation also broke an injected mock executor; validation was moved into the real executor, and that test passes unchanged.

## HTTP integration coverage

The real plugin executable receives framed LIVE events and performs actual TCP HTTP requests against a local SonicBoom-contract mock. Tests cover Supertonic synchronous playback; Pocket/Qwen `profile` routing, token forwarding, synthesis status and playback queue IDs; and 100 messages during active Qwen generation. Disconnect, settings-change and EOF/shutdown cases cancel the server job and reconcile every observed message to a terminal counter. Replay bursts do not create duplicate synthesis requests.

Transport tests cover empty catalogs becoming populated, catalog identity invalidation, invalid provider pairing, 408/429/503 and malformed accepted-response recovery with the same idempotency key, temporary polling errors, playback failure, deadline cancellation and cancellation requested before tracking begins. Existing policy, replay, expiration, 1,000-message overflow and server-timeout tests remain intact. The host action integration independently verifies inherited provider/language/token over actual HTTP for all three providers.

## Real-model verification

An isolated temporary server on loopback port 17843 used existing local model bundles and a synthetic Supertonic-generated reference. It did not use private user recordings. An initial probe inadvertently started the previous executable while the new build was linking; those measurements are excluded from patched-path claims. Checks were repeated with the rebuilt headless executable.

Patched-server results (short synthetic test input, CPU providers):

| Provider | Manual synchronous API | Automatic actual plugin process |
| --- | ---: | ---: |
| Supertonic | Accepted, 0.62 s | Playback admitted, 0.75 s |
| Pocket TTS | Accepted, 3.39 s | Playback admitted, 1.75 s |
| Qwen3-GGUF | Accepted, 164.81 s | Playback admitted, 109.49 s |

All six requests returned playback admission/queue correlation. Automatic cloned speech used jobs, not the synchronous endpoint. A separate Supertonic immediate-playback probe returned success and queue status `is_playing=true`, `is_paused=false`. These are real inference and audio-device acknowledgements; no human listening confirmation or speech-quality score is claimed. Initial ALSA enumeration errors did not prevent the rebuilt server from initializing its default output.

## Limits and follow-up

- This change does not make CPU Qwen fast. Admission freshness and inference deadlines are distinct: an admitted synthesis can outlive the 15-second waiting window. Saturated arrivals have explicit rejections. Tune generation timeout for the desired freshness/coverage tradeoff.
- Default single-flight cloned speech preserves ordering. Opting into multiple jobs can reorder completion. SonicBoom still enforces its own inference limits.
- `queued` and `started` are the existing API's playback dispositions, not a finished/heard acknowledgement. Paused queues remain intentionally paused; the existing audio manager handles progression and device recovery. Long-term audible completion and hardware disappearance/recovery need dedicated device testing.
- Cancellation is bounded best effort during network loss. If every idempotent submission response is lost, the client reports an indeterminate transport/protocol outcome rather than risking duplicate synthesis with a new key. A server job may remain until its inference deadline in that case.
- The automated matrix is representative, not an exhaustive test of every Cartesian combination in the request. GPU builds, Windows, real TikTok connectivity/reconnection and a human listening test were not performed. Real LIVE transport is represented by the actual host event envelope across the binary boundary.
- The 17 unrelated E2E failures need separate screenshot/widget maintenance; none were hidden or weakened.

The temporary verification server was stopped after the checks. Private recordings and credentials are not committed. Integration source and rebuilt package files are committed independently from SonicBoom's compatibility fixes.

## Related commits

- TikTools host settings inheritance: `9f931d1`.
- SonicBoom job/profile and HTTP-budget compatibility: `0932cb8`.
- Plugin implementation and packaged assets: the commit containing this report.

## Files

TikTools changes: `plugins-example-tiktools/plugins/sonicboom/backend/src/{main,config,transport}.rs`, `plugins-example-tiktools/plugins/sonicboom/backend/tests/observer.rs`, `plugins-example-tiktools/plugins/sonicboom/plugin.json`, `plugins-example-tiktools/plugins/sonicboom/ui/src/App.vue`, both UI locale catalogs, UI fixtures/specs, `crates/tiktools-core/src/services/declarative_http/execution.rs`, `crates/tiktools-plugin-api/src/manifest/validation.rs`, this report, and rebuilt tracked UI/backend distribution files.

SonicBoom changes: `src/api/jobs.rs`, `src/server/manager.rs`, and `docs/tiktools-live-integration.md`.

## Follow-up: per-user providers and minimal controls

Special users may now persist an optional `providerId`. Empty/omitted means the general provider; an explicit provider scopes that user's voice catalog and speech transport independently. For example, use Supertonic generally and `qwen3-gguf` for one allowed viewer. Blocked users remain blocked. Explicit overrides survive general-provider changes and catalog reloads; legacy entries retain their inheritance behavior. Both shared TypeScript and backend Rust policy implement the same routing rule. Missing Qwen voices never fall back to Supertonic.

The backend caches the full authenticated catalog and selects a provider pool for each message, avoiding a catalog fetch whenever adjacent users choose different engines. Existing cache identity, negative retry and failure backoff behavior remains intact. Actual-process HTTP tests cover both general cloned providers and cloned special-user overrides with Supertonic selected generally.

Diagnostics and the manual interruption checkbox are now inside a collapsed Advanced section (English/Spanish). Unsupported per-user speed/pitch columns are hidden while persisted settings remain compatible.

Follow-up verification: 102 shared/plugin tests, 55 backend unit tests, 3 process integration tests and 21 UI tests pass; UI/root typechecks, plugin lint, backend clippy and packaged builds pass. Real inference was not repeated for this routing/UI extension. Mixed-provider parallel inference requires SonicBoom `inference.max_concurrent >= 2`; this extension does not bypass or silently increase server limits.

# LIVE event reliability

The PR builds on `remake` commit `4acf53f`, which intentionally reverted
`c5e7cf9`. Gift/like originals remain the public events; small bounded counters
provide optional aggregates without restoring the previous aggregation workers.

## Delivery, accounting, and recovery

Backend replay identity includes creator, room, method, type, and nonzero message
ID. It survives reconnects and room switches within 4,096 entries and a five-minute
TTL. A missing message ID does not establish a replay. Frontend delivery cursors
identify events already accepted into the authoritative core journal. Cursors
survive typed IPC and browser serialization. Legacy deliveries without a cursor
use the message identity cache.

Original automation events and UI events are published before SQLite or points
work. An ordered effects queue has 128 slots; admission does not block delivery.
Rejected, cancelled, or uncertain persistence outcomes are measured separately
and degrade `system.health`. The original connection context is captured before
asynchronous work so a room switch cannot reattribute analytics or points.
Consumers observe `points.changed` and `points.awarded`; the original event is
not asynchronously enriched after publication.

Combo identity requires creator, room, sender, gift, and a nonzero group ID.
Cumulative maxima produce quantity and diamond increments. A final packet does
not count the same quantity again, including final packets with a new message ID.
Unknown prices remain zero. Gifts without combo identity keep their independent
quantities. The optional typed `data.aggregation` contains gift increments or
like `{ previous, total }` transitions. A goal crosses at `previous < T <= total`.
Counter memory is bounded by five minutes, 4,096 combos, and 4,096 like scopes.
Capacity exhaustion preserves originals and reports untracked aggregates instead
of silently evicting a counter and retriggering a goal.

The feed detects cursor gaps during realtime delivery and on resume, recovers
ordered pages, and deduplicates overlap. Desktop, browser, and CLI use the same
512-delivery memory journal. Recovery reads presentation data; it does not
reexecute points, workflows, plugins, TTS, or OBS alerts. Rejected effects cannot
be recovered by replaying the feed. Retention and TTL limits are explicit.

OBS visual combos include creator, room, sender, and gift. A missing/zero group
ID never groups packets. Visual state is bounded and uses the core replay TTL;
final packets with no new increment do not reopen a completed alert.

## Upstream decoder migration

All signer dependencies now declare upstream revision
`9b4f344d105c62e23bf3a883c7a35066e001e078`. The local `ttl-live-events`
patch and its checked-in sources are removed. The revision check rejects a local
event-decoder patch. `ttl-live-proto` retains the existing, separate Termux
compiler-selection patch: it uses native Bionic `protoc` on an Android build host.
Its schemas and runtime code match upstream. This is a build compatibility
exception, not an event decoding or connection fix.

A reported real session showed 64 gifts as Unknown despite no ingress loss.
Prost accepted the packet, but the old diagnostic traversal truncated deep
metadata and prevented canonical normalization. Upstream now gives normalization
and the diagnostic schema independent budgets. The app requests Normalized mode;
the regression also checks Full mode can normalize a deep gift while reporting
`schema.truncated`. Invalid neighbors remain isolated and envelope identity is
preserved. The sanitized fixture replaces personal values and removes opaque
fields; it is not an original session capture.

The upstream audit sampled 100 real rooms, finding 35 inline emotes, and exercised
local concurrency and lifecycle stress. These are upstream results, not evidence
that this application connected to 100 rooms. Standalone emote wire behavior and
undocumented fields retain their documented uncertainty. Unsupported methods
remain Unknown rather than receiving invented application semantics.

## Connection retry policy

The application owns one retry layer. The SDK owns signing, framing, heartbeats,
ACKs, and socket cleanup. Each socket attempt obtains a fresh signature; it does
not reuse an expired URL. Initial discovery/handshake failures and dropped sockets
use the same classified policy:

- HTTP 429 and narrowly recognized rate-limit refusals use a 30/60-second fallback
  with up to 7.5 seconds of positive jitter, at most two retries by default.
- Where the application receives `Retry-After`, delta seconds and HTTP dates are
  parsed. The wait is at least the server delay and the fallback. Header delays
  are bounded to one year for deadline representability.
- The pinned SDK exposes HTTP status but drops response headers for discovery and
  WebSocket handshake failures. Those paths use the conservative fallback. The
  guest-bootstrap and bundle-download paths retain and honor their response headers.
- Temporary network failures and HTTP 5xx use the configured exponential policy
  (default five retries, two-second initial delay, 30-second ceiling) with jitter.
- HTTP 401/403, blocked HTTP-200 handshakes, offline/invalid rooms, malformed
  responses, and signing failures terminate without automatic retries.

Cooldown belongs to the client, survives cancellation and replacement, and is
shared with browsing and monitoring. Repeated Connect clicks cannot immediately
bypass a rate-limit wait. Optional metadata remains best-effort; a 429 still
establishes cooldown before another request. Disconnect interrupts retry waits.
Network deadlines apply per operation and do not accidentally expire a deliberate
cooldown. `live.reconnecting` reports the actual wait and attempt; recovery reports
Connected. The UI keeps a terminal error when Disconnected follows it and clears
that error on recovery. No cookies or participant payloads are logged by retries.

The feed Event Debugger reads `system.health` on demand, including ingress,
publication, queue, drop, replay, presentation, and recovery counters. The latest
12 identities/cursors/correlations exclude chat bodies and credentials. CLI:
`tiktools rpc system.health '{}'`.

## Validation and limits

Run:

```sh
bun run deps:check
bun run contracts:check
bun run check:web
bun run test:rust
bun run lint:rust
bun run fmt:rust
```

The application smoke probe also connected through the upgraded native client
for 30 seconds: 171 events (45 chats, two gifts, 116 other supported events, and
eight Unknown events), zero subscriber lag, and zero reported errors. Unknown
events remain visible as evidence; this is not a claim of complete schema coverage.
The 429 regression uses a real local HTTP handshake and virtual time; it does not
attempt to trigger a live-service rate limit.

```sh
TIKTOOLS_TIKTOK_SIGNING_BUNDLE=/path/webmssdk.js \
TIKTOOLS_TIKTOK_SESSION_FILE=/path/private-session \
TIKTOOLS_LIVE_PROBE_SECONDS=30 \
  cargo run -p tiktools-tiktok --example live_decode_probe --locked
# Optional positional creator and room ID bypass random room selection.
```

This update passed the locked Rust workspace suite (736 tests, zero failures,
one existing ignored test), a final focused boundary run (25 tests), Clippy with
all targets/features and warnings denied, formatting, dependency and contract
checks, 291 Bun tests, 479 Vitest tests, web lint/type checking/build, and the
desktop debug build. The focused run includes the cancellation-during-wait test
added after the broad workspace run.

Retry tests use virtual time to assert the initial 429 path, bounded exhaustion,
server delays, cancellation, retained cooldown, and terminal classification.
Deep gift tests run against the upstream dependency, not a local decoder copy.

Historical PR validation before this dependency update: 803 Rust tests passed,
one existing ignored test; 291 Bun and 477 Vitest tests passed; widget suites
passed 15 plus 13 tests. Five full E2E failures were reproduced on clean
`4acf53f`: two expect an absent `Test connection` button and three use outdated
Connections snapshots. Those unrelated tests/references were not rewritten to
hide failures. The two feed snapshots changed only for the Event Debugger.

Live-service quotas vary by account, network, region, and endpoint. The fallback
wait is an application policy, not a measured TikTok quota. Recovery does not
promise every future schema or traffic pattern. Native Termux behavior needs an
Android device check after dependency changes; a Linux build does not prove it.
An already running desktop process must restart to load the updated binary.

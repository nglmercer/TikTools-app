# LIVE event delivery and creator monitoring

The native SDK remains pinned to `tiktok-signer` commit
`9b4f344d105c62e23bf3a883c7a35066e001e078`. Its emote JSON uses camelCase
(`imageUrl`, `emoteType`, `packageId`); raw protobuf indices do not declare an
offset unit. The synthetic regression fixtures in `crates/tiktools-core/tests/fixtures/live`
were originally copied from revision `4f9d0b63799c3e23cf20be717779ece507b9eed4`
and remain synthetic. They are not captured LIVE traffic.

## Event paths and fixes

`ttl-live-events` decodes protobuf into canonical events; `TikToolsEvent` adds
message metadata and room gift catalog enrichment. AppCore's ordered ingress
worker fences generations and deduplicates before points, analytics, plugin and
automation side effects. `live.event` carries the existing automation contract;
`live.ui-event` independently carries the feed render contract. The gateway
and OBS controllers consume `live.event`, not the feed presentation projection.

The UI previously deduplicated by message ID without event type and suppressed
identical payload checksums for five seconds when IDs were absent. This could
hide gifts that had already reached automation. Both checks are replaced by
explicit type/creator/room/message identity. `"0"`, empty and absent IDs never
identify a message; there is no sender/text/diamond checksum. IDs stay strings.
Backend and feed retain at most 4096 identities for five minutes; ordinary
same-room reconnects retain this memory, while creator/room changes clear it.
The backend also bounds its FIFO after TTL reinsertion. OBS uses the same
identity builder, with its existing bounded recent-ID cache and fresh envelope
ID fallback. OBS intentionally discards historical events and does not replay
alerts after a transport gap; its recent-ID cache is capacity-based, not TTL-based.

The feed journal cursor is independent of message identity. A committed staging
batch is preserved before suspending presentation. Reactivation requests history
even when no first cursor was seen. Recovery pages from each page's last cursor,
not the journal's newest cursor; concurrent realtime delivery is deduplicated by
cursor, including messages with no upstream ID. The unfinished recovery floor
survives another suspension. Recovered journal events are sorted by cursor;
system notices retain their positions. Gaps, failed recovery and bounded recovery
truncation produce a visible discontinuity instead of fabricated events. Display
and staging limits remain bounded. New room-scoped payload fields prevent old
rooms' retained events from leaking into recovery for a new creator.

Gift artwork now prefers a validated event URL, then room catalog or observed
room metadata cached by stable gift ID (at most 4096 entries). URLs are never
guessed. `giftDetails.giftId`, `roomId`, `creatorId` and `sourceType` are optional
additions to the UI contract. Rows always retain sender, gift name, quantity and
time; failed/missing artwork shows a generic gift icon. Search includes the gift
name and ID independently of translated text. Diamond/point/message details
remain in the context inspector rather than the compact translated row.

## Emotes and image policy

The resolver preserves the original comment, including multilingual text,
Unicode modifiers and ZWJ sequences. It only replaces complete bracket tokens
when explicit shortcode metadata, a verified catalog entry or one unambiguous
UTF-8/code-point/UTF-16 interpretation supports the match. Unmatched metadata
appears afterwards in wire order, with each consumed occurrence excluded.
Opaque numeric IDs are never converted to invented shortcodes. Standalone emotes
remain visible even without text or artwork. The shared resolver and renderer
are used by both the feed and chat widget. SonicBoom's catalog version and token
set remain unchanged; unknown bracketed text is not stripped.

The signer retains validated HTTP and HTTPS artwork metadata. The browser emote
renderer deliberately continues to load **HTTPS only**, rejecting credentialed
URLs and privileged/data/file schemes. This avoids plaintext requests and mixed
content failures in secure widget hosts. HTTP metadata is preserved but shown as
its original token or a neutral accessible emote marker; no silent text loss or
automatic URL rewriting occurs. Unknown opaque emotes use a neutral marker when
the image is blocked or missing. Known failed URLs are globally cached (256);
the affected component updates reactively using bounded local state (64).
A genuinely new URL can render. Dimensions remain fixed and content is escaped
by Vue; neither the feed nor widget uses HTML injection for message content.

## Native LIVE Monitor

From Connections, enter a creator and select **Monitor and auto-connect**.
The host immediately checks the SDK's existing unsigned `room_lookup`; offline
(`NoRoom` or a non-live room status) is an observation, not an error. On a live
room it invokes the existing native connection and AppCore event pump. While
connected or reconnecting it defers discovery to the transport lifecycle. After
that connection ends it resumes creator lookup, including a new room ID.

Control methods (same authenticated JSON-RPC infrastructure, available headlessly):

- `live.monitor.start`: `{uniqueId, sessionCookie?, pollIntervalSeconds?}`.
- `live.monitor.status`: `{}`; returns the current state and diagnostics.
- `live.monitor.stop`: `{}`; cancels the supervisor and disconnects its session.

The `live.monitor` topic carries `{status}`. Status contains enabled, creator,
phase (`stopped`, `waiting`, `checking`, `connecting`, `live`, `error`), interval,
check/failure counts, last/next check timestamps, and last error. The default
interval is 30 seconds, valid range 10–3600; lookup has a 30-second deadline.
Errors back off exponentially to at least the configured interval and at most
max(interval, 300 seconds). There is one owned supervisor with cancellation and
join; manual connect/pick/disconnect and shutdown retire it before replacing the
native session. In-progress native initialization stays cancellable. UI route
changes, minimizing and closing the presentation do not own this supervisor.
Monitoring lasts for the host process; it is not automatically enabled on restart.
The session cookie is kept in memory and is never returned in status or persisted
as monitor configuration. Builds without `native-tiktok` return unavailable.

## Diagnostics and verification

Use existing transport health diagnostics (`liveIngress`, `liveDedup`,
`liveMonitor`) to distinguish decode/admission loss from UI publication.
Ingress adds `uiPublished`, `uiGifts`, `giftsWithoutArtwork`; existing lag,
stale-generation, queue and processing counters remain intact. Debug tracing
correlates decoded method/message identity with UI publication without logging
message text or cookies. Desktop projection statistics expose delivery sequence,
suspension, bus loss and journal cursors. Frontend diagnostics expose received,
duplicates, identities, recovery gaps/in-flight state, cursor, matching filtered
rows, rendered rows and known image failures. Rendering counts are not proof of
native delivery, and automation execution is not proof of feed delivery.

Tests cover protobuf-to-AppCore UI/automation contract parity, room and type
identity, absent IDs, streak updates, feed suspension and paginated realtime
backfill overlap, Unicode and opaque emotes, reactive fallback, native monitor
offline/online/end/restart and cancellation/backoff. Chromium tests cover the
actual Vue feed, monitor configuration, and chat/gift rendering over the real
Rust gateway. These fixtures do not establish real TikTok or OBS/CEF behavior.

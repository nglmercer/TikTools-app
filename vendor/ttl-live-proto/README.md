# ttl-live-proto

Prost bindings for the TikTok Webcast Protobuf **v3** schema — the same schema
the modern `TikTok-Live-Connector` consumes through the `tiktok-live-proto` npm
package.

The crate is deliberately thin. It generates the types and exposes one entry
point, [`decode_event_batch`]; normalising events into a stable API is
`ttl-live-events`' job, and transport stays in `ttl-live-ws`.

## Provenance

`proto/v3/` is copied **verbatim** from
[isaackogan/TikTok-Webcast-Protobuf](https://github.com/isaackogan/TikTok-Webcast-Protobuf),
path `src/slim/v3`, pinned to the commit recorded in [`UPSTREAM`](UPSTREAM).

Schemas are never fetched at build time. To move the pin:

```sh
scripts/update-tiktok-protos.sh <commit>
```

The Node package generates its TypeScript bindings directly from this same original tree with
`protoc-gen-es`. The two compatibility renames in `build.rs` are only needed for Rust identifier
collisions during Prost generation; they are not applied to the vendored files or to TypeScript
generation.

## Licensing — read before redistributing

The rest of this workspace is MIT. **These schemas are not.**

Upstream is licensed under a *modified* AGPL-3.0: the stock AGPL text plus
Section 7 additional permissions, reproduced in [`LICENSE.upstream`](LICENSE.upstream).
In summary, and this is a summary rather than legal advice:

- **§18** grants an explicit exception letting you integrate the schemas and
  generated bindings into downstream client libraries and applications —
  commercial or not — *without* placing those downstream projects under AGPL.
  Overlays, LIVE games and stream bots are named as permitted use cases.
- **§19** withdraws that exception when the code powers a **commercial,
  closed-source or hosted service offered to third parties over a network** —
  a WebSocket relay, a data-scraping API, or managed hosting. In that case the
  unmodified AGPL applies, including its network-use clause: the complete
  server-side source must be offered under AGPL.
- **§20** revokes the additional permissions automatically on violation.
- **§21** exempts two named parties (TikFinity/STV GmbH and Euler Stream Inc.)
  from §19.

This repository ships `ttl-sign-server` and a `Dockerfile`, so §19 is the clause
to watch. Running the server for yourself is not the trigger; **offering it to
third parties over a network as a closed-source service is.** If that is the
plan, take legal advice before shipping.

Who actually links these schemas:

```
ttl-sign-server → ttl-live-events → ttl-live-proto
```

So the **server binary contains AGPL-licensed schema material**. That is the
deliberate consequence of retiring the old snapshot and making v3 the single
schema source; it is exactly the case §19 speaks to. `ttl-sign-core` and
`ttl-live-ws` are the only crates that do not depend on this one.

Practical measures applied here:

- the schemas live in this crate alone, so the boundary is visible in the
  dependency graph rather than buried in a shared module;
- `publish = false` — this crate is not pushed to crates.io;
- upstream's `LICENSE` is vendored unmodified as `LICENSE.upstream` and the
  update script re-copies it every time, so the notice cannot drift or be lost.

## Codegen notes

Upstream flattens nested protobuf types into underscore-separated names, which
produces one collision under Prost's Rust naming rules:
`SubPinCardText_TextType` and `SubPinCard_Text_TextType` both become
`SubPinCardTextTextType`.

Rather than edit the vendored files, `build.rs` stages a copy into `OUT_DIR` and
applies a small, documented rename table on the way. Enum *names* never appear
on the wire and no tag number is touched, so this cannot affect decoding. If a
rename stops matching after an upstream update, the build fails loudly instead of
silently generating something different.

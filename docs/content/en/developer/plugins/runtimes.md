---
title: "Plugin runtimes and trust limits"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a249fa36fa53f3bfe7da57fdd25a445e593517946f80ac7e891b54d95cdc17d"
translationRevision: "6a249fa36fa53f3bfe7da57fdd25a445e593517946f80ac7e891b54d95cdc17d"
order: 260
---

# Plugin runtimes and trust limits {#plugin-runtimes-and-trust-limits}

## Goal and prerequisites {#goal-and-prerequisites}

Select an executable format whose dependencies and trust match the host. Read [plugin overview](index.md), validate the manifest against the actual build and inspect target-specific restrictions before loading code. Protocol compatibility, native ABI compatibility and permission grants are independent checks.

## Choose an implemented runtime {#choose-an-implemented-runtime}

| Runtime | Execution | Trust and portability |
| --- | --- | --- |
| `native` | `libloading` C ABI | Trusted in-process code; target-specific |
| `process` | Standalone executable, framed stdin/stdout | Crash isolation, ordinary user OS permissions |
| `napi-vm` | Ahead-of-time JS on QuickJS via `rquickjs` | Denied host effects by default; optional native addons change trust |
| `declarative` | Validated integration descriptors | No guest program; effects use host broker |
| `wasm` | Optional runtime slot | General engine not enabled in normal builds |

Native ABI v1 exchanges serialized bytes through a small C table; Rust containers, trait objects and futures do not cross it. Native libraries remain loaded until shutdown. Process framing uses a little-endian u32 length followed by JSON bytes, not newline-delimited control RPC. Keep diagnostic output away from the framed stdout channel.

## Understand QuickJS compatibility {#understand-quickjs-compatibility}

The manifest name `napi-vm` remains stable, but old claims about a separate pure-Rust interpreter are historical. Guests get the documented compatibility globals, not a complete Node environment. `napi_get_uv_event_loop` is unsupported. Native `.node` addons use a deliberately trusted, allowlisted and SHA-256-verified in-process path; they are not made safe by the JavaScript runtime name.

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

Use focused loader tests when a full native-addon fixture would require unavailable platform tools.

## Expected result and status {#expected-result-and-status}

Manifest parsing rejects unsupported protocol/ABI combinations before loading. Android permits portable QuickJS and valid Android/Bionic process entries, but rejects native dynamic libraries and `.node` addons. A desktop Linux/glibc artifact is not an Android process plugin.

## Recovery {#recovery}

Restart a private host after replacing a native library. If startup times out and becomes uncertain, the manager can retain the worker identity and block replacement; do not bypass that state by declaring shutdown successful. Event buffers are bounded and delivery may report lag; startup emission is not a durable queue. The [loader implementation](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src) owns the current engine and lifecycle behavior.

## Operational details {#operational-details}

Current loader limits are 512 MiB QuickJS memory, 2 MiB stack, per-call interruption budgets, 16 MiB protocol frames and up to 128 queued calls per instance. These are implementation limits, not an effects cancellation guarantee. A timed-out process is terminated/removed; a timed-out native call cannot be safely killed and its instance is retained. Native OS effects may already have happened. Inspect the original outcome and lifecycle before replacement/retry. Trusted `.node` addons run with host-process privileges, outside the JavaScript capability boundary.

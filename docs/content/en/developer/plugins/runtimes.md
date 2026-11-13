---
title: "Plugin runtimes and trust limits"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8c9d406371879e3d0f5bf813d43af66dd12e24795f8dab281c395d42ba47939e"
translationRevision: "8c9d406371879e3d0f5bf813d43af66dd12e24795f8dab281c395d42ba47939e"
order: 260
---

# Plugin runtimes and trust limits

## Goal and prerequisites

Select an executable format whose dependencies and trust match the host. Read [plugin overview](index.md), validate the manifest against the actual build and inspect target-specific restrictions before loading code. Protocol compatibility, native ABI compatibility and permission grants are independent checks.

## Choose an implemented runtime

| Runtime | Execution | Trust and portability |
| --- | --- | --- |
| `native` | `libloading` C ABI | Trusted in-process code; target-specific |
| `process` | Standalone executable, framed stdin/stdout | Crash isolation, ordinary user OS permissions |
| `napi-vm` | Ahead-of-time JS on QuickJS via `rquickjs` | Denied host effects by default; optional native addons change trust |
| `declarative` | Validated integration descriptors | No guest program; effects use host broker |
| `wasm` | Optional runtime slot | General engine not enabled in normal builds |

Native ABI v1 exchanges serialized bytes through a small C table; Rust containers, trait objects and futures do not cross it. Native libraries remain loaded until shutdown. Process framing uses a little-endian u32 length followed by JSON bytes, not newline-delimited control RPC. Keep diagnostic output away from the framed stdout channel.

## Understand QuickJS compatibility

The manifest name `napi-vm` remains stable, but old claims about a separate pure-Rust interpreter are historical. Guests get the documented compatibility globals, not a complete Node environment. `napi_get_uv_event_loop` is unsupported. Native `.node` addons use a deliberately trusted, allowlisted and SHA-256-verified in-process path; they are not made safe by the JavaScript runtime name.

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

Use focused loader tests when a full native-addon fixture would require unavailable platform tools.

## Expected result and status

Manifest parsing rejects unsupported protocol/ABI combinations before loading. Android permits portable QuickJS and valid Android/Bionic process entries, but rejects native dynamic libraries and `.node` addons. A desktop Linux/glibc artifact is not an Android process plugin.

## Recovery

Restart a private host after replacing a native library. If startup times out and becomes uncertain, the manager can retain the worker identity and block replacement; do not bypass that state by declaring shutdown successful. Event buffers are bounded and delivery may report lag; startup emission is not a durable queue. The [loader implementation](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src) owns the current engine and lifecycle behavior.

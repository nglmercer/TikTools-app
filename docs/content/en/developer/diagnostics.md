---
title: "Diagnostics and safe evidence"
docId: "developer.diagnostics"
translationKey: "developer.diagnostics"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "123301b5d60789ec9ef26ce98cb57270b2882860d3cdb13b52daa722d09c8794"
translationRevision: "123301b5d60789ec9ef26ce98cb57270b2882860d3cdb13b52daa722d09c8794"
order: 130
---

# Diagnostics and safe evidence

## Goal and prerequisites

Collect enough evidence to locate a failure without retaining account secrets or confusing diagnostics with recovery. Use a private test host and know which [transport](transports.md) is selected. The application installs one session collector; status components navigate to it rather than append entries while rendering.

## Inspect a failure

1. Open the contextual status action for the failing connection, plugin, sync pass or updater. Keyboard focus exposes the desktop log action as hover does.
2. Filter by source, severity and text, then inspect the selected entry's context and repetition count.
3. Copy selected or filtered entries when they explain the issue. Review exports before sharing them.
4. Compare native health with the frontend observation:

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system health
target/debug/tiktools --json system doctor
```

A disconnected LIVE probe may be skipped. A compiled capability does not prove runtime availability. The UI panel is nonmodal on desktop; narrow layouts use `/debug` with a return location. Native text export uses `system.saveFile`; browsers download locally.

## Understand retention

`src/web/shared/diagnostics/log.ts` bounds history to 2,000 entries and 2 MiB of serialized UTF-8. Consecutive matching source/entity events within five seconds combine with a repetition count. Redaction happens before retention, including recognized credentials and URL secret locations; arbitrary unlabelled third-party secrets cannot always be recognized. Do not add raw settings, cookies, RPC parameters or HTTP bodies to collectors.

## Expected result and status

The viewer provides bounded session history and context navigation, not a durable audit log. Reload/unmount clears memory. Clearing history does not clear a connection error or repair its cause. Cloud request IDs can help correlate a safe failure envelope; source coverage follows the installed collectors and does not guarantee capture of every registry or native error.

## Verify and recover

Run `bun scripts/task.ts test web diagnostics` and inspect the [collector source](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/diagnostics). Reproduce redaction, eviction and repeated-event behavior with synthetic values. For a conflict, unknown command or credential expiry, return to the owning recovery flow. Copying a diagnostic entry is evidence gathering; it never retries an action or approves a new trust policy.

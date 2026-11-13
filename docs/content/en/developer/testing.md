---
title: "Tests and evidence boundaries"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "be694b0c471f68219481d14ca77bf7842c6b7935e4c97b5dbe6cd2a8cfe1fc1d"
translationRevision: "be694b0c471f68219481d14ca77bf7842c6b7935e4c97b5dbe6cd2a8cfe1fc1d"
order: 210
---

# Tests and evidence boundaries

## Goal and prerequisites

Choose checks that can establish the behavior changed. Use Bun 1.4.4, a lockfile-clean install and Rust only for affected native boundaries. Keep tests isolated from production hosts, credentials and accounts. A successful browser fixture is useful evidence for its boundary, not for every integration beneath it.

## Choose a focused suite

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

The default fast suite combines Bun units and web/widget Vitest suites. The targeted runner validates arguments and rejects missing Rust test filters rather than accepting zero matching tests. Use the `rust` passthrough when you need Cargo target flags beyond the targeted runner's grammar.

## Verify the actual boundary

Vue component tests use the shared fake control client. Main Playwright tests exercise the same SPA against a fake `window.ipc` host installed before navigation. Unknown RPCs fail loudly and console/page errors are checked. They run Chromium, not Wry. Plugin UI browser suites live with the external plugin repository. Widget gateway suites test the shared wire/client and built assets.

For authorized browser work after building assets, use `bun scripts/task.ts test e2e`. This launches a browser, so it is intentionally separate from documentation editing and headless source review. Native-shell, OBS Browser Source, phone, OS credential-store, updater replacement and two-PC tests need their real environments and explicit evidence.

## Expected result and status

A change report names the passing suite and its scope. Run frontend lint/type/build checks and required CI gates for implementation changes; docs content uses `bun scripts/task.ts docs check` and `docs test`. Do not change dependencies merely to make a test command download a newer tool.

## Recovery

If a filter matches nothing, fix the filter instead of reporting success. If a fixture misses a newly required RPC, extend its explicit contract rather than making unknown calls succeed. Keep uncertain/skipped manual acceptance visible. The [test runner](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts) defines current command syntax; [performance](performance.md) explains reproducible load evidence.

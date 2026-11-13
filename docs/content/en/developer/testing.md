---
title: "Tests and evidence boundaries"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
translationRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
order: 210
---

# Tests and evidence boundaries {#tests-and-evidence-boundaries}

## Goal and prerequisites {#goal-and-prerequisites}

Choose checks that can establish the behavior changed. Use Bun 1.4.4, a lockfile-clean install and Rust only for affected native boundaries. Keep tests isolated from production hosts, credentials and accounts. A successful browser fixture is useful evidence for its boundary, not for every integration beneath it.

## Choose a focused suite {#choose-a-focused-suite}

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

The default fast suite combines Bun units and web/widget Vitest suites. The targeted runner validates arguments and rejects missing Rust test filters rather than accepting zero matching tests. Use the `rust` passthrough when you need Cargo target flags beyond the targeted runner's grammar.

## Verify the actual boundary {#verify-the-actual-boundary}

Vue component tests use the shared fake control client. Main Playwright tests exercise the same SPA against a fake `window.ipc` host installed before navigation. Unknown RPCs fail loudly and console/page errors are checked. They run Chromium, not Wry. Plugin UI browser suites live with the external plugin repository. Widget gateway suites test the shared wire/client and built assets.

For authorized browser work after building assets, use `bun scripts/task.ts test e2e`. This launches a browser, so it is intentionally separate from documentation editing and headless source review. Native-shell, OBS Browser Source, phone, OS credential-store, updater replacement and two-PC tests need their real environments and explicit evidence.

## Execution time and CI cost {#execution-time-and-ci-cost}

Start with a file/name filter or `bun scripts/task.ts test --changed` locally. A full frontend run keeps all files isolated; Vitest uses `vmThreads` to reuse the DOM runtime per worker rather than sharing application state across files.

CI compares the complete push or pull-request range. Reviewed documentation paths omit native jobs; Vue/demo changes run the frontend gate. Native sources, build infrastructure, dependency manifests, unknown paths or an unavailable comparison retain every existing gate. `scripts/ci-scope.mjs` is the tested selection policy. Manual CI offers `frontend`, `native` or `full`; reusable callers default to full validation. The final CI status rejects failed or cancelled selected jobs.

Rust caches are separated by role, OS, toolchain, lockfile and source revision. CI retains line-table debug information and disables incremental sessions to reduce cache volume. Serial SQLite/profile tests remain serial until their shared resources are independently isolated. Do not remove that guard to obtain a faster timing.

The site workflow is manual, so editing or pushing documentation does not start a second remote build. Release recovery is also manual: its default checks only Linux and does not publish. Both platform gates and an explicit publish choice are required for release publication. Never dispatch or repeat paid-runner work merely to confirm a Markdown edit.

## Expected result and status {#expected-result-and-status}

A change report names the passing suite and its scope. Run frontend lint/type/build checks and required CI gates for implementation changes; docs content uses `bun scripts/task.ts docs check` and `docs test`. Do not change dependencies merely to make a test command download a newer tool.

## Recovery {#recovery}

If a filter matches nothing, fix the filter instead of reporting success. If a fixture misses a newly required RPC, extend its explicit contract rather than making unknown calls succeed. Keep uncertain/skipped manual acceptance visible. The [test runner](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts) defines current command syntax; [performance](performance.md) explains reproducible load evidence.

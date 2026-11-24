# Tests

All black-box tests live here. Tests that need private internals stay next to
the code they exercise (Rust `#[cfg(test)]` modules inside `crates/*/src/`,
`tiktools-cli` binary modules, and feature-gated SDK internals).

## Layout

```text
tests/
  rust/<crate>/<module>.rs   # Cargo [[test]] targets (path="../../tests/rust/...")
  unit/web/...               # Vitest (happy-dom), migrated from src/web
  unit/widgets/...           # Vitest *.vitest-case.ts (vitest.widgets.config.ts)
  unit/demo|automation|plugin-ui|shared|pairing/...  # Vitest, converted from bun:test
  bun/scripts|packages|widgets/  # `bun test` suites needing Bun-only APIs
  site/                      # Playwright site specs (playwright.site.config.ts)
  e2e/                       # Playwright e2e specs (playwright.config.ts)
  fixtures/ helpers/         # shared spec helpers
```

Decision rule for Rust: a test module moves to `tests/rust/<crate>/` only when
it uses the crate's public API. Anything touching private fields/functions (or
`pub(crate)` helpers) stays in `src/`. `tiktools-cli` is a binary-only crate,
so its in-src tests cannot move at all.

## Running

```sh
bun run test                  # fast gate: bun units + Vitest + widgets Vitest
bun run test <area> [filter]  # targeted: web|bun|rust|e2e ...
bun run test rust <crate> [filter] [--features <list>]
bun scripts/task.ts rust-test-sharded   # full Rust suite (cargo-nextest when installed)
bun run lint                  # eslint over src + tests/unit + tests/bun
bun scripts/task.ts docs check
TIKTOOLS_SITE_PORT=4176 bun x --no-install playwright test --config playwright.site.config.ts
```

Rust tests run under `cargo-nextest` when it is on PATH: every test gets its
own process, so PID-scoped `TIKTOOLS_HOME` fixtures isolate themselves and the
scheduler work-steals across `TIKTOOLS_RUST_TEST_POOL` workers (default 4).
Without nextest the script falls back to the legacy shard runner
(`TIKTOOLS_RUST_TEST_SHARDS=0` forces it); both keep doc tests serial. Unit
tests that leave `TIKTOOLS_HOME` unset resolve to a per-process temp dir
(`cfg(test)` fallback in `tiktools-core`'s `paths.rs`), never the real data
root.

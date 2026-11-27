# Agent notes

## Git

- Never add `Co-Authored-By` (or any co-author trailer) to commits in this repository.

## Resource constraints

- This machine has **2 CPUs and 14 GiB RAM**. Never run heavy verification
  steps (frontend-lint, cargo test, vitest, contracts) in parallel background
  shells — run them strictly one at a time. Parallel runs already triggered
  systemd-oomd kills that took down the whole IDE.
- If the user runs OBS/Discord or other heavy apps, prefer the lightest
  verification that covers the change instead of the full matrix.

## Verification

- Rust: `cargo test -p tiktools-core`, `cargo test -p tiktools-plugin-api`, `cargo clippy`
- Web: `bun scripts/task.ts frontend-lint` (oxlint-vize + lint-custom + vue-tsc), `bun scripts/task.ts test web <filter>`
- Contracts: `bun run check`

## Experimental Vize typecheck (upstream collaboration)

- `bun scripts/task.ts frontend-typecheck:vize` typechecks `.vue` files through
  [Vize](https://github.com/ubugeeei-prod/vize)'s content mapper on TypeScript
  7.1 beta (`tsconfig.vize.json`, aliased package `typescript-vize` — the main
  `typescript` dependency stays on the line `vue-tsc` needs).
- **The Vize typecheck is the gate** inside `frontend-lint` (inverted after
  measuring zero divergences against vue-tsc across the whole codebase).
  `bun scripts/task.ts frontend-typecheck:full` keeps the vue-tsc run
  available — run it periodically (weekly, before releases) and any
  divergence is an upstream report, never a local silence.
- Measured on this machine: ~5 s vs ~37 s for `vue-tsc` (7x+).
- **Reporting policy — this is a collaboration with upstream, not just a tool:**
  - Every failure or divergence from `vue-tsc` must be reported: minimize a
    repro, then open an issue at https://github.com/ubugeeei-prod/vize/issues
    (or microsoft/typescript-go when the divergence is TS6-vs-TS7 behavior,
    visible because it also reproduces without any `.vue` file involved).
  - After opening the issue, evaluate whether it warrants a PR: if the fix is
    small and clearly understood, implement it upstream and link the PR from
    the issue; if not, say so in the issue and move on.
  - Never silence a divergence locally (no `// @ts-ignore`, no config
    exclusions) without linking the upstream issue in a comment.
- Version policy: `vize` and `typescript-vize` are pinned exact. Bump them
  deliberately every couple of weeks as part of the collaboration — never
  float. Before adopting a bump, run both typecheckers and diff the output.

## Linting: oxlint + vize (eslint retired)

- Linting runs through `oxlint-vize` (oxlint 1.x + oxlint-plugin-vize native Vue
  template rules) configured in `.oxlintrc.json`; eslint, typescript-eslint,
  eslint-plugin-vue and vue-eslint-parser were removed entirely.
- `scripts/lint-custom.ts` carries the two project-specific enforcements
  oxlint cannot express (no `<details>`/`<summary>` in templates — use
  BaseDisclosure; no hardcoded locale ternaries in tests — use testMessage).
  Retire it if upstream gains equivalent rule support.
- `src` is linted in its own invocation because the vize bridge fails on
  HTML-bearing trees combined with other targets — reported upstream as
  https://github.com/ubugeeei-prod/vize/issues/13652. Revisit once fixed.
- Same collaboration policy as the typecheck: every oxlint/vize divergence or
  false positive gets an upstream issue, then an evaluation of whether it
  warrants a PR. Never silence one locally without linking the issue.
- Version matrix is pinned exact: oxlint + oxlint-plugin-vize + vize. The
  bridge validates specific oxlint versions — bump them together, never
  independently, and rerun the full lint before adopting.

## Intro video (demo showcase)

- The landing tour is the demo's own `?showcase=1` mode (`src/web/demo/showcase`), so the video is a
  recording of the real UI. After `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build`, run
  `bun scripts/task.ts site record-intro` (needs `bunx playwright install chromium --only-shell` and
  `ffmpeg`). It writes `dist/intro-video/{intro.mp4,intro.webp,poster.png}`; copy `intro.webp` to
  `assets/` for the README and attach `intro.mp4` to the release.
- View Transitions are off in framed demos: `startViewTransition` inside the scaled docs iframe
  crashed headless Chromium. Framed views use a CSS fade-in instead.

## Site e2e speed

- `playwright.site.config.ts` runs 2 workers (one per CPU), reduced motion, and no tracing or
  failure screenshots. A full run of 200 tests takes about 2 minutes; it was 3.5 minutes before.
- Debugging: `TIKTOOLS_E2E_DEBUG=1` turns tracing, failure screenshots and the `/tmp` review
  artifacts (`qaScreenshot`, `qaWrite` in `tests/site/support/qa.ts`) back on.
  `TIKTOOLS_E2E_WORKERS=n` overrides the worker count.
- A spec that tests motion opts back in (`test.use({ reducedMotion: 'no-preference' })` or
  `page.emulateMedia`). Tests that only need the tour to progress add `pace=4` to the showcase URL.
- Prefer one generated test per case (`for (const id of ids) test(...)`) over a loop inside a test, so
  the two workers share the work and a failure names its case.
- Frame commands (`showcase-goto`, ...) are ignored until the demo reports ready: wait for a rendered
  event (`expectFirstChatMessage`) before sending one.


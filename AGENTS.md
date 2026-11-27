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
- While iterating, `bunx playwright test -c playwright.site.config.ts --only-changed[=<ref>]` runs
  only the spec files whose import graph touches the changed files (e.g. a tour definition selects
  `tours.spec.ts`). It cannot see code the browser loads from the built site (demo, landing), so run the
  full suite before committing a frontend change.

## App e2e (`tests/e2e`, fake host)

- The fake host defines `window.ipc`, so the app renders in native-chrome mode: the in-page menu bar is
  hidden and menu actions arrive as pushes. Drive them with
  `emitPush(page, { type: 'shell-menu-action', id: 'locale:es' | 'toggle-sidebar' | ... })`.
- Edit `INIT_SCRIPT` in `fixtures/tiktools-host.ts` carefully: it is a template string run in the page,
  so a stray brace breaks every spec at once (the symptom is a 30 s `waitForFrontend` timeout). Check it
  with `node --check` on the extracted script.
- Run the suite with `bunx playwright test` (about 2.5 minutes, one worker, shared dev server).

## Shared test tokens

- `tests/shared/` holds the identifiers tests repeat: `RPC` methods, `TOPIC`s, `EVENT_KIND`s, `PLUGIN_ID`s,
  `ACTION_TYPE`s, `ERROR_CODE`s, `ACCOUNT_STATE`s, `FIXTURE_ID`s, `ROUTE`s and `STORAGE_KEY`s. Import them
  (`import { RPC } from '.../tests/shared/index.ts'`) instead of retyping the string. Keep the module
  dependency-free: Playwright loads it in Node.
- `bun scripts/lint-custom.ts` (part of `frontend-lint`) fails a unit test that retypes one of these values.
  Add a new token to its group in `tests/shared` first; `tests/unit/shared/test-tokens.test.ts` pins values
  that mirror production (storage keys) and rejects duplicates.
- E2E specs share `tests/e2e/support/selectors.ts` (`SEL`, `PLUGIN_TITLE`) and `timeouts.ts` (`WAIT_MS`).
  Code that runs inside the page (`page.evaluate`, `addInitScript`, the fake host `INIT_SCRIPT`) cannot see
  imports, so literals stay there.
- Typing: the tokens are `as const`. When a test needs a wider type (a mutable id, `.includes(method)` on a
  `string`), widen at the use site (`typeId: string = ...`, `[...] as string[]`), not in the token.
- Named pauses and limits: `PAUSE_MS`, `TEST_BUDGET_MS` (`tests/e2e/support/timeouts.ts`) and `LIMIT`
  (`tests/shared/limits.ts`, pinned to production by the token test). A real-time `waitForTimeout` needs a
  named entry that says what it waits for.
- `bun scripts/test-literals.ts [--dir=tests/unit] [--threshold=5] [--fail]` lists what is still repeated. i18n keys,
  DOM attribute names and the fixture data it names are ignored on purpose.


## Readable tests

- `scripts/lint-custom.ts` (part of `frontend-lint`) fails a test with a line over 160 characters or two
  statements on one line. There is no formatter for tests, so write them already broken up: one
  statement per line, one argument or property per line when a call does not fit.
- A test longer than about 40 lines should show its phases: `await test.step('what happens', ...)` in
  Playwright specs, `// Arrange` / `// Act` / `// Assert` (or a plain description of the scenario step) in
  unit tests. Say why the step matters, not what the code literally does. Prefer splitting into separate
  tests when the phases do not share state.
- Titles say the behavior and the condition ("empties the feed when Clear feed is pressed"), not
  "works" or "is idempotent".
- Declare helpers such as `const t = ...` after the imports.
- Tests and test fixtures (`tests/**`, `src/**/testing/**`) contain no type assertions and no non-null `!`: oxlint's
  `consistent-type-assertions` (style `never`) fails `frontend-lint`; only `as const` is allowed. Instead of
  `value as Foo`:
  - annotate (`const data: Foo = JSON.parse(text)`) or use `satisfies`;
  - build complete objects with the builders in `tests/unit/support` (`liveEvent`, `liveAction`, `actionType`,
    `pluginStatus`, `automationEdit`, ...) rather than a partial plus a cast;
  - narrow with a runtime guard: `expectElement` / `toElement` (`tests/unit/support/dom.ts`),
    `parseRecord` / `isRecord` / `propertyOf` (`json.ts`), or VTU generics (`wrapper.get<HTMLInputElement>('input')`);
  - make fakes real: a class that `implements` the interface, `vi.stubGlobal`, `new DOMRect(...)`,
    `createFakeControlClient`; call erased handlers with `Reflect.apply`, and use `Reflect.get/set` for page
    globals inside `page.evaluate`;
  - fix the fixture type once (a generic `onTopic<T>`, `emit(topic, data: unknown)`) instead of every call site;
  - replace `value!` with `defined(value, 'what it is')` from `tests/shared` (throws a readable error when missing);
    inside `page.evaluate` use a guard that throws, because page code cannot import.

# Playwright E2E

Browser-level end-to-end verification in Chromium. Playwright never
drives the Wry shell: native behavior stays covered by Rust tests. Two
suites cover the two frontends:

- **Main suite** (`tests/e2e`, `playwright.config.ts`): the generic
  TikTools SPA against a fake host at the exact `window.ipc` boundary.
- **Plugin UI suite** (`plugins/sonicboom/ui/tests`,
  `plugins/sonicboom/ui/playwright.config.ts`): the compiled plugin UI
  against a fake native broker, plus a real host/plugin interop spec.

Testing layers:

```text
Bun unit tests (src/web, src/automation, src/plugin-ui)
  + Rust unit/integration tests (cargo)
  + Playwright main E2E (tests/e2e)
  + Playwright plugin UI E2E + interop (plugins/sonicboom/ui/tests)
  + Playwright screenshot regression (tests/e2e/screenshots.spec.ts)
```

## Run

```bash
bun run test:e2e            # headless Chromium main suite
bun run test:e2e:ui         # interactive UI mode (development/debugging)
bun run test:e2e:headed     # headed Chromium
bun run test:e2e:update     # regenerate screenshot baselines (see below)

bun run build:sonicboom-ui  # build the plugin UI first (once per change)
bun run test:sonicboom-ui   # headless Chromium plugin UI suite
```

The first run installs nothing extra: `@playwright/test` is a
devDependency. The main config starts the app with `bun run serve:web`
automatically (`reuseExistingServer: true`); the UI config serves the
compiled `ui/dist/` with `vite preview` (with
`Access-Control-Allow-Origin`, which opaque-origin frames require).

## Fake host (main suite)

`tests/e2e/fixtures/tiktools-host.ts` installs `window.ipc` via
`page.addInitScript()` before `page.goto()`. Responses flow back through
`window.__webview_on_message__`, exactly like the native bridge. Behavior
is data-driven through the mutable `window.__TIKTOOLS_E2E_STATE__` object,
so specs can change host responses mid-test with `page.evaluate()`.
Legacy `type:` page messages (`plugin-ui-open`/`plugin-ui-close`) are
recorded in `state.legacyMessages` for assertions.

- Implemented RPCs cover the boot path and tested screens
  (`automation.snapshot/runs`, `app.state.*`, `plugins.settings.*`,
  `plugins.options/health/action.execute/token.provision`,
  `processors.status`, `gifts.list`, `points.*`, `creators.*`, `live.status`).
- `state.failures[method]` forces a method to fail (error-state tests).
- `state.actionDelayMs` delays action execution (in-flight-state tests).
- **Unexpected RPC methods fail tests loudly**: the dispatcher records
  them in `state.unhandled` and answers with an error; every spec calls
  `assertNoUnhandledCalls(page)` so new frontend dependencies cannot hide
  behind silent mocks.
- Every spec captures `console` errors and `pageerror`s and fails when
  any appear (`consoleCapture.assertClean()`), and traces are retained on
  failures (`trace: 'retain-on-failure'`).

Fixtures: `tests/e2e/fixtures/states.ts` (deterministic app states:
empty, connected creator, SonicBoom connected/disconnected, legacy
TTS-status/list/form/connection pages, webview launcher page,
action/option failures, locales/themes) and
`tests/e2e/fixtures/plugins.ts` (descriptors, settings, voice/output
options, typed webview UI descriptor).

## Specs (main suite)

| file | coverage |
|------|----------|
| `navigation.spec.ts` | boot, named nav, dynamic plugin tab, removal fallback |
| `plugin-pages.spec.ts` | declarative form save, dynamic list + refresh, text page, option-source error |
| `plugin-webview.spec.ts` | legacy TTS section degrades to a note; launcher posts open/close host messages |
| `connections.spec.ts` | empty state, probe success/failure, settings stay editable |
| `accessibility.spec.ts` | named nav + `aria-current`, launcher keyboard reachability, `status` errors |
| `screenshots.spec.ts` | semantic assertions + deterministic baselines |

## Plugin UI suite

`plugins/sonicboom/ui/tests/` runs against the **compiled** `ui/dist/`
bundle (`bun run build:sonicboom-ui` first), so the suite verifies what
actually ships in the isolated view.

| file | coverage |
|------|----------|
| `tts-ui.spec.ts` | boot from broker (settings/voices/outputs), tester speak + log, failure surface, save coalescing, output refresh |
| `interop.spec.ts` | the real `PluginWebviewHost` (bundled from `src/web` at test time) drives the real UI in a sandboxed iframe over `postMessage` |

The interop spec pins the versioned broker envelope on both sides at
once — client/shim drift fails here, not in production. (It once caught
a real bug: Vue reactive Proxies posted across the frame boundary,
which structured clone rejects.)

## Screenshots

Baselines live in `tests/e2e/screenshots.spec.ts-snapshots/` (one
Chromium project owns them). Stability rules: fixed 1440×900 viewport,
`deviceScaleFactor: 1`, fixed locale/theme/mock data, frozen clock
(`page.clock.install`), no real HTTP, disabled animations, hidden caret,
no remote avatars.

Regenerating baselines:

```bash
bun run test:e2e:update
```

**Do not blindly approve changed screenshots.** Inspect the diff —
`git diff` on the PNGs plus the Playwright report — and confirm every
pixel change is an intended visual change before committing. Layout
regressions from renderer refactors show up here first.

Inspecting failures:

```bash
bunx playwright show-trace test-results/<failing-test>/trace.zip
```

## Adding a test

1. Build the host state from `fixtures/states.ts` (extend it with new
   deterministic states rather than inlining random data).
2. `installFakeHost(page, state)` before `page.goto('/')`.
3. Prefer semantic locators: `getByRole`, `getByLabel`, `getByText`.
4. Assert semantic state first, screenshot second.
5. End with `assertNoUnhandledCalls(page)` + `consoleCapture.assertClean()`.

# TikTools documentation and demo implementation

The work starts at `remake` source `80c5cb1fbfa45eaeb1c086c71320a15e565d8541`, adapting the earlier `b72370014ac0cdd4da01be43e7ad86d19d4c9d1c` audit. It lives on `feat/docs-site-demo`; the original checkout and its branch are preserved. The final build's exact source revision is recorded in `dist/site/site-manifest.json`, together with every published file's SHA-256.

## Delivered coverage

| Surface | English | Spanish | Notes |
| --- | ---: | ---: | --- |
| User guides | 26 | 26 | Installation/LIVE/points/statistics/automation/plugins/audio/OBS/Cloud/collaboration/preferences/support |
| Developer guides and references | 47 | 47 | Includes 17 executable component families and a generated 291-method RPC reference |
| Release notes | 5 | 5 | Index plus RC1, RC2, RC3 and 4.2.2; source, experiment and publication evidence are distinguished |
| Locale entry | 1 | 1 | Independent User/Developer navigation and equivalent routes/heading IDs |

There are 159 public Markdown pages including the bilingual root selector. Every translated page carries reviewed source/translation revisions. Missing/stale translations, broken source destinations, lost legacy anchors and drifted contracts fail validation. Public content does not silently inherit a Vue i18n translation. The application/demo retains `en`, `es`, `pt`, `fr`; public Markdown is EN/ES. Search explicitly excludes historical/internal/draft/opted-out content. Slugs, canonical/hreflang, language switching and the deployment base are checked.

`docs/inventory.json` records 71 original Markdown files (69 under docs plus README and CONTRIBUTING) and 11 images. `docs/migration-map.json` supplies topic-specific destinations, audience/status, link treatments, migration state and byte-preserved archives. All original paths remain meaningful compatibility stubs or maintained roots. The archive and stubs stay outside the published srcDir and search. New legacy Markdown cannot enter unnoticed. The current public-link audit checked 38 external destinations successfully; historical URLs remain documented evidence.

## Real UI and simulated boundaries

The explicit mount factory shares App, Pinia, router, i18n, pages and components. The native entry retains its actual transport selection and frontend-ready handshake. The public demo injects a memory ControlTransport; ordinary native startup never falls back to it. Pure fixture data is shared with E2E tests; Playwright, INIT_SCRIPT and mutable E2E globals are excluded from public bundles.

Eleven validated scenarios cover connection, feed, automation, points, analytics, overlays, plugins, Cloud changes, collaboration, preferences and diagnostics/errors. Request IDs, rpc-response envelopes, discovery, version metadata, current feed topics and errors use the real frontend contract. Unsupported code/effects fail explicitly. The runtime has fixed time/data, bounded records/UTF-8 response queues/events/runs, pause and per-instance reset. Storage uses `tiktools.demo.<instance>.*`, with browsing-context identifiers preventing copied URLs from sharing a namespace; reset reloads the frame to clear journals, i18n and diagnostics without clearing other frames or native preference keys. Page exit disposes timers, subscriptions, transport and stores. Diagnostic surfaces remain the desktop panel and mobile page. Public CSP blocks outgoing connections.

The 17 families demonstrate buttons/actions, menus, fields/numbers, selection, switches, forms, tabs/toolbars, lists/tables, filters, feedback, panels/dialogs, FloatingPanel, split/master-detail, JSON view/diff, import review and actual UI-kit Cloud/overlay patterns. Component API tables derive from source macros; snippets and previews use the same executable files. A small button example is rendered directly in Markdown; global styles, portals and stateful examples use on-demand iframes.

Seven Driver.js tours are shared with the application. They have versioned IDs/steps, stable targets, translated catalogs, required capabilities and missing-target policies. The coordinator bounds waits, respects actual visibility/reduced motion, distinguishes dismissal from verified completion, permits its own router navigation, cancels external navigation and cleans observers/listeners/timers. It does not execute task effects for the visitor.

## Reproduction

```sh
bun ci
bun scripts/task.ts docs check
bun scripts/task.ts site build
bun scripts/task.ts docs test
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts site preview
```

The assembled production artifact requires a clean checkout and matching docs/demo SHA/base. `TIKTOOLS_SITE_ALLOW_DIRTY=1` is available only for a local preview; publication rejects dirty artifacts. Docs builds consume checked-in schemas without Cargo. `bun docs/scripts/regenerate-control.ts` is the explicit Rust regeneration path when contract sources change.

Independent outputs are `dist/web`, `dist/docs`, `dist/demo`, and assembled `dist/site`. Run `docs dev` and `demo dev` separately when authoring. The docs test task also builds `dist/web` for the native-bridge absence regression. Browser verification uses `playwright.site.config.ts`, a subpath-serving static server and isolated artifact directories per test port.

## Verification and limits

The implementation review passed the full frontend suites: 519 Bun tests (three explicit platform/external-fixture skips), 1,538 Vue tests and 10 widget tests; frontend lint/types and the existing production build passed. Focused runtime/tour/diagnostic checks passed 67 tests; content/publication/startup-mode checks passed 52 Bun tests, including real temporary-Git concurrent writers and rollback preservation. The assembled link check covers 161 HTML files and 9,816 built links/assets/anchors. The browser suite exercises every scenario and component family, the connection/event/action/result chain, all seven tours, four UI locales, real plugin/overlay/Variables/collaboration interactions, Markdown/search/mobile routes, iframe isolation, native-bridge absence and bundle hygiene. The final CI run/artifact is the authority for its final counts and source SHA.

This is browser/frontend validation with fictional data. It does not establish a real TikTok connection, OBS Browser Source, Wry/WebView behavior, production Cloud compatibility, a native updater replacement or collaboration between computers. Rankings remains hidden. The website is not automatically published by a source-branch push.

## Publication contract

The website and release feed writers share bounded CAS/reapply against the latest `gh-pages` tree and normal fast-forward pushes. Site ownership is explicit through a hash manifest; `/updates/`, signatures, CNAME, `.nojekyll`, foreign files and modes are protected. Feed publication authenticates the latest channel heads on each retry and does not downgrade versions or rewrite same-version signed bytes. Rollback of documentation cannot restore former feeds. Initial adoption of existing site files requires a reviewed ownership seed.

The verified current destination is legacy Pages from `gh-pages:/`, `https://nglmercer.github.io/TikTools-app/`, without CNAME. Site publication is a manual `remake` action; it checks the existing destination and explicitly triggers/observes Pages, since GITHUB_TOKEN pushes alone do not trigger legacy builds. Cross-repository tokens need target Contents and Pages permissions. Canonical Actions remain disabled according to the user's preference; builds can run in the secondary account. No live website or feed publication is claimed by the local checks.

/**
 * Minimal protocol-speaking plugin document for the inline-frame specs.
 *
 * Served through a Playwright route at the `__TIKTOOLS_PLUGIN_UI_BASE__`
 * override (real Chromium has no `tiktools-plugin://` scheme): on load it
 * reads settings and subscribes to `plugin.event` over `postMessage`;
 * its Speak button executes the speak action. The host side under test —
 * `PluginFrame` + the real `PluginWebviewHost` + the real control client
 * — is production code throughout.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { Page, Route } from '@playwright/test';

export const PLUGIN_FIXTURE_BASE = '/__plugin-fixture/';

export const PLUGIN_FRAME_FIXTURE_HTML = `<!doctype html>
<html><body>
<div id="server">(loading)</div>
<button id="speak" type="button">Speak</button>
<div id="log" role="log"></div>
<div id="events"></div>
<script>
(function () {
  var seq = 0;
  var pending = new Map();
  window.addEventListener('message', function (event) {
    var envelope = event.data;
    if (!envelope || envelope.apiVersion !== 1) return;
    if (typeof envelope.event === 'string') {
      document.getElementById('events').textContent = JSON.stringify(envelope.data);
      return;
    }
    var entry = pending.get(envelope.id);
    if (!entry) return;
    pending.delete(envelope.id);
    if (envelope.ok) entry.resolve(envelope.result);
    else entry.reject(new Error(envelope.error || 'broker error'));
  });
  function call(method, params) {
    seq += 1;
    var id = 'fixture-' + seq;
    return new Promise(function (resolve, reject) {
      pending.set(id, { resolve: resolve, reject: reject });
      window.parent.postMessage({ apiVersion: 1, id: id, method: method, params: params }, '*');
    });
  }
  window.__fixtureReady = call('settings.get', {}).then(function (values) {
    document.getElementById('server').textContent = values.serverUrl || '(none)';
    return call('events.subscribe', { topics: ['plugin.event'] });
  }).catch(function (error) {
    document.getElementById('server').textContent = 'error: ' + error.message;
  });
  document.getElementById('speak').addEventListener('click', function () {
    call('actions.execute', { action: 'sonicboom.server.speak', config: { text: 'hi' } }).then(
      function (outcome) {
        document.getElementById('log').textContent = (outcome.ok ? 'ok' : 'fail') + ' ' + outcome.summary;
      },
      function (error) {
        document.getElementById('log').textContent = 'error: ' + error.message;
      }
    );
  });
})();
</script>
</body></html>`;

export async function installPluginUiOverride(page: Page): Promise<void> {
  await page.addInitScript((base: string) => {
    (window as unknown as Record<string, unknown>)['__TIKTOOLS_PLUGIN_UI_BASE__'] = base;
  }, PLUGIN_FIXTURE_BASE);
}

export async function routePluginFixture(page: Page): Promise<void> {
  await page.route(
    (url) => url.pathname.startsWith('/__plugin-fixture/'),
    async (route: Route) => {
      await route.fulfill({
        status: 200,
        contentType: 'text/html',
        body: PLUGIN_FRAME_FIXTURE_HTML,
      });
    },
  );
}

/**
 * External-asset plugin document: mirrors the production failure mode the
 * inline fixture above cannot reproduce. The built plugin bundle loads its
 * behavior from external module scripts and stylesheets
 * (`<script type="module" crossorigin>` / `<link rel="stylesheet"
 * crossorigin>`), and the production frame is `sandbox="allow-scripts"`,
 * so the document carries an opaque origin and strict engines CORS-check
 * those subresources. The module marks `#external` and the stylesheet
 * colors it; either staying unapplied means the assets were blocked.
 */
export const PLUGIN_EXTERNAL_BASE = '/__plugin-external/';

export const PLUGIN_EXTERNAL_HTML = `<!doctype html>
<html><head>
<link rel="stylesheet" crossorigin href="./ext/app.css">
</head><body>
<div id="external">(pending)</div>
<script type="module" crossorigin src="./ext/app.js"></script>
</body></html>`;

export const PLUGIN_EXTERNAL_JS = `document.getElementById('external').textContent = 'external-js-ok';\n`;

export const PLUGIN_EXTERNAL_CSS = `#external { color: rgb(1, 2, 3); }\n`;

/**
 * Minimal host page for the external-asset document: one production-shaped
 * `sandbox="allow-scripts"` frame, no broker. The suites navigate here and
 * assert inside `#plugin-frame`.
 */
export const PLUGIN_EXTERNAL_HARNESS_HTML = `<!doctype html>
<html><body>
<iframe id="plugin-frame" sandbox="allow-scripts" src="./index.html" style="width:800px;height:600px;border:0"></iframe>
</body></html>`;

/**
 * Serves the external-asset document plus its module/CSS subresources.
 * With `withCors` the subresources answer `Access-Control-Allow-Origin:
 * *`, as every production plugin-asset host must; without it they answer
 * an explicitly non-matching origin, so the module load is CORS-blocked
 * in the opaque-origin frame, reproducing the blank production iframe
 * (missing marker, CORS console error).
 *
 * The non-matching value must be explicit: `route.fulfill` auto-injects
 * permissive CORS headers when none are given, which would hide the very
 * bug this fixture reproduces — the same way `vite preview` headers hide
 * a broken production asset server.
 */
export async function routeExternalAssetFixture(
  page: Page,
  options: { withCors: boolean },
): Promise<void> {
  const cors: Record<string, string> = options.withCors
    ? { 'access-control-allow-origin': '*' }
    : { 'access-control-allow-origin': 'https://example.invalid' };
  await page.route(
    (url) => url.pathname.startsWith(PLUGIN_EXTERNAL_BASE),
    async (route: Route) => {
      const pathname = new URL(route.request().url()).pathname;
      const relative = pathname.slice(PLUGIN_EXTERNAL_BASE.length);
      if (relative === 'harness.html') {
        await route.fulfill({
          status: 200,
          contentType: 'text/html',
          body: PLUGIN_EXTERNAL_HARNESS_HTML,
        });
        return;
      }
      if (relative === 'index.html') {
        await route.fulfill({
          status: 200,
          contentType: 'text/html',
          headers: cors,
          body: PLUGIN_EXTERNAL_HTML,
        });
        return;
      }
      if (relative === 'ext/app.js') {
        await route.fulfill({
          status: 200,
          contentType: 'text/javascript',
          headers: cors,
          body: PLUGIN_EXTERNAL_JS,
        });
        return;
      }
      if (relative === 'ext/app.css') {
        await route.fulfill({
          status: 200,
          contentType: 'text/css',
          headers: cors,
          body: PLUGIN_EXTERNAL_CSS,
        });
        return;
      }
      await route.fulfill({ status: 404, contentType: 'text/plain', body: 'not found' });
    },
  );
}

const COMPILED_UI_DIST = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
  '..',
  'plugins',
  'sonicboom',
  'ui',
  'dist',
);

const COMPILED_UI_MIME: Record<string, string> = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

/**
 * Serves the REAL compiled SonicBoom bundle (`plugins/sonicboom/ui/dist/`,
 * `bun run build:sonicboom-ui` first) at the `__TIKTOOLS_PLUGIN_UI_BASE__`
 * override, so the inline tab renders production assets end to end. Every
 * response carries `Access-Control-Allow-Origin: *`, as the production
 * asset server must (pinned by the Rust header tests). The production CSP
 * header is deliberately NOT reproduced here: its `tiktools-plugin:`
 * sources can never match this suite's `http://127.0.0.1` origin, so
 * sending it would block loads that production allows.
 */
export async function routeCompiledPluginUi(
  page: Page,
  pluginId = 'sonicboom.server',
): Promise<void> {
  const index = join(COMPILED_UI_DIST, 'index.html');
  if (!existsSync(index)) {
    throw new Error(
      `Compiled plugin UI missing at ${COMPILED_UI_DIST} — run \`bun run build:sonicboom-ui\` first.`,
    );
  }
  const prefix = `${PLUGIN_FIXTURE_BASE}${pluginId}/`;
  await page.route(
    (url) => url.pathname.startsWith(prefix),
    async (route: Route) => {
      const pathname = new URL(route.request().url()).pathname;
      const relative = decodeURIComponent(pathname.slice(prefix.length));
      const file = resolve(COMPILED_UI_DIST, ...relative.split('/'));
      if (!file.startsWith(COMPILED_UI_DIST + sep) || !existsSync(file)) {
        await route.fulfill({
          status: 404,
          contentType: 'text/plain',
          headers: { 'access-control-allow-origin': '*' },
          body: 'compiled plugin asset not found',
        });
        return;
      }
      const dot = file.lastIndexOf('.');
      const contentType =
        dot >= 0
          ? (COMPILED_UI_MIME[file.slice(dot)] ?? 'application/octet-stream')
          : 'application/octet-stream';
      await route.fulfill({
        status: 200,
        contentType,
        headers: { 'access-control-allow-origin': '*' },
        body: readFileSync(file),
      });
    },
  );
}

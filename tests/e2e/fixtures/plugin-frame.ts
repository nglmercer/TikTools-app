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

import { expect, test } from 'bun:test';

import {
  BROKER_API_VERSION,
  PluginWebviewHost,
  pluginUiUrl,
  type BrokerBackend,
  type BrokerEvent,
  type BrokerOkResponse,
  type BrokerResponse,
} from './plugin-webview-host.ts';

function ok(entry: BrokerResponse | BrokerEvent): BrokerOkResponse {
  const response = entry as BrokerOkResponse;
  if (response.ok !== true) throw new Error(`expected ok response, got ${JSON.stringify(entry)}`);
  return response;
}

function fakeBackend(): BrokerBackend & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    async getSettings() {
      calls.push('settings.get');
      return { voice: 'M1' };
    },
    async setSettings(values) {
      calls.push('settings.set');
      return values;
    },
    async executeAction(action, config) {
      calls.push(`actions.execute:${action}`);
      return { action, config, ok: true };
    },
    async getOptions(source, refresh) {
      calls.push(`options.get:${source}:${refresh}`);
      return { options: [], selected: null };
    },
    async getLocale() {
      return 'en';
    },
    async getTheme() {
      return 'dark';
    },
  };
}

function harness(pluginId = 'owner.plugin') {
  const frameWindow = {} as WindowProxy;
  const posted: Array<BrokerResponse | BrokerEvent> = [];
  const backend = fakeBackend();
  const host = new PluginWebviewHost(frameWindow, {
    pluginId,
    backend,
    postToFrame: (envelope) => {
      posted.push(envelope);
    },
  });
  const send = (id: string, method: string, params: Record<string, unknown> = {}) =>
    host.handleFrameMessage({ apiVersion: BROKER_API_VERSION, id, method, params }, frameWindow);
  return { host, backend, posted, frameWindow, send };
}

test('routes known methods to the backend with correlated ids', async () => {
  const { posted, send, backend } = harness();
  await send('1', 'settings.get');
  await send('2', 'settings.set', { values: { voice: 'F1' } });
  await send('3', 'actions.execute', { action: 'owner.plugin.speak', config: { text: 'hi' } });
  await send('4', 'options.get', { source: 's', refresh: true });
  await send('5', 'host.locale');
  await send('6', 'host.theme');
  expect(posted).toHaveLength(6);
  expect(posted.map((entry) => ok(entry).id)).toEqual(['1', '2', '3', '4', '5', '6']);
  expect(ok(posted[0] as BrokerResponse).result).toEqual({ voice: 'M1' });
  expect(ok(posted[4] as BrokerResponse).result).toBe('en');
  expect(ok(posted[5] as BrokerResponse).result).toBe('dark');
  expect(backend.calls).toContain('settings.get');
  expect(backend.calls).toContain('actions.execute:owner.plugin.speak');
});

test('drops foreign sources, versions, and malformed envelopes', async () => {
  const { host, posted, frameWindow } = harness();
  // Wrong source window: silent drop, no backend contact.
  await host.handleFrameMessage({ apiVersion: 1, id: 'x', method: 'settings.get', params: {} }, {} as WindowProxy);
  // Wrong version: silent drop.
  await host.handleFrameMessage({ apiVersion: 999, id: 'y', method: 'settings.get', params: {} }, frameWindow);
  // Missing id: silent drop (nothing to correlate).
  await host.handleFrameMessage({ apiVersion: 1, method: 'settings.get', params: {} }, frameWindow);
  // Non-object: silent drop.
  await host.handleFrameMessage('settings.get', frameWindow);
  expect(posted).toHaveLength(0);
  // Unknown method and bad params: correlated errors.
  await host.handleFrameMessage({ apiVersion: 1, id: 'a', method: 'plugins.uninstall', params: {} }, frameWindow);
  await host.handleFrameMessage({ apiVersion: 1, id: 'b', method: 'settings.set', params: { values: 'nope' } }, frameWindow);
  await host.handleFrameMessage({ apiVersion: 1, id: 'c', method: 'settings.get', params: [] }, frameWindow);
  expect(posted).toHaveLength(3);
  expect(posted.map((entry) => (entry as BrokerResponse).id)).toEqual(['a', 'b', 'c']);
  expect(posted.every((entry) => (entry as BrokerResponse).ok === false)).toBe(true);
});

test('rejects mismatched plugin identities', async () => {
  const { posted, host, frameWindow } = harness('owner.plugin');
  await host.handleFrameMessage(
    { apiVersion: 1, id: '1', method: 'host.locale', params: { pluginId: 'someone.else' } },
    frameWindow,
  );
  expect(posted).toHaveLength(1);
  expect((posted[0] as BrokerResponse).ok).toBe(false);
  // A matching identity is accepted (and ignored — the host injects its own).
  await host.handleFrameMessage(
    { apiVersion: 1, id: '2', method: 'host.locale', params: { pluginId: 'owner.plugin' } },
    frameWindow,
  );
  expect((posted[1] as BrokerResponse).ok).toBe(true);
});

test('subscriptions gate event pushes by topic', async () => {
  const subscribed: string[] = [];
  const frameWindow = {} as WindowProxy;
  const posted: Array<BrokerResponse | BrokerEvent> = [];
  const host = new PluginWebviewHost(frameWindow, {
    pluginId: 'owner.plugin',
    backend: fakeBackend(),
    postToFrame: (envelope) => {
      posted.push(envelope);
    },
    subscribeTopic: (topic) => {
      subscribed.push(topic);
      return () => {
        subscribed.splice(subscribed.indexOf(topic), 1);
      };
    },
  });
  // Unsubscribed pushes never reach the frame.
  host.pushEvent('plugin.event', {});
  expect(posted).toHaveLength(0);
  await host.handleFrameMessage(
    { apiVersion: 1, id: '1', method: 'events.subscribe', params: { topics: ['plugin.event'] } },
    frameWindow,
  );
  expect(subscribed).toEqual(['plugin.event']);
  host.pushEvent('plugin.event', { hello: true });
  host.pushEvent('plugin.status', {});
  expect(posted).toHaveLength(2);
  expect(posted[1]).toEqual({ apiVersion: 1, event: 'plugin.event', data: { hello: true } });
  // Non-plugin topics are rejected without subscribing.
  await host.handleFrameMessage(
    { apiVersion: 1, id: '2', method: 'events.subscribe', params: { topics: ['live.event'] } },
    frameWindow,
  );
  expect((posted[2] as BrokerResponse).ok).toBe(false);
  expect(subscribed).toEqual(['plugin.event']);
  await host.handleFrameMessage(
    { apiVersion: 1, id: '3', method: 'events.unsubscribe', params: { topics: ['plugin.event'] } },
    frameWindow,
  );
  expect(subscribed).toEqual([]);
  host.pushEvent('plugin.event', {});
  expect(posted).toHaveLength(4);
  host.dispose();
});

test('frame URLs stay on the plugin scheme with a test override', () => {
  const scope = globalThis as Record<string, unknown>;
  const previous = scope['window'];
  try {
    // Plain browser without the desktop shell: no servable URL.
    scope['window'] = {};
    expect(pluginUiUrl('sonicboom.server', 'ui/dist/index.html', 'tts')).toBe(null);
    // Desktop shell: custom scheme, entry file only, page in the hash.
    scope['window'] = { ipc: { postMessage: () => undefined } };
    expect(pluginUiUrl('sonicboom.server', 'ui/dist/index.html', 'tts')).toBe(
      'tiktools-plugin://app/sonicboom.server/index.html#page=tts',
    );
    // Test override wins over the desktop default.
    (scope['window'] as Record<string, unknown>)['__TIKTOOLS_PLUGIN_UI_BASE__'] =
      '/__plugin-fixture/';
    expect(pluginUiUrl('sonicboom.server', 'ui/dist/index.html', 'tts')).toBe(
      '/__plugin-fixture/sonicboom.server/index.html#page=tts',
    );
    // Entries that escape the ui document shape never produce a URL.
    expect(pluginUiUrl('sonicboom.server', 'ui/dist/app.js', 'tts')).toBe(null);
    expect(pluginUiUrl('sonicboom.server', 'ui\\dist\\index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('sonicboom.server', 'ui/dist/index.html', '  ')).toBe(null);
  } finally {
    if (previous === undefined) delete scope['window'];
    else scope['window'] = previous;
  }
});

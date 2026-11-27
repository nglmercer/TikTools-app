import { createControlClient } from '@/shared/api/control-client.ts';
import { expect, test, vi } from 'vitest';

import {
  BROKER_API_VERSION,
  PluginWebviewHost,
  controlBackend,
  pluginUiUrl,
  type BrokerBackend,
  type BrokerEvent,
  type BrokerOkResponse,
  type BrokerResponse,
} from '../../../../../../src/web/entities/plugin/api/plugin-webview-host.ts';
import { PLUGIN_ID, RPC, TOPIC } from '../../../../../shared/index.ts';

type Posted = BrokerResponse | BrokerEvent | undefined;

/** The reply to a frame request (as opposed to a pushed event); throws when the entry is missing or an event. */
function reply(entry: Posted): BrokerResponse {
  if (entry === undefined || !('ok' in entry)) throw new Error(`expected a reply, got ${JSON.stringify(entry)}`);
  return entry;
}

function ok(entry: Posted): BrokerOkResponse {
  const response = reply(entry);
  if (response.ok !== true) throw new Error(`expected ok response, got ${JSON.stringify(entry)}`);
  return response;
}

/** A distinct window-like object: `Object.create` is untyped, so it stands in for a frame window without a cast. */
function newFrameWindow(): WindowProxy {
  return Object.create(window);
}

/** Replace (or, with `undefined`, remove) the page `window` global the desktop-shell detection reads. */
function setPageWindow(value: unknown): void {
  if (value === undefined) Reflect.deleteProperty(globalThis, 'window');
  else Reflect.set(globalThis, 'window', value);
}

function fakeBackend(): BrokerBackend & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    async getSettings() {
      calls.push(RPC.settingsGet);
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
  const frameWindow = newFrameWindow();
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
  await send('1', RPC.settingsGet);
  await send('2', 'settings.set', { values: { voice: 'F1' } });
  await send('3', 'actions.execute', { action: 'owner.plugin.speak', config: { text: 'hi' } });
  await send('4', 'options.get', { source: 's', refresh: true });
  await send('5', 'host.locale');
  await send('6', 'host.theme');
  expect(posted).toHaveLength(6);
  expect(posted.map((entry) => ok(entry).id)).toEqual(['1', '2', '3', '4', '5', '6']);
  expect(ok(posted[0]).result).toEqual({ voice: 'M1' });
  expect(ok(posted[4]).result).toBe('en');
  expect(ok(posted[5]).result).toBe('dark');
  expect(backend.calls).toContain(RPC.settingsGet);
  expect(backend.calls).toContain('actions.execute:owner.plugin.speak');
});

test('drops foreign sources, versions, and malformed envelopes', async () => {
  const { host, posted, frameWindow } = harness();
  // Wrong source window: silent drop, no backend contact.
  await host.handleFrameMessage({ apiVersion: 1, id: 'x', method: RPC.settingsGet, params: {} }, newFrameWindow());
  // Wrong version: silent drop.
  await host.handleFrameMessage({ apiVersion: 999, id: 'y', method: RPC.settingsGet, params: {} }, frameWindow);
  // Missing id: silent drop (nothing to correlate).
  await host.handleFrameMessage({ apiVersion: 1, method: RPC.settingsGet, params: {} }, frameWindow);
  // Non-object: silent drop.
  await host.handleFrameMessage(RPC.settingsGet, frameWindow);
  expect(posted).toHaveLength(0);
  // Unknown method and bad params: correlated errors.
  await host.handleFrameMessage({ apiVersion: 1, id: 'a', method: 'plugins.uninstall', params: {} }, frameWindow);
  await host.handleFrameMessage({ apiVersion: 1, id: 'b', method: 'settings.set', params: { values: 'nope' } }, frameWindow);
  await host.handleFrameMessage({ apiVersion: 1, id: 'c', method: RPC.settingsGet, params: [] }, frameWindow);
  expect(posted).toHaveLength(3);
  expect(posted.map((entry) => reply(entry).id)).toEqual(['a', 'b', 'c']);
  expect(posted.every((entry) => reply(entry).ok === false)).toBe(true);
});

test('rejects mismatched plugin identities', async () => {
  const { posted, host, frameWindow } = harness('owner.plugin');
  await host.handleFrameMessage(
    { apiVersion: 1, id: '1', method: 'host.locale', params: { pluginId: 'someone.else' } },
    frameWindow,
  );
  expect(posted).toHaveLength(1);
  expect(reply(posted[0]).ok).toBe(false);
  // A matching identity is accepted (and ignored — the host injects its own).
  await host.handleFrameMessage(
    { apiVersion: 1, id: '2', method: 'host.locale', params: { pluginId: 'owner.plugin' } },
    frameWindow,
  );
  expect(reply(posted[1]).ok).toBe(true);
});

test('subscriptions gate event pushes by topic', async () => {
  // Arrange: a host whose subscribeTopic hook records which topics the bus is currently listening to
  const subscribed: string[] = [];
  const frameWindow = newFrameWindow();
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
  host.pushEvent(TOPIC.pluginEvent, {});
  expect(posted).toHaveLength(0);
  await host.handleFrameMessage(
    { apiVersion: 1, id: '1', method: RPC.eventsSubscribe, params: { topics: [TOPIC.pluginEvent] } },
    frameWindow,
  );
  expect(subscribed).toEqual([TOPIC.pluginEvent]);
  // Act: once subscribed, the plugin topic is delivered, while an unsubscribed topic stays filtered
  host.pushEvent(TOPIC.pluginEvent, { hello: true });
  host.pushEvent('plugin.status', {});
  expect(posted).toHaveLength(2);
  expect(posted[1]).toEqual({ apiVersion: 1, event: TOPIC.pluginEvent, data: { hello: true } });
  // Non-plugin topics are rejected without subscribing.
  await host.handleFrameMessage(
    { apiVersion: 1, id: '2', method: RPC.eventsSubscribe, params: { topics: [TOPIC.liveEvent] } },
    frameWindow,
  );
  expect(reply(posted[2]).ok).toBe(false);
  expect(subscribed).toEqual([TOPIC.pluginEvent]);
  // Act: unsubscribing releases the topic, so the shared bus no longer delivers it to this frame
  await host.handleFrameMessage(
    { apiVersion: 1, id: '3', method: 'events.unsubscribe', params: { topics: [TOPIC.pluginEvent] } },
    frameWindow,
  );
  expect(subscribed).toEqual([]);
  // Assert: with the topic released, the final push never reaches the frame
  host.pushEvent(TOPIC.pluginEvent, {});
  expect(posted).toHaveLength(4);
  host.dispose();
});

test('frame URLs stay on the plugin scheme with a test override', () => {
  const previous: unknown = Reflect.get(globalThis, 'window');
  try {
    // Plain browser without the desktop shell: no servable URL.
    setPageWindow({});
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts')).toBe(null);
    // Desktop shell: custom scheme, entry file only, page in the hash.
    setPageWindow({ ipc: { postMessage: () => undefined } });
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts')).toBe(
      'tiktools-plugin://app/sonicboom.server/index.html#page=tts',
    );
    // Test override wins over the desktop default.
    setPageWindow({ ipc: { postMessage: () => undefined }, __TIKTOOLS_PLUGIN_UI_BASE__: '/__plugin-fixture/' });
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts')).toBe(
      '/__plugin-fixture/sonicboom.server/index.html#page=tts',
    );
    // Entries that escape the ui document shape never produce a URL.
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/app.js', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui\\dist\\index.html', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', '  ')).toBe(null);
  } finally {
    setPageWindow(previous);
  }
});

test('Windows WebView2 uses the rewritten plugin asset host', () => {
  const previousWindow: unknown = Reflect.get(globalThis, 'window');
  const userAgent = vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue('Mozilla/5.0 (Windows NT 10.0) Edg/140.0');
  try {
    setPageWindow({ ipc: { postMessage: () => undefined } });
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts')).toBe(
      'http://tiktools-plugin.app/sonicboom.server/index.html#page=tts',
    );
  } finally {
    userAgent.mockRestore();
    setPageWindow(previousWindow);
  }
});

test('frame URLs reject identifier and path injection', () => {
  const previous: unknown = Reflect.get(globalThis, 'window');
  // Arrange: run as the desktop shell, so every accepted URL uses the plugin scheme
  setPageWindow({ ipc: { postMessage: () => undefined } });
  try {
    const ok = 'tiktools-plugin://app/sonicboom.server/index.html#page=tts';
    // Assert: a well-formed id and entry still resolve, so the nulls below come from the checks
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts')).toBe(ok);
    // Act: each hostile id, entry or page id is tried alone; any one that yields a URL fails
    // Plugin ids cannot inject paths: the id travels as a URL segment.
    expect(pluginUiUrl('', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('../evil', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('..%2fwebui', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('sonicboom.server/evil', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('sonicboom.server%2findex.html', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('sonicboom\\server', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('9lives', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('has space', 'ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl('x'.repeat(129), 'ui/dist/index.html', 'tts')).toBe(null);
    // Entries stay HTML basenames under relative directories.
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, '../index.html', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, '..', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/../index.html', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, '/ui/dist/index.html', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui//index.html', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html/', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/.html', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.HTML', 'tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html\0.html', 'tts')).toBe(null);
    // Only the basename travels: directory segments never reach the URL,
    // so an encoded segment there cannot inject a server path.
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, '%2e%2e/index.html', 'tts')).toBe(ok);
    // Page ids ride the fragment but still follow the identifier rule.
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', '')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', '../tts')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts/evil')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'tts evil')).toBe(null);
    expect(pluginUiUrl(PLUGIN_ID.sonicboomServer, 'ui/dist/index.html', 'x'.repeat(129))).toBe(null);
    // Dots, dashes, and underscores stay valid on both ids.
    expect(pluginUiUrl('plugin-a_b.c', 'ui/dist/index.html', 'page-1_x.y')).toBe(
      'tiktools-plugin://app/plugin-a_b.c/index.html#page=page-1_x.y',
    );
  } finally {
    setPageWindow(previous);
  }
});

test('dispose during a pending backend call prevents responses to the old frame', async () => {
  const { host, backend, send, posted } = harness();
  let complete!: (value: Record<string, unknown>) => void;
  backend.getSettings = () => new Promise((resolve) => { complete = resolve; });
  const pending = send('pending', RPC.settingsGet);
  host.dispose();
  host.dispose();
  complete({ voice: 'M1' });
  await pending;
  await send('stale', RPC.eventsSubscribe, { topics: [TOPIC.pluginEvent] });
  expect(posted).toHaveLength(0);
});

test('iframe admission stays bounded after deadlines and disposal clears timers', async () => {
  const frame = newFrameWindow();
  const posted: Array<BrokerResponse | BrokerEvent> = [];
  let complete!: (value: Record<string, unknown>) => void;
  const stalled = new Promise<Record<string, unknown>>((resolve) => { complete = resolve; });
  const host = new PluginWebviewHost(frame, {
    pluginId: 'owner.plugin', backend: { ...fakeBackend(), getSettings: () => stalled },
    postToFrame: (entry) => posted.push(entry), requestTimeoutMs: 5,
  });
  const requests = Array.from({ length: 128 }, (_, id) => host.handleFrameMessage({ apiVersion: 1, id: String(id), method: RPC.settingsGet }, frame));
  await host.handleFrameMessage({ apiVersion: 1, id: 'overflow', method: RPC.settingsGet }, frame);
  expect(host.diagnostics().pending).toBe(128);
  expect(reply(posted[0]).id).toBe('overflow');
  await new Promise((resolve) => setTimeout(resolve, 20));
  expect(host.diagnostics()).toEqual({ pending: 128, timers: 0, subscriptions: 0 });
  expect(posted).toHaveLength(129);
  host.dispose();
  expect(host.diagnostics()).toEqual({ pending: 0, timers: 0, subscriptions: 0 });
  complete({});
  await Promise.all(requests);
  expect(posted).toHaveLength(129);
});

test('concurrent subscribe requests cannot exceed the aggregate topic capacity', async () => {
  const { host, posted, send } = harness();
  await Promise.all([
    send('first', RPC.eventsSubscribe, { topics: Array.from({ length: 32 }, (_, index) => `plugin.first.${index}`) }),
    send('second', RPC.eventsSubscribe, { topics: Array.from({ length: 32 }, (_, index) => `plugin.second.${index}`) }),
  ]);
  expect(host.diagnostics().subscriptions).toBe(32);
  expect(reply(posted[0]).ok).toBe(true);
  expect(reply(posted[1]).ok).toBe(false);
  host.dispose();
});


test('50 iframe owner disposals cancel client waits without detaching the shared control', async () => {
  setPageWindow({ ipc: { postMessage() {} } });
  const control = createControlClient();
  control.attach();
  for (let cycle = 0; cycle < 50; ++cycle) {
    const frame = newFrameWindow();
    const posted: Array<BrokerResponse | BrokerEvent> = [];
    const host = new PluginWebviewHost(frame, {
      pluginId: 'owner.plugin', backend: controlBackend('owner.plugin', control, () => 'en', () => 'dark'),
      postToFrame: (value) => posted.push(value),
    });
    const pending = host.handleFrameMessage({ apiVersion: 1, id: 'settings', method: RPC.settingsGet }, frame);
    expect(control.diagnostics?.().pending).toBe(1);
    host.dispose();
    await pending;
    expect(control.diagnostics?.().pending).toBe(0);
    expect(host.diagnostics()).toEqual({ pending: 0, timers: 0, subscriptions: 0 });
    expect(posted).toHaveLength(0);
  }
  control.detach();
  setPageWindow(undefined);
});

test('uses the local adapter asset prefix without requiring a desktop host', () => {
  const meta = document.createElement('meta');
  meta.name = 'tiktools-plugin-assets';
  meta.content = '/__plugins/asset-only-test/';
  document.head.append(meta);
  try {
    expect(pluginUiUrl('portable-plugin', 'ui/dist/index.html', 'settings'))
      .toBe('/__plugins/asset-only-test/portable-plugin/index.html#page=settings');
  } finally { meta.remove(); }
});

import { afterEach, expect, test } from 'bun:test';

import { PluginBroker } from './broker.ts';

type FakeWindow = {
  parent: unknown;
  tiktools?: unknown;
  addEventListener: (type: string, listener: (event: { data: unknown }) => void) => void;
  removeEventListener: (type: string, listener: (event: { data: unknown }) => void) => void;
  postToParent?: (message: unknown) => void;
};

const scope = globalThis as unknown as { window: unknown };
const realWindow = scope.window;

afterEach(() => {
  scope.window = realWindow;
});

function installWindow(fake: FakeWindow): void {
  scope.window = fake;
}

function listenersOf(fake: FakeWindow): Array<(event: { data: unknown }) => void> {
  const collected: Array<(event: { data: unknown }) => void> = [];
  fake.addEventListener = (_type, listener) => {
    collected.push(listener);
  };
  return collected;
}

function nativeTikTools() {
  return {
    settings: {
      get: async () => ({ voice: 'M1' }),
      set: async (values: Record<string, unknown>) => values,
    },
    actions: {
      execute: async (action: string) => ({
        actionType: action,
        ok: true,
        summary: 'spoken',
        logs: [] as string[],
        durationMs: 1,
        error: null,
      }),
    },
    options: {
      get: async () => ({ options: [], selected: null }),
    },
    events: {
      subscribe: () => () => undefined,
    },
    host: {
      locale: async () => 'en',
      theme: async () => 'dark',
    },
  };
}

test('native injection wins over every other transport', async () => {
  const posted: unknown[] = [];
  const parent = { postMessage: (message: unknown) => posted.push(message) };
  const fake: FakeWindow = {
    parent,
    tiktools: nativeTikTools(),
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  };
  installWindow(fake);

  const broker = new PluginBroker();
  expect(broker.transport()).toBe('native');
  expect(broker.usesNativeTransport()).toBe(true);
  expect(await broker.getSettings()).toEqual({ voice: 'M1' });
  expect(await broker.getLocale()).toBe('en');
  const outcome = await broker.executeAction('sonicboom.server.speak', { text: 'hi' });
  expect(outcome.ok).toBe(true);
  // Native calls never touch the frame transport.
  expect(posted).toHaveLength(0);
  broker.dispose();
});

test('embedded frame without native injection uses postMessage', async () => {
  const posted: Array<Record<string, unknown>> = [];
  const parent = { postMessage: (message: Record<string, unknown>) => posted.push(message) };
  const fake: FakeWindow = {
    parent,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  };
  const listeners = listenersOf(fake);
  installWindow(fake);

  const broker = new PluginBroker();
  expect(broker.transport()).toBe('iframe');
  expect(broker.usesNativeTransport()).toBe(false);

  const pending = broker.getSettings();
  expect(posted).toHaveLength(1);
  expect(posted[0]).toMatchObject({ apiVersion: 1, method: 'settings.get' });
  const id = posted[0]?.id as string;
  // The host answers through the listener the broker installed.
  expect(listeners).toHaveLength(1);
  listeners[0]?.({ data: { apiVersion: 1, id, ok: true, result: { voice: 'F2' } } });
  expect(await pending).toEqual({ voice: 'F2' });
  broker.dispose();
});

test('top-level page with no native host fails explicitly and never self-posts', async () => {
  let posts = 0;
  const fake = {
    tiktools: undefined,
    addEventListener: () => {
      throw new Error('must not listen when no host can answer');
    },
    removeEventListener: () => undefined,
  } as unknown as FakeWindow & { parent: unknown };
  // A top-level page: `window.parent === window`, and any post would echo
  // the request back to the page itself.
  fake.parent = fake;
  (fake as unknown as Record<string, unknown>)['postMessage'] = () => {
    posts += 1;
  };
  installWindow(fake);

  const broker = new PluginBroker();
  expect(broker.transport()).toBe('unavailable');
  expect(broker.usesNativeTransport()).toBe(false);
  await expect(broker.getSettings()).rejects.toThrow('native plugin broker is unavailable');
  await expect(broker.getOptions('sonicboom.server.speak:voice')).rejects.toThrow(
    'native plugin broker is unavailable',
  );
  // Subscribing is a silent no-op (nothing to notify, nothing to leak).
  const unsubscribe = broker.subscribe('plugin.event', () => undefined);
  unsubscribe();
  expect(posts).toBe(0);
  broker.dispose();
});

test('partial native injection is rejected and stays explicit at top level', async () => {
  const fake = {
    tiktools: { settings: {}, actions: {}, options: {} },
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  } as unknown as FakeWindow & { parent: unknown };
  fake.parent = fake;
  installWindow(fake);

  const broker = new PluginBroker();
  expect(broker.transport()).toBe('unavailable');
  await expect(broker.getTheme()).rejects.toThrow('native plugin broker is unavailable');
  broker.dispose();
});

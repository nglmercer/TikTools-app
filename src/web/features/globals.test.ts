import { expect, test } from 'bun:test';

import type { ControlClient } from '../platform/control-client.ts';
import {
  globalSuggestionItems,
  isGlobalKey,
  useGlobals,
} from './globals.ts';

function fakeControl(
  handler: (method: string, params: unknown) => Promise<unknown>,
): { client: ControlClient; calls: Array<{ method: string; params: unknown }> } {
  const calls: Array<{ method: string; params: unknown }> = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: ((method: string, params?: unknown) => {
      calls.push({ method, params });
      return handler(method, params);
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return { client, calls };
}

test('global keys mirror the host identifier rule', () => {
  for (const key of ['commandPort', '_port', 'a', 'A1._-b']) {
    expect(isGlobalKey(key)).toBe(true);
  }
  for (const key of ['', '9lives', 'has space', 'semi;colon', 'slash/a', '{{ x }}', 'k'.repeat(65)]) {
    expect(isGlobalKey(key)).toBe(false);
  }
});

test('globals build sorted universal autocomplete rows', () => {
  const items = globalSuggestionItems({ commandPort: '46665', commandHost: '127.0.0.1' }, 'en');
  expect(items.map((item) => item.value)).toEqual(['globals.commandHost', 'globals.commandPort']);
  expect(items[0]?.label).toBe('commandHost');
  expect(items[1]?.kind).toBe('number');
  expect(items[1]?.preview).toBe('46665');
});

test('load fills state and surfaces host errors', async () => {
  const { client } = fakeControl(async (method) => {
    if (method === 'globals.list') return { globals: { commandPort: '46665', dropped: 42 } };
    throw new Error(`unexpected ${method}`);
  });
  const state = useGlobals(client);
  await state.loadGlobals();
  expect(state.globals.value).toEqual({ commandPort: '46665' });
  expect(state.error.value).toBeNull();

  const failing = useGlobals(fakeControl(async () => {
    throw new Error('host down');
  }).client);
  await failing.loadGlobals();
  expect(failing.error.value).toBe('host down');
});

test('save diffs the draft: deletes removals, writes changes only', async () => {
  const { client, calls } = fakeControl(async (method) => {
    if (method === 'globals.list') return { globals: { keep: '1', stale: 'x', change: 'old' } };
    return { ok: true };
  });
  const state = useGlobals(client);
  await state.loadGlobals();
  await state.saveGlobals({ keep: '1', change: 'new', added: 'a' });
  expect(calls.map((call) => [call.method, call.params])).toEqual([
    ['globals.list', {}],
    ['globals.delete', { key: 'stale' }],
    ['globals.set', { key: 'added', value: 'a' }],
    ['globals.set', { key: 'change', value: 'new' }],
  ]);
  expect(state.globals.value).toEqual({ keep: '1', change: 'new', added: 'a' });
});

test('a failed save keeps host state and reports the error', async () => {
  const { client } = fakeControl(async (method) => {
    if (method === 'globals.list') return { globals: { keep: '1' } };
    throw new Error('write failed');
  });
  const state = useGlobals(client);
  await state.loadGlobals();
  await expect(state.saveGlobals({ keep: '2' })).rejects.toThrow('write failed');
  expect(state.error.value).toBe('write failed');
  expect(state.globals.value).toEqual({ keep: '1' });
});

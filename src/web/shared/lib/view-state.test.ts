import { expect, test } from 'vitest';

// Bun's test runtime has no DOM: back localStorage with an
// in-memory map so the module's storage access works. The
// module only touches localStorage inside its functions, so
// installing the shim before the first call is enough.
const store = new Map<string, string>();
Object.defineProperty(globalThis, 'localStorage', {
  configurable: true,
  value: {
    getItem: (key: string): string | null =>
      store.has(key) ? (store.get(key) ?? null) : null,
    setItem: (key: string, value: string): void => {
      store.set(key, String(value));
    },
    removeItem: (key: string): void => {
      store.delete(key);
    },
    clear: (): void => {
      store.clear();
    },
  },
});

import { debounce, loadViewState, saveViewState, type FeedViewState } from './view-state.ts';

const KEY = 'tiktok-live-view-state';

function readStored(): Record<string, unknown> | null {
  const raw = localStorage.getItem(KEY);
  if (!raw) return null;
  return JSON.parse(raw) as Record<string, unknown>;
}

test('save and load round-trips every section', () => {
  localStorage.clear();
  saveViewState('tab', 'behavior');
  saveViewState('feed', {
    filter: 'gift',
    searchQuery: 'rose',
    autoScroll: false,
    scrollTop: 420,
  });
  saveViewState('behavior', {
    actionQuery: 'act',
    actionSort: 'name-desc',
    eventQuery: 'evt',
    eventSort: 'enabled',
    tab: 'actions',
    selected: 'action-1',
  });
  saveViewState('points', { searchQuery: 'alice', sortBy: 'level', sortDir: 'asc' });
  saveViewState('analytics', {
    range: 'custom',
    metric: 'gifts',
    tab: 'contributors',
    contributorQuery: 'bob',
    customStart: '2026-01-01',
    customEnd: '2026-01-31',
  });
  saveViewState('widgets', { selectedKind: 'chat' });
  saveViewState('plugins', { view: 'plugins', catalog: 'browse', query: 'tts' });
  saveViewState('connections', {
    showCookie: true,
    openServers: { srv1: true, srv2: false },
  });
  saveViewState('aside', { open: true });

  const state = loadViewState();
  expect(state.tab).toBe('behavior');
  expect(state.feed).toEqual({
    filter: 'gift',
    searchQuery: 'rose',
    autoScroll: false,
    scrollTop: 420,
  });
  expect(state.behavior).toEqual({
    actionQuery: 'act',
    actionSort: 'name-desc',
    eventQuery: 'evt',
    eventSort: 'enabled',
    tab: 'actions',
    selected: 'action-1',
  });
  expect(state.points).toEqual({ searchQuery: 'alice', sortBy: 'level', sortDir: 'asc' });
  expect(state.analytics).toEqual({
    range: 'custom',
    metric: 'gifts',
    tab: 'contributors',
    contributorQuery: 'bob',
    customStart: '2026-01-01',
    customEnd: '2026-01-31',
  });
  expect(state.widgets).toEqual({ selectedKind: 'chat' });
  expect(state.plugins).toEqual({ view: 'plugins', catalog: 'browse', query: 'tts' });
  expect(state.connections).toEqual({
    showCookie: true,
    openServers: { srv1: true, srv2: false },
  });
  expect(state.aside).toEqual({ open: true });

  // Sections never clobber each other: the blob holds all of them.
  const stored = readStored();
  expect(stored?.['version']).toBe(1);
  expect(Object.keys(stored ?? {})).toHaveLength(10);
});

test('empty storage and corrupt JSON load as empty state', () => {
  localStorage.clear();
  expect(loadViewState()).toEqual({});

  localStorage.setItem(KEY, 'not json{');
  expect(loadViewState()).toEqual({});

  localStorage.setItem(KEY, '"a string"');
  expect(loadViewState()).toEqual({});
});

test('version mismatch discards the blob', () => {
  localStorage.clear();
  localStorage.setItem(KEY, JSON.stringify({ version: 999, tab: 'points' }));
  expect(loadViewState()).toEqual({});
});

test('invalid values fall back per field, valid siblings survive', () => {
  localStorage.clear();
  // Deliberately ill-typed: the blob is user-writable storage, so
  // the loader must reject each bad field on its own.
  saveViewState('feed', {
    filter: 'nope',
    searchQuery: 42,
    autoScroll: 'yes',
    scrollTop: -50,
  } as unknown as FeedViewState);
  expect(loadViewState().feed).toBeUndefined();

  saveViewState('feed', { filter: 'chat', scrollTop: 100 });
  expect(loadViewState().feed).toEqual({ filter: 'chat', scrollTop: 100 });
});

test('scrollTop clamps to the documented bounds', () => {
  localStorage.clear();
  saveViewState('feed', { scrollTop: 1e9 });
  expect(loadViewState().feed?.scrollTop).toBe(10_000_000);

  saveViewState('feed', { scrollTop: 1.5 });
  expect(loadViewState().feed?.scrollTop).toBe(2);
});

test('strings are capped at 256 characters', () => {
  localStorage.clear();
  saveViewState('plugins', { query: 'x'.repeat(500) });
  expect(loadViewState().plugins?.query).toHaveLength(256);
});

test('tab accepts builtins and well-formed plugin pages only', () => {
  localStorage.clear();
  saveViewState('tab', 'widgets');
  expect(loadViewState().tab).toBe('widgets');

  saveViewState('tab', 'plugin:my-plugin:settings');
  expect(loadViewState().tab).toBe('plugin:my-plugin:settings');

  saveViewState('tab', 'not a tab');
  expect(loadViewState().tab).toBeUndefined();

  saveViewState('tab', 'x'.repeat(200));
  expect(loadViewState().tab).toBeUndefined();
});

test('openServers keeps only boolean entries within the cap', () => {
  localStorage.clear();
  const openServers: Record<string, unknown> = {};
  for (let index = 0; index < 80; index += 1) openServers[`srv${index}`] = index % 2 === 0;
  openServers['bad'] = 'yes';
  saveViewState('connections', { showCookie: false, openServers: openServers as Record<string, boolean> });
  const restored = loadViewState().connections?.openServers ?? {};
  expect(Object.keys(restored)).toHaveLength(64);
  expect(restored['bad']).toBeUndefined();
});

test('debounce coalesces rapid calls into one', async () => {
  let calls = 0;
  const save = debounce(() => {
    calls += 1;
  }, 20);
  save();
  save();
  save();
  expect(calls).toBe(0);
  await new Promise((resolve) => setTimeout(resolve, 45));
  expect(calls).toBe(1);
});

test('cancelled debounce releases pending callback', async () => {
  let calls = 0;
  const callback = debounce(() => { calls += 1; }, 1);
  callback();
  callback.cancel();
  await new Promise((resolve) => setTimeout(resolve, 5));
  expect(calls).toBe(0);
});

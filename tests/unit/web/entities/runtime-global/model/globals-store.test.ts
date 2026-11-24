import { describe, expect, it } from 'vitest';
import { createApp } from 'vue';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { useGlobalsStore } from '../../../../../../src/web/entities/runtime-global/model/globals-store.ts';

function setup(handler: Parameters<typeof createFakeControlClient>[0]) {
  const { client, calls } = createFakeControlClient(handler);
  const app = createApp({});
  for (const plugin of storePlugins(client)) app.use(plugin);
  const store = app.runWithContext(() => useGlobalsStore());
  return { store, calls };
}

describe('useGlobalsStore', () => {
  it('loads string globals and drops other values', async () => {
    const { store } = setup(() => ({ globals: { commandPort: '46665', dropped: 42 } }));
    await store.load();
    expect(store.globals).toEqual({ commandPort: '46665' });
    expect(store.error).toBeNull();
    expect(store.loading).toBe(false);
  });

  it('surfaces host errors on load', async () => {
    const { store } = setup(() => {
      throw new Error('host down');
    });
    await store.load();
    expect(store.error).toBe('host down');
  });

  it('saves only the diff: deletions first, then sorted writes', async () => {
    const { store, calls } = setup((method) =>
      method === 'globals.list' ? { globals: { keep: '1', stale: 'x', change: 'old' } } : { ok: true },
    );
    await store.load();
    await store.save({ keep: '1', change: 'new', added: 'a' });
    expect(calls.map((call) => [call.method, call.params])).toEqual([
      ['globals.list', {}],
      ['globals.delete', { key: 'stale' }],
      ['globals.set', { key: 'added', value: 'a' }],
      ['globals.set', { key: 'change', value: 'new' }],
    ]);
    expect(store.globals).toEqual({ keep: '1', change: 'new', added: 'a' });
  });

  it('resyncs from the host and keeps the error when a write fails', async () => {
    const { store } = setup((method) => {
      if (method === 'globals.list') return { globals: { keep: '1' } };
      throw new Error('write failed');
    });
    await store.load();
    await expect(store.save({ keep: '2' })).rejects.toThrow('write failed');
    expect(store.error).toBe('write failed');
    expect(store.globals).toEqual({ keep: '1' });
  });
});

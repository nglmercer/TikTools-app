import { describe, expect, it } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { flushPromises } from '@vue/test-utils';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { DEFAULT_CATALOG_FILTERS } from './catalog-query.ts';
import { useCatalogStore } from './catalog-store.ts';

function item(id: string, kind = 'plugin') {
  return {
    identity: { kind, id },
    presentation: { title: id, summary: '', categories: [] },
    compatibility: { targets: [] },
    releaseRef: { type: 'pluginRelease', version: '1.0.0' },
    publication: { yanked: false },
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (failure: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function host(announced: string[], handler: (method: string, params: unknown) => unknown) {
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'rpc.discover') return { methods: announced.map((name) => ({ name })) };
    return handler(method, params);
  });
  const stores = createStoreHarness(client);
  return { stores, calls };
}

async function discover(stores: ReturnType<typeof createStoreHarness>) {
  await stores.use(() => useCapabilityStore()).refresh('host-a');
}

describe('unified catalog store', () => {
  it('does not call the catalog when the host did not announce it', async () => {
    const { stores, calls } = host(['rpc.discover'], () => { throw new Error('unexpected'); });
    await discover(stores);
    await stores.use(() => useCatalogStore()).refresh();
    expect(calls.map((call) => call.method)).toEqual(['rpc.discover']);
  });

  it('loads the first page, then appends the next one for the same query', async () => {
    const { stores, calls } = host(['catalog.items'], (_method, params) => {
      const cursor = (params as { cursor?: string }).cursor;
      return cursor
        ? { items: [item('b.two')], nextCursor: null }
        : { items: [item('a.one')], nextCursor: 'c1' };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    await catalog.refresh();
    expect(catalog.items.map((i) => i.id)).toEqual(['a.one']);
    expect(catalog.hasMore).toBe(true);
    await catalog.loadMore();
    expect(catalog.items.map((i) => i.id)).toEqual(['a.one', 'b.two']);
    expect(catalog.hasMore).toBe(false);
    expect(calls.at(-1)?.params).toMatchObject({ cursor: 'c1', limit: 25 });
  });

  it('drops a slow reply after the filters changed', async () => {
    const slow = deferred<unknown>();
    const { stores } = host(['catalog.items'], (_method, params) => {
      const text = (params as { query: string }).query;
      return text === 'old' ? slow.promise : { items: [item('new.one')], nextCursor: null };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    const stale = catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'old' });
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'new' });
    slow.resolve({ items: [item('old.one')], nextCursor: 'stale' });
    await stale;
    expect(catalog.items.map((i) => i.id)).toEqual(['new.one']);
    expect(catalog.hasMore).toBe(false);
  });

  it('resolves installed plugins one by one and keeps local-only ones labelled', async () => {
    const { stores, calls } = host(['catalog.items', 'catalog.item'], (method, params) => {
      if (method === 'catalog.item') {
        if ((params as { id: string }).id === 'b.two') return item('b.two');
        throw new ControlCallError('not_found', 'catalog item not found');
      }
      return { items: [item('a.one')], nextCursor: null };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    await catalog.setInstalled(new Set(['z.local', 'b.two']));
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, view: 'installed' });
    expect(catalog.items.map((entry) => [entry.id, entry.published])).toEqual([['b.two', true], ['z.local', false]]);
    expect(catalog.hasMore).toBe(false);
    expect(calls.filter((call) => call.method === 'catalog.items')).toHaveLength(0);
    expect(calls.filter((call) => call.method === 'catalog.item').map((call) => call.params)).toEqual([
      { kind: 'plugin', id: 'b.two' },
      { kind: 'plugin', id: 'z.local' },
    ]);
  });

  it('pages the owner library through catalog.mine and never asks it for plugins', async () => {
    const { stores, calls } = host(['catalog.items', 'catalog.mine'], (method) => {
      if (method === 'catalog.mine') return { items: [item('tpl-1', 'template')], nextCursor: null };
      return { items: [], nextCursor: null };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, view: 'mine', type: 'template' });
    expect(catalog.items.map((entry) => entry.id)).toEqual(['tpl-1']);
    expect(calls.at(-1)).toEqual({ method: 'catalog.mine', params: { query: '', limit: 25, kind: 'template' } });
    const before = calls.length;
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, view: 'mine', type: 'plugin' });
    expect(catalog.items).toEqual([]);
    expect(calls.slice(before).some((call) => call.method === 'catalog.mine')).toBe(false);
  });

  it('sends the host target only while compatibility filtering is on', async () => {
    const { stores, calls } = host(['catalog.items'], () => ({ items: [], nextCursor: null }));
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    await catalog.setTarget('windows');
    expect(calls.at(-1)?.params).toMatchObject({ target: 'windows' });
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, compatibleOnly: false });
    expect(calls.at(-1)?.params).not.toHaveProperty('target');
  });

  it('reviews plugins through the signed package path and never installs yanked ones', async () => {
    const { stores, calls } = host(['catalog.items', 'plugins.packages.inspect'], (method) => {
      if (method === 'plugins.packages.inspect') return { change: 'install', name: 'x' };
      return { items: [], nextCursor: null };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    const plugin = { kind: 'plugin', id: 'acme.gift', title: '', summary: '', publisher: null, categories: [], targets: [], version: '1.0.0', yanked: false, published: true, revision: null, sha256: null } as const;
    await catalog.install(plugin);
    expect(calls.at(-1)).toEqual({ method: 'plugins.packages.inspect', params: { spec: { kind: 'registry', id: 'acme.gift' } } });
    const before = calls.length;
    await catalog.install({ ...plugin, yanked: true });
    await catalog.install({ ...plugin, kind: 'template' });
    expect(calls.length).toBe(before);
  });

  it('imports a template only with a signed-in account and a published revision', async () => {
    const { stores, calls } = host(['catalog.items'], () => ({ items: [], nextCursor: null }));
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    const template = { kind: 'template', id: 'tpl-1', title: 'T', summary: '', publisher: null, categories: [], targets: [], version: null, yanked: false, published: true, revision: 3, sha256: 'a'.repeat(64) } as const;
    expect(await catalog.importItem(template)).toBe(false);
    expect(catalog.importState).toBe('signIn');
    expect(await catalog.importItem({ ...template, revision: null })).toBe(false);
    expect(calls.some((call) => call.method === 'cloudLibrary.read')).toBe(false);
  });

  it('never shares items or cursors between the public catalog and the owner library', async () => {
    const mine = deferred<unknown>();
    const { stores } = host(['catalog.items', 'catalog.mine'], (method) => {
      if (method === 'catalog.mine') return mine.promise;
      return { items: [item('public.one', 'template')], nextCursor: 'public-cursor' };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    await catalog.refresh();
    expect(catalog.items.map((entry) => entry.id)).toEqual(['public.one']);
    expect(catalog.hasMore).toBe(true);
    const switching = catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, view: 'mine' });
    expect(catalog.items).toEqual([]);
    expect(catalog.hasMore).toBe(false);
    mine.resolve({ items: [item('mine.one', 'template')], nextCursor: null });
    await switching;
    expect(catalog.items.map((entry) => entry.id)).toEqual(['mine.one']);
  });

  it('drops private items and ignores a late reply when the session changes, even for the same account', async () => {
    const mine = deferred<unknown>();
    const { stores } = host(['catalog.items', 'catalog.mine'], (method) => (method === 'catalog.mine' ? mine.promise : { items: [], nextCursor: null }));
    await discover(stores);
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    const catalog = stores.use(() => useCatalogStore());
    const loading = catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, view: 'mine' });
    expect(catalog.loading).toBe(true);
    account.status = { origin: 'https://cloud.example', state: 'signed-out', persistence: 'memory-only' };
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    expect(catalog.loading).toBe(false);
    mine.resolve({ items: [item('alice.private', 'template')], nextCursor: 'alice-cursor' });
    await loading; await flushPromises();
    expect(catalog.items).toEqual([]);
    expect(catalog.hasMore).toBe(false);
    expect(catalog.loading).toBe(false);
  });

  it('clears the message and the structured detail of a failure together, and ignores a late failure', async () => {
    const stale = deferred<unknown>();
    let calls = 0;
    const { stores } = host(['catalog.items'], () => {
      calls += 1;
      if (calls === 1) return stale.promise;
      if (calls === 2) throw new ControlCallError('unavailable', 'Cloud is down', { cloud: { code: 'offline', requestId: 'req-1' } });
      return { items: [], nextCursor: null };
    });
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    const first = catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'a' });
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'b' });
    expect(catalog.error).toBe('Cloud is down');
    expect(catalog.errorDetail?.code).toBe('offline');
    stale.reject(new ControlCallError('unavailable', 'older failure'));
    await first;
    expect(catalog.error).toBe('Cloud is down');
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'c' });
    expect(catalog.error).toBeNull();
    expect(catalog.errorDetail).toBeNull();
  });

  it('does not let a late detail answer reopen what was closed', async () => {
    const fresh = deferred<unknown>();
    const { stores } = host(['catalog.items', 'catalog.item'], (method) => (method === 'catalog.item' ? fresh.promise : { items: [], nextCursor: null }));
    await discover(stores);
    const catalog = stores.use(() => useCatalogStore());
    const plugin = { kind: 'plugin', id: 'acme.gift', title: '', summary: '', publisher: null, categories: [], targets: [], version: '1.0.0', yanked: false, published: true, revision: null, sha256: null } as const;
    const opening = catalog.openDetail(plugin);
    expect(catalog.detail?.id).toBe('acme.gift');
    catalog.closeDetail();
    fresh.resolve(item('acme.gift'));
    await opening;
    expect(catalog.detail).toBeNull();
  });
});

import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import ExploreToolbar from './ExploreToolbar.vue';

enableAutoUnmount(afterEach);
beforeEach(() => { vi.useFakeTimers({ shouldAdvanceTime: true }); });
afterEach(() => { vi.useRealTimers(); });

async function mountToolbar() {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'rpc.discover') return { methods: [{ name: 'catalog.items' }, { name: 'catalog.mine' }] };
    return { items: [], nextCursor: null };
  });
  const stores = createStoreHarness(client);
  await stores.use(() => useCapabilityStore()).refresh('test-host');
  const catalog = stores.use(() => useCatalogStore());
  const wrapper = mountWithPlugins(ExploreToolbar, { attachTo: document.body, global: { plugins: stores.plugins } });
  return { wrapper, calls, catalog };
}

const queries = (calls: { method: string; params?: unknown }[]) =>
  calls.filter((call) => call.method.startsWith('catalog.')).map((call) => [call.method, call.params]);

describe('ExploreToolbar', () => {
  it('has a search box and one toggle per kind and for the own library, all with accessible names', async () => {
    const { wrapper } = await mountToolbar();
    expect(wrapper.get('input[type="search"]').attributes('aria-label')).toBe('Search the Cloud catalog…');
    const group = wrapper.get('[role="group"]');
    expect(group.attributes('aria-label')).toBe('Show');
    expect(group.findAll('button').map((button) => button.attributes('aria-label'))).toEqual(['Plugins', 'Templates', 'Profiles', 'My library']);
    expect(group.findAll('button').every((button) => button.attributes('aria-pressed') === 'false')).toBe(true);
  });

  it('filters by one kind and shows everything again when the same kind is pressed twice', async () => {
    const { wrapper, calls, catalog } = await mountToolbar();
    await wrapper.get('button[aria-label="Templates"]').trigger('click');
    await flushPromises();
    expect(catalog.filters.type).toBe('template');
    expect(wrapper.get('button[aria-label="Templates"]').attributes('aria-pressed')).toBe('true');
    expect(queries(calls).at(-1)).toEqual(['catalog.items', { query: '', limit: 25, kind: 'template' }]);
    await wrapper.get('button[aria-label="Profiles"]').trigger('click');
    expect(catalog.filters.type).toBe('profile');
    expect(wrapper.get('button[aria-label="Templates"]').attributes('aria-pressed')).toBe('false');
    await wrapper.get('button[aria-label="Profiles"]').trigger('click');
    expect(catalog.filters.type).toBe('all');
  });

  it('switches to the own library with catalog.mine and back', async () => {
    const { wrapper, calls, catalog } = await mountToolbar();
    await wrapper.get('button[aria-label="My library"]').trigger('click');
    await flushPromises();
    expect(catalog.filters.view).toBe('mine');
    expect(queries(calls).at(-1)?.[0]).toBe('catalog.mine');
    await wrapper.get('button[aria-label="My library"]').trigger('click');
    await flushPromises();
    expect(catalog.filters.view).toBe('all');
    expect(queries(calls).at(-1)?.[0]).toBe('catalog.items');
  });

  it('applies the search after a short pause, once, and at once on Enter', async () => {
    const { wrapper, calls } = await mountToolbar();
    const input = wrapper.get('input[type="search"]');
    await input.setValue('t');
    await input.setValue('tts');
    expect(queries(calls)).toEqual([]);
    await vi.advanceTimersByTimeAsync(300);
    expect(queries(calls)).toEqual([['catalog.items', { query: 'tts', limit: 25 }]]);
    await input.setValue('gift');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(queries(calls).at(-1)).toEqual(['catalog.items', { query: 'gift', limit: 25 }]);
    await vi.advanceTimersByTimeAsync(300);
    expect(queries(calls)).toHaveLength(2);
  });
});

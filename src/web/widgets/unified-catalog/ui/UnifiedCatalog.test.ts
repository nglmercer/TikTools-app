import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import UnifiedCatalog from './UnifiedCatalog.vue';

enableAutoUnmount(afterEach);
afterEach(async () => { await activateLocale('en'); });

const item = (id: string, kind = 'plugin', extra: Record<string, unknown> = {}) => ({
  identity: { kind, id },
  presentation: { title: `${id} title`, summary: `${id} summary`, publisher: 'Acme', categories: [] },
  compatibility: { targets: ['windows'] },
  releaseRef: { type: 'pluginRelease', version: '1.0.0' },
  publication: { yanked: false },
  ...extra,
});

const button = (wrapper: ReturnType<typeof mountWithPlugins>, text: string) =>
  wrapper.findAll('button').find((node) => node.text() === text)!;

function mount(handler: (method: string, params: Record<string, unknown> | undefined) => unknown) {
  const { client, calls } = createFakeControlClient((method, params) => handler(method, params as Record<string, unknown> | undefined));
  const wrapper = mountWithPlugins(UnifiedCatalog, {
    props: { installedIds: new Set(['b.two']) },
    global: { plugins: storePlugins(client) },
  });
  return { wrapper, calls };
}

describe('UnifiedCatalog', () => {
  it('lists items, marks installed plugins and offers install only for installable plugins', async () => {
    const { wrapper, calls } = mount((method) => {
      if (method === 'catalog.items') {
        return {
          items: [item('a.one'), item('b.two'), item('t.one', 'template')],
          nextCursor: null,
        };
      }
      if (method === 'catalog.item') return item('a.one');
      throw new Error(`unexpected ${method}`);
    });
    await flushPromises();
    expect(calls.map((call) => call.method)).toEqual(['catalog.items']);
    const text = wrapper.text();
    expect(text).toContain('a.one title');
    expect(text).toContain('Installed');
    expect(text).toContain('Template');
    await wrapper.findAll('.unified-catalog__row')[0]!.trigger('click');
    await flushPromises();
    expect(button(wrapper, 'Install')).toBeDefined();
    expect(calls.at(-1)?.method).toBe('catalog.item');
  });

  it('opens detail for a template without offering an install', async () => {
    const { wrapper } = mount((method) => {
      if (method === 'catalog.items') return { items: [item('t.one', 'template')], nextCursor: null };
      if (method === 'catalog.item') return item('t.one', 'template');
      throw new Error(`unexpected ${method}`);
    });
    await flushPromises();
    await wrapper.findAll('.unified-catalog__row')[0]!.trigger('click');
    await flushPromises();
    expect(wrapper.find('.unified-catalog__detail').exists()).toBe(true);
    expect(wrapper.findAll('button').some((node) => node.text() === 'Install')).toBe(false);
    expect(wrapper.text()).toContain('Import template');
  });

  it('appends the next page with the cursor of the same query', async () => {
    const { wrapper, calls } = mount((method, params) => {
      if (method !== 'catalog.items') throw new Error(`unexpected ${method}`);
      return params?.cursor
        ? { items: [item('b.two')], nextCursor: null }
        : { items: [item('a.one')], nextCursor: 'page-2' };
    });
    await flushPromises();
    await button(wrapper, 'Load more').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('a.one title');
    expect(wrapper.text()).toContain('b.two title');
    expect(calls.at(-1)?.params).toMatchObject({ cursor: 'page-2', limit: 25 });
  });

  it('applies search text only after a short pause', async () => {
    const { wrapper, calls } = mount(() => ({ items: [], nextCursor: null }));
    await flushPromises();
    await wrapper.get('input[type="search"]').setValue('gift');
    await flushPromises();
    expect(calls.filter((call) => call.method === 'catalog.items')).toHaveLength(1);
    await new Promise((done) => setTimeout(done, 300));
    await flushPromises();
    expect(calls.at(-1)?.params).toMatchObject({ query: 'gift' });
  });

  it('shows the error without removing the list', async () => {
    const { wrapper } = mount((method) => {
      if (method === 'catalog.items') throw new Error('cloud catalog is unavailable');
      throw new Error(`unexpected ${method}`);
    });
    await flushPromises();
    expect(wrapper.text()).toContain('cloud catalog is unavailable');
  });
});

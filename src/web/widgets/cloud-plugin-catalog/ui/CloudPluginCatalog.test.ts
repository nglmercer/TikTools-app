import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudPluginCatalog from './CloudPluginCatalog.vue';

enableAutoUnmount(afterEach);
afterEach(async () => { await activateLocale('en'); });

const capabilities = { schemaVersion: 1, summaryPath: '/api/v2/catalog/plugin-summaries',
  versionPagePath: '/api/v2/catalog/plugins/{id}/version-pages', maxItems: 50,
  maxBytes: 524288, versionOrder: 'ascii-version-target' };
const summary = (id: string) => ({ id, latestStableVersion: '1.0.0', metadataPath: `/generation/17/${id}` });
const version = (name = '1.0.0') => ({ version: name, target: 'any', size: '9007199254740995', yanked: true,
  keyRevoked: true, publisherApproved: false, advisory: { severity: 'high', message: 'Review', url: null } });
const legacyPlugin = (id: string) => ({ id, latestStableVersion: '1.0.0', versions: [version()] });
const button = (wrapper: ReturnType<typeof mountWithPlugins>, text: string) =>
  wrapper.findAll('button').find((item) => item.text() === text)!;

describe('CloudPluginCatalog', () => {
  it('pins summary and version cursors to the exact generation and exposes status without installing', async () => {
    const { client, calls } = createFakeControlClient((method, params) => {
      if (method === 'registry.cloudCatalogPaginationCapabilities') return capabilities;
      if (method === 'registry.cloudCatalogVersions') return {
        generation: '9007199254740993', id: params?.id, metadataPath: '/generation/17/first.plugin',
        items: [version(params?.cursor ? '2.0.0' : '1.0.0')], nextCursor: params?.cursor ? null : 'version-cursor',
      };
      return { generation: '9007199254740993', items: [summary(params?.cursor ? 'second.plugin' : 'first.plugin')], nextCursor: params?.cursor ? null : 'summary-cursor' };
    });
    const wrapper = mountWithPlugins(CloudPluginCatalog, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(wrapper.text()).toContain('first.plugin');
    expect(wrapper.text()).toContain('Showing: all plugins');
    expect(wrapper.text()).toContain('does not verify a package');
    await button(wrapper, 'View versions').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Publisher key revoked');
    expect(wrapper.text()).toContain('Withdrawn');
    expect(wrapper.text()).toContain('high: Review');
    await wrapper.get('nav[aria-label="Version pages"]').findAll('button')[1]!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('2.0.0');
    expect(calls.find((call) => call.method === 'registry.cloudCatalogVersions' && call.params?.cursor)).toEqual({
      method: 'registry.cloudCatalogVersions', params: { id: 'first.plugin', generation: '9007199254740993', cursor: 'version-cursor' },
    });
    await wrapper.get('nav[aria-label="Catalog pages"]').findAll('button')[1]!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('second.plugin');
    expect(wrapper.text()).not.toContain('1.0.0 · any');
    expect(calls.find((call) => call.method === 'registry.cloudCatalogSummaries' && call.params?.cursor)).toEqual({
      method: 'registry.cloudCatalogSummaries', params: { query: '', cursor: 'summary-cursor', generation: '9007199254740993' },
    });
    await wrapper.get('nav[aria-label="Catalog pages"]').findAll('button')[0]!.trigger('click');
    await flushPromises();
    expect(calls.at(-1)).toEqual({ method: 'registry.cloudCatalogSummaries', params: { query: '', generation: '9007199254740993' } });
    expect(wrapper.text()).toContain('first.plugin');
    expect(calls.every((call) => !call.method.startsWith('plugins.packages.'))).toBe(true);
  });

  it('retains the previous page for failed or inconsistent continuation', async () => {
    let continuation = 0;
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'registry.cloudCatalogPaginationCapabilities') return capabilities;
      if (!params?.cursor) return { generation: '17', items: [summary('safe.plugin')], nextCursor: 'safe-cursor' };
      continuation += 1;
      if (continuation === 1) throw new Error('offline');
      if (continuation === 2) return { generation: '18', items: [summary('stale.plugin')], nextCursor: null };
      return { generation: '17', items: [{ id: 'broken.plugin' }], nextCursor: null };
    });
    const wrapper = mountWithPlugins(CloudPluginCatalog, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    const next = () => wrapper.get('nav[aria-label="Catalog pages"]').findAll('button')[1]!;
    for (const message of ['offline', 'could not be verified', 'could not be verified']) {
      await next().trigger('click');
      await flushPromises();
      expect(wrapper.get('[role="alert"]').text()).toContain(message);
      expect(wrapper.text()).toContain('safe.plugin');
      expect(wrapper.text()).not.toContain('stale.plugin');
      expect(wrapper.text()).not.toContain('broken.plugin');
    }
  });

  it('ignores old version and summary results when the search changes', async () => {
    await activateLocale('es');
    let resolveOld!: (value: unknown) => void;
    const old = new Promise((resolve) => { resolveOld = resolve; });
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'registry.cloudCatalogPaginationCapabilities') return capabilities;
      if (method === 'registry.cloudCatalogVersions') return old;
      return { generation: params?.query ? '23' : '17', items: [summary('same.plugin')], nextCursor: null };
    });
    const wrapper = mountWithPlugins(CloudPluginCatalog, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    await button(wrapper, 'Ver versiones').trigger('click');
    await wrapper.get('input').setValue('new');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    resolveOld({ generation: '17', id: 'same.plugin', metadataPath: '/generation/17/same.plugin', items: [version()], nextCursor: null });
    await flushPromises();
    expect(wrapper.text()).toContain('Mostrando: new');
    expect(wrapper.text()).not.toContain('Clave del publicador revocada');
  });

  it('discards versions opened on the old page while a new summary is in flight', async () => {
    let resolveSummary!: (value: unknown) => void;
    let resolveVersions!: (value: unknown) => void;
    const pendingSummary = new Promise((resolve) => { resolveSummary = resolve; });
    const pendingVersions = new Promise((resolve) => { resolveVersions = resolve; });
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'registry.cloudCatalogPaginationCapabilities') return capabilities;
      if (method === 'registry.cloudCatalogVersions') return pendingVersions;
      if (params?.query) return pendingSummary;
      return { generation: '17', items: [summary('same.plugin')], nextCursor: null };
    });
    const wrapper = mountWithPlugins(CloudPluginCatalog, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.get('input').setValue('new');
    await wrapper.get('form').trigger('submit');
    await button(wrapper, 'View versions').trigger('click');
    resolveSummary({ generation: '23', items: [summary('same.plugin')], nextCursor: null });
    await flushPromises();
    resolveVersions({ generation: '17', id: 'same.plugin', metadataPath: '/generation/17/same.plugin', items: [version()], nextCursor: null });
    await flushPromises();
    expect(wrapper.text()).toContain('Showing: new');
    expect(wrapper.text()).not.toContain('Publisher key revoked');
    expect(button(wrapper, 'View versions').exists()).toBe(true);
  });

  it('offers legacy mode only for the explicit unsupported capability response', async () => {
    const unsupported = new ControlCallError('capability_unavailable', 'unsupported', { reason: 'bounded_catalog_unsupported' });
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'registry.cloudCatalogPaginationCapabilities') throw unsupported;
      return { generation: '17', items: [legacyPlugin('legacy.plugin')], nextCursor: null };
    });
    const wrapper = mountWithPlugins(CloudPluginCatalog, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(calls.map((call) => call.method)).toEqual(['registry.cloudCatalogPaginationCapabilities']);
    await button(wrapper, 'Use older catalog').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('legacy.plugin');
    expect(wrapper.text()).toContain('Publisher key revoked');
    expect(calls[1]).toEqual({ method: 'registry.cloudCatalog', params: { query: '' } });
  });

  it('does not downgrade on network or malformed capability errors', async () => {
    for (const failure of [new ControlCallError('unavailable', 'offline'), { schemaVersion: 1 }]) {
      const { client, calls } = createFakeControlClient(() => {
        if (failure instanceof Error) throw failure;
        return failure;
      });
      const wrapper = mountWithPlugins(CloudPluginCatalog, { global: { plugins: storePlugins(client) } });
      await flushPromises();
      expect(wrapper.find('[role="alert"]').exists()).toBe(true);
      expect(wrapper.text()).not.toContain('Use older catalog');
      expect(calls.map((call) => call.method)).toEqual(['registry.cloudCatalogPaginationCapabilities']);
      wrapper.unmount();
    }
  });
});

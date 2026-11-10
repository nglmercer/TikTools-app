import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginVersionsPane from './PluginVersionsPane.vue';

enableAutoUnmount(afterEach);
afterEach(async () => { await activateLocale('en'); });

function setup(kind: 'registry' | 'file') {
  const origin = kind === 'registry' ? { kind, registry: 'old-source', url: 'https://old.example' } : { kind };
  const receipt = { id:'acme.plugin', version:'2.0.0', origin, signature:{ status:'verified' }, runtime:'process',
    grantedPermissions:[], grantedCapabilities:[], installedAt:1, updatedAt:2,
    policy:{ channel:'stable', autoUpdate:'notify', pinned:false }, retained:[{ version:'1.0.0', retainedAt:1 }] };
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'plugins.packages.versions') return { id:receipt.id, installed:receipt.version, receipt, retained:receipt.retained, registry:'old-source', available:[] };
    if (method === 'plugins.packages.inspect') return { id:receipt.id, version:'1.0.0', consentToken:'review-token' };
    if (method === 'plugins.packages.install') return { job:{ id:'install-job', kind:'install', stage:'queued', status:'queued', createdAt:1, updatedAt:1 } };
    if (method === 'plugins.packages.rollback') return { job:{ id:'job', kind:'rollback', stage:'queued', status:'queued', createdAt:1, updatedAt:1 } };
    return {};
  });
  const stores = createStoreHarness(client);
  const store = stores.use(() => usePackagesStore());
  const wrapper = mountWithPlugins(PluginVersionsPane, { props:{ pluginId:receipt.id }, global:{ plugins:stores.plugins } });
  return { wrapper, calls, store };
}

it('reviews a previous Cloud version from the configured instance before installation', async () => {
  await activateLocale('en');
  const { wrapper, calls, store } = setup('registry');
  await flushPromises();
  const button = wrapper.findAll('button').find((item) => item.text() === 'Review Cloud version');
  expect(button).toBeDefined();
  expect(wrapper.text()).toContain('require a new review');
  await button!.trigger('click');
  await flushPromises();
  expect(calls.filter((call) => call.method === 'plugins.packages.inspect')).toEqual([
    { method:'plugins.packages.inspect', params:{ spec:{ kind:'registry', id:'acme.plugin', version:'1.0.0' } } },
  ]);
  expect(calls.some((call) => ['plugins.packages.install', 'plugins.packages.rollback'].includes(call.method))).toBe(false);
  expect(store.review.open).toBe(true);
  await store.confirmReview();
  expect(calls.find((call) => call.method === 'plugins.packages.install')).toEqual({
    method:'plugins.packages.install', params:{ spec:{ kind:'registry', id:'acme.plugin', version:'1.0.0' }, consentToken:'review-token' },
  });
});

it('keeps verified local restoration available for local packages', async () => {
  await activateLocale('en');
  const { wrapper, calls } = setup('file');
  await flushPromises();
  expect(wrapper.text()).not.toContain('Review Cloud version');
  await wrapper.findAll('button').find((item) => item.text() === 'Restore')!.trigger('click');
  await flushPromises();
  expect(calls.find((call) => call.method === 'plugins.packages.rollback')).toEqual({
    method:'plugins.packages.rollback', params:{ pluginId:'acme.plugin', version:'1.0.0' },
  });
});

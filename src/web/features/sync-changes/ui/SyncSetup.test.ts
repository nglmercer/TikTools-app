import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import SyncSetup from './SyncSetup.vue';

enableAutoUnmount(afterEach);
it('retains the migration blocker and never migrates without a successful review', async () => {
  let enabled = false;
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'cloud.status') return { state: 'signed-in', accountId: 'tester', persistence: 'memory-only', origin: 'https://cloud.example' };
    if (method === 'sync.enable') enabled = true;
    if (method === 'sync.status' || method === 'sync.enable') return { enabled, signedIn: true, syncing: false, pending: 1, conflicts: [], lastError: null };
    if (method === 'sync.v2.selected') return false;
    if (method === 'sync.v2.discover') return { state: 'backendCompatible', selected: false, desktopSyncAvailable: false };
    if (method === 'sync.v2.preview') throw new Error('Cloud migration review blocked: installed_plugin_missing');
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(SyncSetup, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  expect(calls.some(call => call.method === 'sync.enable')).toBe(false);
  await wrapper.findAll('button').find(button => button.text() === 'Prepare Cloud sync')!.trigger('click');
  await flushPromises();
  expect(wrapper.text()).toContain('installed_plugin_missing');
  expect(calls.some(call => call.method === 'sync.v2.migrate')).toBe(false);
  expect(wrapper.emitted('ready')).toBeUndefined();
});


it('requires explicit confirmation of the reviewed inventory and rechecks migration afterwards', async () => {
  const review = 'a'.repeat(64);
  let repaired = false;
  const { client, calls } = createFakeControlClient(method => {
    if (method === 'cloud.status') return { state: 'signed-in', accountId: 'tester', persistence: 'memory-only', origin: 'https://cloud.example' };
    if (method === 'sync.status' || method === 'sync.enable') return { enabled: true, signedIn: true, syncing: false, pending: 1, conflicts: ['legacy.plugin'], lastError: null };
    if (method === 'sync.v2.selected') return false;
    if (method === 'sync.v2.discover') return { state: 'backendCompatible', selected: false, desktopSyncAvailable: false };
    if (method === 'sync.v2.preview') {
      if (!repaired) throw new Error('Cloud migration review blocked: installed_plugin_missing');
      return { selections: 1, pending: 1, conflicts: 1, retainedV1Only: 0, migrationAvailable: true };
    }
    if (method === 'sync.v2.inventory.review') return { review, plugins: ['missing.plugin'] };
    if (method === 'sync.v2.inventory.repair') { repaired = true; return { review, plugins: ['missing.plugin'] }; }
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(SyncSetup, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  const click = async (label: string) => { await wrapper.findAll('button').find(button => button.text() === label)!.trigger('click'); await flushPromises(); };
  await click('Prepare Cloud sync');
  await click('Review missing packages');
  expect(wrapper.text()).toContain('missing.plugin');
  expect(wrapper.text()).toContain('pending changes are kept');
  expect(calls.some(call => call.method === 'sync.v2.inventory.repair')).toBe(false);
  await click('Confirm local repair');
  expect(calls.find(call => call.method === 'sync.v2.inventory.repair')?.params).toEqual({ review });
  expect(calls.filter(call => call.method === 'sync.v2.preview')).toHaveLength(2);
  expect(calls.some(call => call.method === 'sync.v2.migrate')).toBe(false);
});

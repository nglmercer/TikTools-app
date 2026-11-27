import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import SyncSetup from '../../../../../../src/web/features/sync-changes/ui/SyncSetup.vue';
import { ACCOUNT_STATE, RPC, defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);
it('retains the migration blocker and never migrates without a successful review', async () => {
  let enabled = false;
  const { client, calls } = createFakeControlClient((method) => {
    if (method === RPC.cloudStatus) return {
      state: ACCOUNT_STATE.signedIn,
      accountId: 'tester',
      persistence: ACCOUNT_STATE.memoryOnly,
      origin: 'https://cloud.example',
    };
    if (method === 'sync.enable') enabled = true;
    if (method === RPC.syncStatus || method === 'sync.enable') return { enabled, signedIn: true, syncing: false, pending: 1, conflicts: [], lastError: null };
    if (method === 'sync.v2.selected') return false;
    if (method === 'sync.v2.discover') return { state: 'backendCompatible', selected: false, desktopSyncAvailable: false };
    if (method === 'sync.v2.preview') throw new Error('Cloud migration review blocked: installed_plugin_missing');
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(SyncSetup, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  expect(calls.some(call => call.method === 'sync.enable')).toBe(false);
  await defined(wrapper.findAll('button').find(button => button.text() === t('syncChanges.prepare'))).trigger('click');
  await flushPromises();
  expect(wrapper.text()).toContain('installed_plugin_missing');
  expect(calls.some(call => call.method === 'sync.v2.migrate')).toBe(false);
  expect(wrapper.emitted('ready')).toBeUndefined();
});


it('requires explicit confirmation of the reviewed inventory and rechecks migration afterwards', async () => {
  // Arrange: migration is blocked by a missing plugin; the repair must reuse the reviewed inventory digest
  const review = 'a'.repeat(64);
  let repaired = false;
  const { client, calls } = createFakeControlClient(method => {
    if (method === RPC.cloudStatus) return {
      state: ACCOUNT_STATE.signedIn,
      accountId: 'tester',
      persistence: ACCOUNT_STATE.memoryOnly,
      origin: 'https://cloud.example',
    };
    if (method === RPC.syncStatus || method === 'sync.enable') return {
      enabled: true,
      signedIn: true,
      syncing: false,
      pending: 1,
      conflicts: ['legacy.plugin'],
      lastError: null,
    };
    if (method === 'sync.v2.selected') return false;
    if (method === 'sync.v2.discover') return { state: 'backendCompatible', selected: false, desktopSyncAvailable: false };
    if (method === 'sync.v2.preview') {
      if (!repaired) throw new Error('Cloud migration review blocked: installed_plugin_missing');
      return { selections: 1, pending: 1, conflicts: 1, retainedV1Only: 0, migrationAvailable: true };
    }
    if (method === 'sync.v2.inventory.review') return { review, plugins: ['missing.plugin'] };
    if (method === 'sync.v2.inventory.repair') {
      repaired = true;
      return { review, plugins: ['missing.plugin'] };
    }
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(SyncSetup, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  const click = async (label: string) => {
    await defined(wrapper.findAll('button').find(button => button.text() === label)).trigger('click');
    await flushPromises();
  };
  await click(t('syncChanges.prepare'));
  await click(t('syncChanges.inventoryReview'));
  // Assert: the reviewed inventory lists the missing plugin and promises that pending changes are kept
  expect(wrapper.text()).toContain('missing.plugin');
  expect(wrapper.text()).toContain('pending changes are kept');
  expect(calls.some(call => call.method === 'sync.v2.inventory.repair')).toBe(false);
  // Act: the repair runs only on an explicit click, and it sends back the exact digest the user reviewed
  await click(t('syncChanges.inventoryRepair'));
  expect(calls.find(call => call.method === 'sync.v2.inventory.repair')?.params).toEqual({ review });
  // Assert: migration is rechecked after the repair, and it is still never started automatically
  expect(calls.filter(call => call.method === 'sync.v2.preview')).toHaveLength(2);
  expect(calls.some(call => call.method === 'sync.v2.migrate')).toBe(false);
});

import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import type { AccountStatus } from '@/entities/cloud-account/api/account-types.ts';
import type { SyncStatus } from '@/entities/cloud-sync/api/sync-types.ts';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { allCapabilities, fakeAdapter } from '../testing/fake-adapter.ts';
import SyncButton from './SyncButton.vue';

enableAutoUnmount(afterEach);

const account = (state: AccountStatus['state']): AccountStatus => ({ origin: 'http://127.0.0.1:3000', state, accountId: 'tester', persistence: 'memory-only', error: null });
const status = (enabled: boolean): SyncStatus => ({ enabled, signedIn: true, syncing: false, pending: 0, conflicts: [], lastError: null, lastSyncedAt: null });

async function mount(adapter: SyncAdapter, accountState: AccountStatus['state'] = 'signed-in', enabled = true) {
  const { client } = createFakeControlClient((method) => {
    if (method === 'cloud.status') return account(accountState);
    if (method === 'sync.status') return status(enabled);
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(SyncButton, { attachTo: document.body, props: { adapter }, global: { plugins: storePlugins(client) } });
  await flushPromises();
  return wrapper;
}

describe('SyncButton', () => {
  it('shows the pending count in its badge and accessible name, and opens the panel', async () => {
    const { adapter } = fakeAdapter(allCapabilities);
    const wrapper = await mount(adapter);
    expect(wrapper.get('.sync-button__badge').text()).toBe('4');
    const button = wrapper.get('button');
    expect(button.attributes('aria-label')).toBe('Sync changes, 4 pending');
    expect(button.attributes('disabled')).toBeUndefined();
    await button.trigger('click');
    await flushPromises();
    expect(wrapper.get('dialog').attributes('open')).toBeDefined();
    expect(wrapper.get('h2').text()).toBe('Changes · Events');
  });

  it('has no badge when nothing is pending', async () => {
    const { adapter } = fakeAdapter(allCapabilities, []);
    const wrapper = await mount(adapter);
    expect(wrapper.find('.sync-button__badge').exists()).toBe(false);
    expect(wrapper.get('button').attributes('aria-label')).toBe('Sync changes');
  });

  it('is disabled with the reason when the account is signed out', async () => {
    const { adapter, spies } = fakeAdapter(allCapabilities);
    delete (adapter as { requiresCloud?: boolean }).requiresCloud;
    const wrapper = await mount(adapter, 'signed-out');
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
    expect(wrapper.find('.sync-button__badge').exists()).toBe(false);
    expect(wrapper.find('dialog').exists()).toBe(false);
    expect(spies.list).not.toHaveBeenCalled();
  });

  it('is disabled when Cloud sync is not enabled', async () => {
    const { adapter } = fakeAdapter(allCapabilities);
    delete (adapter as { requiresCloud?: boolean }).requiresCloud;
    const wrapper = await mount(adapter, 'signed-in', false);
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
  });

  it('is enabled once signed in and opted in', async () => {
    const { adapter } = fakeAdapter(allCapabilities);
    delete (adapter as { requiresCloud?: boolean }).requiresCloud;
    const wrapper = await mount(adapter);
    expect(wrapper.get('button').attributes('disabled')).toBeUndefined();
    expect(wrapper.get('.sync-button__badge').text()).toBe('4');
  });

  it('is disabled with the adapter reason when the host cannot sync this section', async () => {
    const { adapter } = fakeAdapter(allCapabilities);
    delete (adapter as { requiresCloud?: boolean }).requiresCloud;
    (adapter as { unavailableReason?: () => string | null }).unavailableReason = () => 'Not on this host';
    const wrapper = await mount(adapter);
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
  });
});

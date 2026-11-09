import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { AccountStatus } from '@/entities/cloud-account/api/account-types.ts';
import type { SyncStatus } from '@/entities/cloud-sync/api/sync-types.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudSyncEditor from './CloudSyncEditor.vue';

function account(overrides: Partial<AccountStatus> = {}): AccountStatus {
  return {
    origin: 'http://127.0.0.1:3000',
    state: 'signed-in',
    accountId: 'tester',
    persistence: 'memory-only',
    error: null,
    ...overrides,
  };
}

function status(overrides: Partial<SyncStatus> = {}): SyncStatus {
  return {
    enabled: false,
    signedIn: true,
    syncing: false,
    pending: 0,
    conflicts: [],
    lastError: null,
    lastSyncedAt: null,
    ...overrides,
  };
}

function setup(accountStatus: AccountStatus, syncStatus: SyncStatus) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'cloud.status') return account(accountStatus);
    if (method === 'sync.status') return status(syncStatus);
    if (method === 'sync.enable') return status({ ...syncStatus, enabled: true });
    if (method === 'sync.disable') return status({ ...syncStatus, enabled: false });
    if (method === 'sync.uncertain.list') return [];
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudSyncEditor, {
    global: { plugins: storePlugins(client) },
  });
  return { wrapper, calls };
}

describe('CloudSyncEditor', () => {
  it('renders nothing until the cloud account is signed in', async () => {
    const { wrapper } = setup(account({ state: 'signed-out' }), status());
    await flushPromises();
    expect(wrapper.html()).not.toContain('input');
  });

  it('shows a switch that toggles sync once signed in', async () => {
    const { wrapper, calls } = setup(account({}), status({ enabled: false }));
    await flushPromises();
    const input = wrapper.get('input[role="switch"]');
    expect((input.element as HTMLInputElement).checked).toBe(false);
    await input.setValue(true);
    await flushPromises();
    expect(calls.map((call) => call.method)).toContain('sync.enable');
    expect(wrapper.text()).toContain('Sync now');
  });

  it('shows the review but disables decisions until local changes are protected', async () => {
    const selection = { state: 'selected', version: '1.0.0', enabled: true };
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.status') return status({ enabled: true });
      if (method === 'sync.uncertain.list') return ['acme.plugin'];
      if (method === 'sync.uncertain.review') return { handle: 'a'.repeat(64), key: 'acme.plugin', base: selection, original: selection, local: selection, cloud: selection };
      throw new Error(`unexpected ${method}`);
    });
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    const button = (label: string) => wrapper.findAll('button').find((item) => item.text() === label)!;
    await button('Review').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Original base');
    expect(wrapper.text()).toContain('Version 1.0.0, enabled');
    expect(wrapper.text()).toContain('Choosing a resolution is temporarily unavailable');
    expect(button('Use cloud').attributes('disabled')).toBeDefined();
    expect(button('Keep mine').attributes('disabled')).toBeDefined();
    expect(calls.filter((call) => call.method === 'sync.uncertain.resolve')).toHaveLength(0);
  });

  it('rejects an incomplete recovery review instead of offering a decision', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.status') return status({ enabled: true });
      if (method === 'sync.uncertain.list') return ['acme.plugin'];
      if (method === 'sync.uncertain.review') return { handle: 'a'.repeat(64), key: 'acme.plugin' };
      throw new Error(`unexpected ${method}`);
    });
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.findAll('button').find((item) => item.text() === 'Review')!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('The recovery response was invalid.');
    expect(wrapper.text()).not.toContain('Original base');
    expect(wrapper.text()).not.toContain('Use cloud');
  });
});

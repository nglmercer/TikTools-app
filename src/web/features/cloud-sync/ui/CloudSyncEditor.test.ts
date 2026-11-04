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
});

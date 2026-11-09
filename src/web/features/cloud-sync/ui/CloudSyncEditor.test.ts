import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { AccountStatus } from '@/entities/cloud-account/api/account-types.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import type { SyncStatus } from '@/entities/cloud-sync/api/sync-types.ts';
import { createFakeControlClient, createStoreHarness, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
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
    if (method === 'sync.v2.selected') return false;
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
  it('reports v2 server compatibility without enabling v2', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.v2.selected') return false;
    if (method === 'sync.status') return status({ enabled: true });
      if (method === 'sync.uncertain.list') return [];
      if (method === 'sync.v2.discover') return { state: 'backendCompatible', desktopSyncAvailable: false };
      throw new Error(`unexpected ${method}`);
    });
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(calls.some((call) => call.method === 'sync.v2.discover')).toBe(false);
    await wrapper.findAll('button').find((item) => item.text() === 'Check Cloud v2 compatibility')!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('The server supports v2. Migration in this app is not available yet.');
    expect(calls.filter((call) => call.method === 'sync.v2.discover')).toHaveLength(1);
    expect(calls.some((call) => call.method === 'sync.enable' || call.method === 'sync.now')).toBe(false);
    wrapper.unmount();
  });

  it('uses v2 sync only for the selected account and reports cached records', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.v2.selected') return true;
    if (method === 'sync.status') return status({ enabled: true });
      if (method === 'sync.uncertain.list') return [];
      if (method === 'sync.v2.discover') return { state: 'backendCompatible', desktopSyncAvailable: false, selected: true };
      if (method === 'sync.v2.now') return { pushed: 1, pulled: 2, workspacePending: 0, availableRecords: 2, conflicts: [], moreQueued: true };
      throw new Error(`unexpected ${method}`);
    });
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(wrapper.text()).toContain('Cloud v2 is selected for this account.');
    await wrapper.findAll('button').find((item) => item.text() === 'Sync now')!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Sent: 1. Downloaded: 2.');
    expect(wrapper.text()).toContain('More changes are queued');
    expect(calls.filter((call) => call.method === 'sync.v2.now')).toHaveLength(1);
    expect(calls.some((call) => call.method === 'sync.now')).toBe(false);
    wrapper.unmount();
  });

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

  it('explains deferred application and does not offer unsafe conflict decisions', async () => {
    let gap!: () => void;
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.v2.selected') return false;
    if (method === 'sync.status') return status({ enabled: true, conflicts: ['acme.plugin'] });
      if (method === 'sync.uncertain.list') return [];
      if (method === 'sync.now') return { pushed: 0, pulled: 1, applied: 0, skipped: 2, applicationDeferred: true, conflicts: ['acme.plugin'] };
      throw new Error(`unexpected ${method}`);
    });
    client.onGap = (handler) => { gap = () => handler(1); return () => {}; };
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(wrapper.text()).toContain('not applied to this device automatically');
    expect(wrapper.text()).toContain('Local changes are not guaranteed to enter the upload queue');
    expect(wrapper.text()).toContain('Your pending changes are retained');
    for (const label of ['Keep mine', 'Use cloud']) {
      expect(wrapper.findAll('button').find((item) => item.text() === label)?.attributes('disabled')).toBeDefined();
    }
    await wrapper.findAll('button').find((item) => item.text() === 'Sync now')!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('2 Cloud selection record(s) are available without automatic application');
    expect(wrapper.text()).toContain('some may already match this device');
    expect(calls.filter((call) => call.method === 'sync.resolve')).toHaveLength(0);
    gap(); await flushPromises();
    expect(wrapper.text()).not.toContain('2 Cloud selection record(s)');
    wrapper.unmount();
  });

  it('discards a late sync outcome and review on Cloud context invalidation', async () => {
    let release!: (value: unknown) => void;
    let invalidate!: () => void;
    const pending = new Promise((resolve) => { release = resolve; });
    const { client } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.v2.selected') return false;
    if (method === 'sync.status') return status({ enabled: true });
      if (method === 'sync.uncertain.list') return [];
      if (method === 'sync.now') return pending;
      throw new Error(`unexpected ${method}`);
    });
    client.onTopic = (topic, handler) => { if (topic === 'cloud.context.invalidated') invalidate = () => (handler as (value: unknown) => void)({}); return () => {}; };
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.findAll('button').find((item) => item.text() === 'Sync now')!.trigger('click');
    invalidate();
    release({ pushed: 0, pulled: 1, applied: 0, skipped: 2, applicationDeferred: true, conflicts: [] });
    await flushPromises();
    expect(wrapper.text()).not.toContain('2 Cloud selection record(s)');
    wrapper.unmount();
  });

  it('shows the review but disables decisions until local changes are protected', async () => {
    const selection = { state: 'selected', version: '1.0.0', enabled: true };
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.v2.selected') return false;
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
      if (method === 'sync.v2.selected') return false;
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

  it('discards a late recovery review when the same account signs in again', async () => {
    let resolveReview!: (value: unknown) => void;
    const pending = new Promise((resolve) => { resolveReview = resolve; });
    const { client } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return account();
      if (method === 'sync.v2.selected') return false;
    if (method === 'sync.status') return status({ enabled: true });
      if (method === 'sync.uncertain.list') return ['acme.plugin'];
      if (method === 'sync.uncertain.review') return pending;
      throw new Error(`unexpected ${method}`);
    });
    const stores = createStoreHarness(client);
    const cloudAccount = stores.use(() => useAccountStore());
    const wrapper = mountWithPlugins(CloudSyncEditor, { global: { plugins: stores.plugins } });
    await flushPromises();
    await wrapper.findAll('button').find((item) => item.text() === 'Review')!.trigger('click');
    cloudAccount.working = true;
    cloudAccount.status = account({ state: 'signed-out', accountId: null });
    cloudAccount.status = account();
    cloudAccount.working = false;
    const selection = { state: 'selected', version: '1.0.0', enabled: true };
    resolveReview({ handle: 'a'.repeat(64), key: 'acme.plugin', base: selection, original: selection, local: selection, cloud: selection });
    await flushPromises();
    expect(wrapper.text()).not.toContain('Original base');
    expect(wrapper.text()).not.toContain('Use cloud');
    wrapper.unmount();
  });
});

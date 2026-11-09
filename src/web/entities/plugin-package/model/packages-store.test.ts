import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import type { JobSnapshot } from '../api/package-types.ts';
import { usePackagesStore } from './packages-store.ts';

function harness(handler: (method: string, params: RpcParams | undefined) => unknown) {
  const calls: { method: string; params: RpcParams | undefined }[] = [];
  const topics = new Map<string, (data: unknown) => void>();
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async (method: string, params?: RpcParams) => {
      calls.push({ method, params });
      return handler(method, params);
    }) as ControlClient['call'],
    onTopic: ((topic: string, listener: (data: unknown) => void) => {
      topics.set(topic, listener);
      return () => topics.delete(topic);
    }) as ControlClient['onTopic'],
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  const stores = createStoreHarness(client);
  const store = stores.use(() => usePackagesStore());
  return { store, calls, topics };
}

function job(overrides: Partial<JobSnapshot>): JobSnapshot {
  return { id: 'job-1', kind: 'update', pluginId: 'acme.tts', stage: 'queued', status: 'queued', createdAt: 1, updatedAt: 1, ...overrides };
}

const preview = {
  id: 'acme.tts',
  name: 'Acme TTS',
  version: '1.1.0',
  consentToken: 'token-2',
  permissionDiff: { addedPermissions: ['http'], removedPermissions: [], addedCapabilities: [], removedCapabilities: [] },
};

describe('packages store', () => {
  it('updates only releases with a currently verified Cloud status', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'plugins.updates.list') return { checkedAt: 1, updates: [
        { id:'allowed', latest:'2.0.0', latestStatus:'approved' },
        { id:'revoked', latest:'2.0.0', latestStatus:'key-revoked' },
        { id:'unknown', latest:'2.0.0' },
      ] };
      if (method === 'plugins.packages.update') return { job: job({}) };
      return { receipts: [], jobs: [] };
    });
    await store.initialize();
    await store.updateAll();
    expect(calls.filter((call) => call.method === 'plugins.packages.update').map((call) => call.params)).toEqual([
      { pluginId:'allowed', version:'2.0.0' },
    ]);
  });

  it('invalidates old approvals while refreshing and ignores late results', async () => {
    let calls = 0;
    let releaseOld!: (value: unknown) => void;
    const { store } = harness((method) => {
      if (method !== 'registry.package') return {};
      calls += 1;
      if (calls === 1) return { registry:'official', document:{ id:'acme.tts', name:'Acme', releases:[] }, liveStatuses:{ '1.0.0':'approved' } };
      if (calls === 2) return new Promise((resolve) => { releaseOld = resolve; });
      throw new Error('offline');
    });
    await store.loadRemotePackage('official', 'acme.tts');
    expect(store.remotePackages['official/acme.tts']?.liveStatuses?.['1.0.0']).toBe('approved');
    const old = store.loadRemotePackage('official', 'acme.tts');
    expect(store.remotePackages['official/acme.tts']?.liveStatuses).toEqual({});
    await store.loadRemotePackage('official', 'acme.tts');
    releaseOld({ registry:'official', document:{ id:'acme.tts', name:'Acme', releases:[] }, liveStatuses:{ '1.0.0':'approved' } });
    await old;
    expect(store.remotePackages['official/acme.tts']?.liveStatuses).toEqual({});
    expect(store.remoteErrors['official/acme.tts']).toBe('offline');
  });

  it('keeps update actions disabled after a failed current-status refresh', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'plugins.updates.list') return { checkedAt: 1, updates: [{ id:'acme.tts', latest:'2.0.0', latestStatus:'approved' }] };
      if (method === 'plugins.updates.check') throw new Error('offline');
      return { receipts: [], jobs: [] };
    });
    await store.initialize();
    await store.checkUpdates();
    expect(store.updates?.updates[0]?.latestStatus).toBe('unknown');
    await store.updateAll();
    expect(calls.some((call) => call.method === 'plugins.packages.update')).toBe(false);
  });

  it('keeps an old list from superseding a newer update check', async () => {
    let finishList!: (value: unknown) => void;
    let finishCheck!: (value: unknown) => void;
    const { store } = harness((method) => {
      if (method === 'plugins.updates.list') return new Promise((resolve) => { finishList = resolve; });
      if (method === 'plugins.updates.check') return new Promise((resolve) => { finishCheck = resolve; });
      return { receipts: [], jobs: [] };
    });
    const initializing = store.initialize();
    const checking = store.checkUpdates();
    // A second initialization while the check is pending must not supersede it.
    await store.initialize();
    expect(store.checkingUpdates).toBe(true);
    finishCheck({ checkedAt: 2, updates: [{ id:'fresh', latest:'2.0.0', latestStatus:'approved' }] });
    await checking;
    finishList({ checkedAt: 1, updates: [{ id:'stale', latest:'1.0.0', latestStatus:'approved' }] });
    await initializing;
    expect(store.checkingUpdates).toBe(false);
    expect(store.updates?.updates.map((entry) => entry.id)).toEqual(['fresh']);
  });

  it('does not start a second update check or discard the first result', async () => {
    let finishCheck!: (value: unknown) => void;
    const { store, calls } = harness((method) => {
      if (method === 'plugins.updates.check') return new Promise((resolve) => { finishCheck = resolve; });
      return { receipts: [], jobs: [], updates: [], checkedAt: 0 };
    });
    await store.initialize();
    const first = store.checkUpdates();
    expect(store.checkingUpdates).toBe(true);
    await store.checkUpdates();
    expect(calls.filter((call) => call.method === 'plugins.updates.check')).toHaveLength(1);
    expect(store.checkingUpdates).toBe(true);
    finishCheck({ checkedAt: 2, updates: [{ id:'fresh', latest:'2.0.0', latestStatus:'approved' }] });
    await first;
    expect(store.checkingUpdates).toBe(false);
    expect(store.updates?.updates.map((entry) => entry.id)).toEqual(['fresh']);
    expect(store.updatesError).toBe('');
  });

  it('reloads current updates for notifications with equal or earlier timestamps', async () => {
    let lists = 0;
    let finishFirst!: (value: unknown) => void;
    let finishSecond!: (value: unknown) => void;
    const report = (checkedAt: number, latestStatus: string) => ({ checkedAt, updates: [{ id:'acme.tts', latest:'2.0.0', latestStatus }] });
    const { store, topics } = harness((method) => {
      if (method === 'plugins.updates.list') {
        lists += 1;
        if (lists === 1) return report(10, 'approved');
        return new Promise((resolve) => { if (lists === 2) finishFirst = resolve; else finishSecond = resolve; });
      }
      return { receipts: [], jobs: [] };
    });
    await store.initialize();
    topics.get('plugin.updates')?.({ report: report(10, 'approved') });
    expect(store.updates?.updates[0]?.latestStatus).toBe('unknown');
    topics.get('plugin.updates')?.({ report: report(9, 'approved') });
    expect(lists).toBe(3);
    finishSecond(report(9, 'yanked'));
    await flushPromises();
    expect(store.updates?.updates[0]?.latestStatus).toBe('yanked');
    finishFirst(report(10, 'approved'));
    await flushPromises();
    expect(store.updates?.updates[0]?.latestStatus).toBe('yanked');
  });

  it('keeps an active check authoritative when an older notification arrives', async () => {
    let finishCheck!: (value: unknown) => void;
    let current = 'approved';
    const report = (latestStatus: string) => ({ checkedAt: 1, updates: [{ id:'acme.tts', latest:'2.0.0', latestStatus }] });
    const { store, topics, calls } = harness((method) => {
      if (method === 'plugins.updates.list') return report(current);
      if (method === 'plugins.updates.check') return new Promise((resolve) => { finishCheck = resolve; });
      return { receipts: [], jobs: [] };
    });
    await store.initialize();
    const checking = store.checkUpdates();
    topics.get('plugin.updates')?.({ report: report('approved') });
    expect(calls.filter((call) => call.method === 'plugins.updates.list')).toHaveLength(1);
    expect(store.updates?.updates[0]?.latestStatus).toBe('unknown');
    current = 'yanked';
    finishCheck(report('yanked'));
    await checking;
    await flushPromises();
    expect(calls.filter((call) => call.method === 'plugins.updates.list')).toHaveLength(2);
    expect(store.updates?.updates[0]?.latestStatus).toBe('yanked');
  });

  it('reloads an event received while another window makes the check busy', async () => {
    let failCheck!: (reason: Error) => void;
    let current = 'approved';
    const report = (latestStatus: string) => ({ checkedAt: 1, updates: [{ id:'acme.tts', latest:'2.0.0', latestStatus }] });
    const { store, topics, calls } = harness((method) => {
      if (method === 'plugins.updates.list') return report(current);
      if (method === 'plugins.updates.check') return new Promise((_resolve, reject) => { failCheck = reject; });
      return { receipts: [], jobs: [] };
    });
    await store.initialize();
    const checking = store.checkUpdates();
    current = 'yanked';
    topics.get('plugin.updates')?.({ report: report('yanked') });
    failCheck(new Error('another update check is in progress'));
    await checking;
    await flushPromises();
    expect(calls.filter((call) => call.method === 'plugins.updates.list')).toHaveLength(2);
    expect(store.updates?.updates[0]?.latestStatus).toBe('yanked');
    expect(store.checkingUpdates).toBe(false);
  });

  it('normalizes partial version responses to empty lists', async () => {
    const { store } = harness((method) => (method === 'plugins.packages.versions' ? { installed: '1.0.0' } : {}));
    await store.loadVersions('acme.tts');
    expect(store.versions['acme.tts']).toMatchObject({ id: 'acme.tts', installed: '1.0.0', retained: [], available: [] });
  });

  it('keeps only the latest review when inspections overlap', async () => {
    let resolveFirst!: (value: unknown) => void;
    const { store } = harness((method, params) => {
      if (method !== 'plugins.packages.inspect') return {};
      const spec = (params as { spec: { path: string } }).spec;
      if (spec.path === 'first') return new Promise((resolve) => (resolveFirst = resolve));
      return { ...preview, name: 'Second' };
    });
    void store.reviewSpec({ kind: 'file', path: 'first' });
    await store.reviewSpec({ kind: 'file', path: 'second' });
    resolveFirst({ ...preview, name: 'First' });
    await flushPromises();
    expect(store.review.preview?.name).toBe('Second');
  });

  it('reports typed inspection failures in the review', async () => {
    const { store } = harness(() => {
      throw new ControlCallError('install_failed', 'no build', { code: 'incompatible', message: 'no build' });
    });
    await store.reviewSpec({ kind: 'spec', spec: 'acme.tts' });
    expect(store.review.error?.code).toBe('incompatible');
    expect(store.review.loading).toBe(false);
  });

  it('opens a review when an automatic update needs consent', async () => {
    const { store, topics, calls } = harness((method) => {
      if (method === 'plugins.packages.update') return { job: job({}) };
      return { receipts: [], jobs: [], updates: [], checkedAt: 0 };
    });
    await store.initialize();
    await store.updatePlugin('acme.tts', '1.1.0');
    topics.get('plugin.job')?.({
      job: job({ status: 'failed', error: { code: 'consent-required', message: 'review', details: preview } }),
    });
    expect(store.review.open).toBe(true);
    expect(store.review.mode).toBe('update');
    await store.confirmReview();
    expect(calls.filter((call) => call.method === 'plugins.packages.update').at(-1)?.params).toEqual({
      pluginId: 'acme.tts',
      version: '1.1.0',
      consentToken: 'token-2',
    });
  });

  it('refreshes receipts and drops the update entry after a successful job', async () => {
    const { store, topics, calls } = harness((method) => {
      if (method === 'plugins.updates.list') {
        return { updates: [{ id: 'acme.tts', latest: '1.1.0', current: '1.0.0', latestStatus: 'approved' }], checkedAt: 1 };
      }
      return { receipts: [], jobs: [] };
    });
    await store.initialize();
    expect(store.updateCount).toBe(1);
    topics.get('plugin.job')?.({ job: job({ status: 'succeeded', stage: 'done' }) });
    await flushPromises();
    expect(store.updateCount).toBe(0);
    expect(calls.filter((call) => call.method === 'plugins.packages.receipts')).toHaveLength(2);
    expect(store.activeJobs).toHaveLength(0);
  });
});

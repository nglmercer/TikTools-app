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
        return { updates: [{ id: 'acme.tts', latest: '1.1.0', current: '1.0.0' }], checkedAt: 1 };
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

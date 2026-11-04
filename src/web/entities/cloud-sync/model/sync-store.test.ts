import { describe, expect, it } from 'vitest';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import type { SyncStatus } from '../api/sync-types.ts';
import { useSyncStore } from './sync-store.ts';

function idle(overrides: Partial<SyncStatus> = {}): SyncStatus {
  return {
    enabled: true,
    signedIn: true,
    syncing: false,
    pending: 0,
    conflicts: [],
    lastError: null,
    lastSyncedAt: null,
    ...overrides,
  };
}

function harness(handler: (method: string, params: RpcParams | undefined) => unknown) {
  const calls: { method: string; params: RpcParams | undefined }[] = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async (method: string, params?: RpcParams) => {
      calls.push({ method, params });
      return handler(method, params);
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  const stores = createStoreHarness(client);
  const store = stores.use(() => useSyncStore());
  return { store, calls };
}

describe('sync store', () => {
  it('refreshes nonsecret status without touching tokens', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'sync.status') return idle({ pending: 2 });
      throw new Error(`unexpected ${method}`);
    });
    await store.refresh();
    expect(store.status?.pending).toBe(2);
    expect(store.error).toBeNull();
    expect(calls.map((call) => call.method)).toEqual(['sync.status']);
  });

  it('toggles the opt-in flag', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'sync.disable') return idle({ enabled: false });
      throw new Error(`unexpected ${method}`);
    });
    await store.setEnabled(false);
    expect(store.status?.enabled).toBe(false);
    expect(calls.map((call) => call.method)).toEqual(['sync.disable']);
  });

  it('runs a sync and refreshes status', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'sync.now') {
        return { pushed: 1, pulled: 2, applied: 2, skipped: 0, conflicts: [] };
      }
      if (method === 'sync.status') return idle({ lastSyncedAt: 1700000000 });
      throw new Error(`unexpected ${method}`);
    });
    const outcome = await store.syncNow();
    expect(outcome?.applied).toBe(2);
    expect(store.status?.lastSyncedAt).toBe(1700000000);
    expect(calls.map((call) => call.method)).toEqual(['sync.now', 'sync.status']);
  });

  it('resolves a conflict and surfaces failures', async () => {
    const { store, calls } = harness((method, params) => {
      if (method === 'sync.resolve') {
        expect(params).toEqual({ key: 'acme.player', keepLocal: false });
        return idle();
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.resolve('acme.player', false);
    expect(store.status?.conflicts).toEqual([]);
    expect(calls.map((call) => call.method)).toEqual(['sync.resolve']);
  });

  it('keeps the error text when sync is rejected', async () => {
    const { store } = harness(() => {
      throw new Error('cloud sync is not enabled');
    });
    await store.refresh();
    expect(store.status).toBeNull();
    expect(store.error).toContain('not enabled');
  });
});

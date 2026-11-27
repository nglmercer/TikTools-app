import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import type { SyncNowResult, SyncStatus } from '../../../../../../src/web/entities/cloud-sync/api/sync-types.ts';
import { AUTO_SYNC_DEBOUNCE_MS, AUTO_SYNC_INTERVAL_MS, useSyncStore } from '../../../../../../src/web/entities/cloud-sync/model/sync-store.ts';
import { RPC } from '../../../../../shared/index.ts';

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
  const calls: {
    method: string;
    params: RpcParams | undefined;
  }[] = [];
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
      if (method === RPC.syncStatus) return idle({ pending: 2 });
      throw new Error(`unexpected ${method}`);
    });
    await store.refresh();
    expect(store.status?.pending).toBe(2);
    expect(store.error).toBeNull();
    expect(calls.map((call) => call.method)).toEqual([RPC.syncStatus]);
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
      if (method === RPC.syncStatus) return idle({ lastSyncedAt: 1700000000 });
      throw new Error(`unexpected ${method}`);
    });
    const outcome = await store.syncNow();
    expect(outcome?.applied).toBe(2);
    expect(store.status?.lastSyncedAt).toBe(1700000000);
    expect(calls.map((call) => call.method)).toEqual(['sync.now', RPC.syncStatus]);
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

  it('discards a status reply from before a context invalidation', async () => {
    let oldReply!: (status: SyncStatus) => void;
    let calls = 0;
    const { store } = harness(() => {
      calls += 1;
      return calls === 1 ? new Promise<SyncStatus>((resolve) => { oldReply = resolve; }) : idle({ enabled: false });
    });
    const old = store.refresh();
    store.invalidate();
    await Promise.resolve();
    await Promise.resolve();
    oldReply(idle({ enabled: true, conflicts: ['old-account'] }));
    await old;
    expect(store.status?.enabled).toBe(false);
    expect(store.status?.conflicts).toEqual([]);
  });

  it('waits for a mutation to settle before refreshing an invalidated status', async () => {
    let finish!: (status: SyncStatus) => void;
    const { store, calls } = harness((method) => method === 'sync.disable'
      ? new Promise<SyncStatus>((resolve) => { finish = resolve; })
      : idle({ enabled: true, pending: 7 }));
    const disabling = store.setEnabled(false);
    store.invalidate();
    expect(store.status).toBeNull();
    expect(calls.map((call) => call.method)).toEqual(['sync.disable']);
    finish(idle({ enabled: false, conflicts: ['stale'] }));
    await disabling;
    expect(calls.map((call) => call.method)).toEqual(['sync.disable', RPC.syncStatus]);
    expect(store.status?.enabled).toBe(true);
    expect(store.status?.conflicts).toEqual([]);
    expect(store.status?.pending).toBe(7);
  });

  it('does not return an old sync outcome when invalidated during its follow-up read', async () => {
    let finishRead!: (status: SyncStatus) => void;
    let reads = 0;
    const { store } = harness((method) => {
      if (method === 'sync.now') return { pushed: 1, pulled: 1, applied: 0, skipped: 1, conflicts: ['old'] };
      reads += 1;
      return reads === 1 ? new Promise<SyncStatus>((resolve) => { finishRead = resolve; }) : idle({ enabled: false });
    });
    const syncing = store.syncNow();
    await Promise.resolve();
    await Promise.resolve();
    store.invalidate();
    finishRead(idle({ enabled: true }));
    expect(await syncing).toBeNull();
    await Promise.resolve();
    expect(store.status?.enabled).toBe(false);
  });
});

describe('automatic sync', () => {
  beforeEach(() => { vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });

  const result = (): SyncNowResult => ({ pushed: 0, pulled: 0, applied: 0, skipped: 0, applicationDeferred: false, conflicts: [] });

  function autoHarness(status: SyncStatus = idle()) {
    return harness((method) => {
      if (method === RPC.syncStatus) return status;
      if (method === 'sync.now') return result();
      throw new Error(`unexpected ${method}`);
    });
  }

  it('calls syncNow every 60 seconds when enabled and signed in', async () => {
    const { store, calls } = autoHarness();
    await store.refresh();
    store.startAutoSync();
    await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL_MS - 1);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(1);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(1);
    store.stopAutoSync();
  });

  it('does not run when sync is disabled or the user is signed out', async () => {
    for (const status of [idle({ enabled: false }), idle({ signedIn: false })]) {
      const { store, calls } = autoHarness(status);
      await store.refresh();
      store.startAutoSync();
      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL_MS * 2);
      expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(0);
      store.stopAutoSync();
    }
  });

  it('does not start a second run while a sync is working', async () => {
    let release!: (value: SyncNowResult) => void;
    const { store, calls } = harness((method) => {
      if (method === RPC.syncStatus) return idle();
      if (method === 'sync.now') return new Promise<SyncNowResult>((resolve) => { release = resolve; });
      throw new Error(`unexpected ${method}`);
    });
    await store.refresh();
    store.startAutoSync();
    const first = store.syncNow();
    expect(store.working).toBe(true);
    await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL_MS * 2);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(1);
    release(result());
    await first;
    store.stopAutoSync();
  });

  it('runs once three seconds after invalidate for a local change', async () => {
    const { store, calls } = autoHarness();
    await store.refresh();
    store.invalidate();
    await Promise.resolve();
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(AUTO_SYNC_DEBOUNCE_MS - 1);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(1);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(1);
    store.stopAutoSync();
  });

  it('coalesces repeated invalidations into a single debounced run', async () => {
    const { store, calls } = autoHarness();
    await store.refresh();
    store.invalidate();
    store.invalidate();
    await Promise.resolve();
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(AUTO_SYNC_DEBOUNCE_MS + 1);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(1);
    store.stopAutoSync();
  });

  it('stops both triggers when auto sync is stopped', async () => {
    const { store, calls } = autoHarness();
    await store.refresh();
    store.startAutoSync();
    store.stopAutoSync();
    store.invalidate();
    await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL_MS + AUTO_SYNC_DEBOUNCE_MS);
    expect(calls.filter((call) => call.method === 'sync.now')).toHaveLength(0);
  });
});

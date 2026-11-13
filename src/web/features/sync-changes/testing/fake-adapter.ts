import { vi } from 'vitest';
import { defineSyncAdapter, type SyncAdapter, type SyncCapabilities, type SyncChange, type SyncOperation, type SyncResolution } from '@/entities/cloud-sync/api/sync-adapter.ts';

export const sampleChanges: SyncChange[] = [
  { id: 'e1', kind: 'event', name: 'Greeting', status: 'modified', side: 'local', before: { name: 'Hi', volume: 5 }, after: { name: 'Hello', volume: 5 } },
  { id: 'e2', kind: 'event', name: 'Gift', status: 'added', side: 'local', after: { name: 'Gift' } },
  { id: 'e3', kind: 'event', name: 'Follow', status: 'modified', side: 'remote', device: 'Pixel 8', before: { name: 'Follow' }, after: { name: 'Follow!' } },
  {
    id: 'e4', kind: 'event', name: 'Clash', status: 'conflict', side: 'conflict',
    base: { name: 'Base', volume: 5, color: 'red' }, before: { name: 'Base', volume: 5, color: 'red' },
    after: { name: 'Mine', volume: 5, color: 'blue' }, remote: { name: 'Theirs', volume: 9, color: 'red' },
  },
];

export const sampleOperations: SyncOperation[] = [
  { id: 'op1', kind: 'push', title: 'Sent two events', device: 'Desktop', at: 1_700_000_000_000, changes: [sampleChanges[0]!, sampleChanges[1]!] },
  { id: 'op2', kind: 'fetch', title: 'Fetched one event', at: 1_700_000_100_000, changes: [sampleChanges[2]!] },
];

/** An adapter whose every operation is a spy, with the capabilities the test asks for. */
export function fakeAdapter(capabilities: Partial<SyncCapabilities> = {}, changes: SyncChange[] = sampleChanges) {
  const spies = {
    list: vi.fn(async () => changes),
    push: vi.fn<(ids: readonly string[]) => Promise<void>>(async () => {}),
    fetch: vi.fn(async () => {}),
    apply: vi.fn<(id: string) => Promise<void>>(async () => {}),
    withdraw: vi.fn<(id: string) => Promise<void>>(async () => {}),
    sync: vi.fn(async () => {}),
    reset: vi.fn<(id: string) => Promise<void>>(async () => {}),
    resetAll: vi.fn(async () => {}),
    resolve: vi.fn<(id: string, resolution: SyncResolution) => Promise<void>>(async () => {}),
    forcePush: vi.fn(async () => {}),
    forcePull: vi.fn(async () => {}),
    history: vi.fn(async () => sampleOperations),
    restore: vi.fn<(operationId: string, id?: string) => Promise<void>>(async () => {}),
    fork: vi.fn(async () => {}),
    markReviewed: vi.fn(async () => {}),
    setAuto: vi.fn(async () => {}),
  };
  const adapter: SyncAdapter = defineSyncAdapter({
    scope: { id: 'events', label: 'Events' },
    capabilities,
    kindLabel: (kind) => kind.toUpperCase(),
    requiresCloud: false,
    list: spies.list,
    push: spies.push,
    fetch: spies.fetch,
    apply: spies.apply,
    withdraw: spies.withdraw,
    sync: spies.sync,
    reset: spies.reset,
    resetAll: spies.resetAll,
    resolve: spies.resolve,
    forcePush: spies.forcePush,
    forcePull: spies.forcePull,
    history: spies.history,
    restore: spies.restore,
    fork: spies.fork,
    markReviewed: spies.markReviewed,
    autoSync: { get: () => false, set: spies.setAuto },
  });
  return { adapter, spies };
}

export const allCapabilities: SyncCapabilities = {
  push: true, fetch: true, sync: true, reset: true, resolve: true, merge: true, force: true, history: true, fork: true, branches: false, autoSync: true, markReviewed: true, apply: true, withdraw: true,
};

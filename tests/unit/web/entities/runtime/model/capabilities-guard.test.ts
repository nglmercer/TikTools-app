import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { useCapabilityStore } from '../../../../../../src/web/entities/runtime/model/capabilities-store.ts';
import { ACCOUNT_STATE, RPC } from '../../../../../shared/index.ts';

function host(announced: string[]) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === RPC.rpcDiscover) return { methods: announced.map((name) => ({ name })) };
    if (method === RPC.cloudStatus) return { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedOut, accountId: null, persistence: 'keychain', error: null };
    if (method === RPC.syncStatus) return { enabled: false, pending: 0, conflicts: [], lastError: null };
    throw new Error(`unexpected ${method}`);
  });
  const stores = createStoreHarness(client);
  return { stores, calls: () => calls.map((call) => call.method) };
}

describe('capability guards', () => {
  it('does not call Cloud or sync methods the host did not announce', async () => {
    const { stores, calls } = host([RPC.rpcDiscover, RPC.systemInfo]);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    await stores.use(() => useAccountStore()).refresh();
    await stores.use(() => useSyncStore()).refresh();
    expect(calls()).toEqual([RPC.rpcDiscover]);
    expect(stores.use(() => useAccountStore()).status).toBeNull();
    expect(stores.use(() => useAccountStore()).error).toBeNull();
  });

  it('calls announced methods normally', async () => {
    const { stores, calls } = host([RPC.rpcDiscover, RPC.cloudStatus, RPC.syncStatus]);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    await stores.use(() => useAccountStore()).refresh();
    await stores.use(() => useSyncStore()).refresh();
    expect(calls()).toEqual([RPC.rpcDiscover, RPC.cloudStatus, RPC.syncStatus]);
  });

  it('keeps calling before any discovery, so unknown is not treated as absent', async () => {
    const { stores, calls } = host([]);
    await stores.use(() => useAccountStore()).refresh();
    expect(calls()).toEqual([RPC.cloudStatus]);
  });
});

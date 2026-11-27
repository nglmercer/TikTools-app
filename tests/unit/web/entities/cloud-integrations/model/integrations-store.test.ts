import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { isValidLabel, useIntegrationsStore } from '../../../../../../src/web/entities/cloud-integrations/model/integrations-store.ts';
import { ACCOUNT_STATE, FIXTURE_ID, RPC } from '../../../../../shared/index.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const secret = 'b'.repeat(64);
const row = { id, provider: 'webhook', label: 'Alerts', status: 'active', createdAt: 'now' };

function host(announced: string[], rows: unknown[] = []) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === RPC.rpcDiscover) return { methods: announced.map((name) => ({ name })) };
    if (method === RPC.integrationsConnections) return { items: rows };
    if (method === RPC.integrationsConnect) return {
      connectionId: id,
      provider: 'webhook',
      label: 'Alerts',
      secret,
      deliveryPath: `/api/v3/integrations/webhook/${id}/deliveries`,
    };
    if (method === RPC.integrationsDisconnect) return { id, status: 'revoked' };
    if (method === 'integrations.events') return { items: [], truncated: false };
    throw new Error(`unexpected ${method}`);
  });
  const stores = createStoreHarness(client);
  return { stores, calls };
}

describe('cloud integrations store', () => {
  it('validates labels with the same bounds as the host', () => {
    expect(isValidLabel('  Stream  ')).toBe(true);
    expect(isValidLabel('   ')).toBe(false);
    expect(isValidLabel('x'.repeat(121))).toBe(false);
    expect(isValidLabel('bad\u0007')).toBe(false);
  });

  it('does not call integrations when the host did not announce them', async () => {
    const { stores, calls } = host([RPC.rpcDiscover]);
    await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
    await stores.use(() => useIntegrationsStore()).refresh();
    expect(calls.map((call) => call.method)).toEqual([RPC.rpcDiscover]);
  });

  it('holds the secret in memory until it is dismissed', async () => {
    const { stores } = host([RPC.integrationsConnections, RPC.integrationsConnect]);
    await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
    const integrations = stores.use(() => useIntegrationsStore());
    expect(await integrations.connect('Alerts')).toBe(true);
    expect(integrations.issued?.secret).toBe(secret);
    integrations.dismissSecret();
    expect(integrations.issued).toBeNull();
  });

  it('clears the secret and any open events when the panel resets', async () => {
    const { stores } = host([RPC.integrationsConnections, RPC.integrationsConnect, 'integrations.events']);
    await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
    const integrations = stores.use(() => useIntegrationsStore());
    await integrations.connect('Alerts');
    await integrations.openEvents(id);
    integrations.reset();
    expect(integrations.issued).toBeNull();
    expect(integrations.selectedId).toBeNull();
    expect(integrations.events).toBeNull();
  });

  it('revokes and refreshes the list', async () => {
    const { stores, calls } = host([RPC.integrationsConnections, RPC.integrationsDisconnect], [row]);
    await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
    const integrations = stores.use(() => useIntegrationsStore());
    await integrations.refresh();
    expect(await integrations.remove(id)).toBe(true);
    expect(calls.map((call) => call.method)).toEqual([RPC.rpcDiscover, RPC.integrationsConnections, RPC.integrationsDisconnect, RPC.integrationsConnections]);
  });

  describe('late replies', () => {
    function deferred<T>() {
      let resolve!: (value: T) => void;
      let reject!: (failure: unknown) => void;
      const promise = new Promise<T>((done, fail) => {
        resolve = done;
        reject = fail;
      });
      return { promise, resolve, reject };
    }
    const issuedSecret = { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: `/api/v3/integrations/webhook/${id}/deliveries` };
    function held(extra: (method: string) => unknown = () => undefined) {
      const connect = deferred<unknown>();
      const { client, calls } = createFakeControlClient((method) => {
        if (method === RPC.rpcDiscover) return {
          methods: [RPC.integrationsConnections, RPC.integrationsConnect, RPC.integrationsDisconnect, 'integrations.events'].map((name) => ({ name })),
        };
        if (method === RPC.integrationsConnect) return connect.promise;
        if (method === RPC.integrationsConnections) return { items: [row] };
        return extra(method) ?? { items: [], truncated: false };
      });
      const stores = createStoreHarness(client);
      return { stores, calls, connect };
    }

    it('never publishes a secret that arrives after the panel reset, and does not leave it working', async () => {
      const { stores, connect, calls } = held();
      await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
      const integrations = stores.use(() => useIntegrationsStore());
      const created = integrations.connect('Alerts');
      expect(integrations.working).toBe(true);
      integrations.reset();
      expect(integrations.working).toBe(false);
      connect.resolve(issuedSecret);
      expect(await created).toBe(false);
      expect(integrations.issued).toBeNull();
      expect(integrations.error).toBeNull();
      expect(integrations.working).toBe(false);
      // The mutation was sent once and is never repeated by the store.
      expect(calls.filter((call) => call.method === RPC.integrationsConnect)).toHaveLength(1);
    });

    it('never publishes a secret that arrives after the account changed, including logout and login of the same account', async () => {
      const { stores, connect } = held();
      await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
      const account = stores.use(() => useAccountStore());
      account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
      const integrations = stores.use(() => useIntegrationsStore());
      const created = integrations.connect('Alerts');
      account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedOut, persistence: ACCOUNT_STATE.memoryOnly };
      account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
      connect.resolve(issuedSecret);
      expect(await created).toBe(false);
      expect(integrations.issued).toBeNull();
      expect(integrations.working).toBe(false);
    });

    it('drops the previous account list and a stale failure when the account changes', async () => {
      const list = deferred<unknown>();
      let lists = 0;
      const { client } = createFakeControlClient((method) => {
        if (method === RPC.rpcDiscover) return { methods: [{ name: RPC.integrationsConnections }] };
        // The first list is Alice's and stays pending; the one for the new account answers.
        return (lists += 1) === 1 ? list.promise : { items: [] };
      });
      const stores = createStoreHarness(client);
      await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
      const account = stores.use(() => useAccountStore());
      account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
      const integrations = stores.use(() => useIntegrationsStore());
      integrations.connections = [row];
      const loading = integrations.refresh();
      expect(integrations.loading).toBe(true);
      account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'bob', persistence: ACCOUNT_STATE.memoryOnly };
      expect(integrations.connections).toEqual([]);
      await flushPromises();
      expect(integrations.loading).toBe(false);
      list.reject(new Error('alice private failure'));
      await loading;
      await flushPromises();
      expect(integrations.connections).toEqual([]);
      expect(integrations.error).toBeNull();
      expect(integrations.errorDetail).toBeNull();
      expect(integrations.loading).toBe(false);
    });

    it('ends the context on the host notice and on an event gap', async () => {
      let notice = () => {};
      let gap = () => {};
      let listeners = 0;
      const { client } = createFakeControlClient((method) =>
        (method === RPC.rpcDiscover ? { methods: [{ name: RPC.integrationsConnections }] } : { items: [] }),
      );
      client.onTopic = (topic, handler) => {
        if (topic === 'cloud.context.invalidated') {
          notice = () => Reflect.apply(handler, undefined, [{}]);
          listeners += 1;
        }
        return () => { listeners -= 1; };
      };
      client.onGap = (handler) => {
        gap = () => handler(1);
        listeners += 1;
        return () => { listeners -= 1; };
      };
      const stores = createStoreHarness(client);
      const integrations = stores.use(() => useIntegrationsStore());
      integrations.issued = { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: '/x' };
      notice();
      expect(integrations.issued).toBeNull();
      integrations.issued = { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: '/x' };
      gap();
      expect(integrations.issued).toBeNull();
      expect(listeners).toBe(2);
    });

    it('a nested operation does not release the activity of the outer one', async () => {
      const { stores, connect } = held();
      await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.hostA);
      const integrations = stores.use(() => useIntegrationsStore());
      const created = integrations.connect('Alerts');
      await integrations.remove(id);
      expect(integrations.working).toBe(true);
      connect.resolve(issuedSecret);
      expect(await created).toBe(true);
      expect(integrations.working).toBe(false);
      expect(integrations.issued?.secret).toBe(secret);
    });
  });
});

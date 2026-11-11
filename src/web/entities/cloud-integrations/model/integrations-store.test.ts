import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { isValidLabel, useIntegrationsStore } from './integrations-store.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const secret = 'b'.repeat(64);
const row = { id, provider: 'webhook', label: 'Alerts', status: 'active', createdAt: 'now' };

function host(announced: string[], rows: unknown[] = []) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'rpc.discover') return { methods: announced.map((name) => ({ name })) };
    if (method === 'integrations.connections') return { items: rows };
    if (method === 'integrations.connect') return { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: `/api/v3/integrations/webhook/${id}/deliveries` };
    if (method === 'integrations.disconnect') return { id, status: 'revoked' };
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
    const { stores, calls } = host(['rpc.discover']);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    await stores.use(() => useIntegrationsStore()).refresh();
    expect(calls.map((call) => call.method)).toEqual(['rpc.discover']);
  });

  it('holds the secret in memory until it is dismissed', async () => {
    const { stores } = host(['integrations.connections', 'integrations.connect']);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const integrations = stores.use(() => useIntegrationsStore());
    expect(await integrations.connect('Alerts')).toBe(true);
    expect(integrations.issued?.secret).toBe(secret);
    integrations.dismissSecret();
    expect(integrations.issued).toBeNull();
  });

  it('clears the secret and any open events when the panel resets', async () => {
    const { stores } = host(['integrations.connections', 'integrations.connect', 'integrations.events']);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const integrations = stores.use(() => useIntegrationsStore());
    await integrations.connect('Alerts');
    await integrations.openEvents(id);
    integrations.reset();
    expect(integrations.issued).toBeNull();
    expect(integrations.selectedId).toBeNull();
    expect(integrations.events).toBeNull();
  });

  it('revokes and refreshes the list', async () => {
    const { stores, calls } = host(['integrations.connections', 'integrations.disconnect'], [row]);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const integrations = stores.use(() => useIntegrationsStore());
    await integrations.refresh();
    expect(await integrations.remove(id)).toBe(true);
    expect(calls.map((call) => call.method)).toEqual(['rpc.discover', 'integrations.connections', 'integrations.disconnect', 'integrations.connections']);
  });

  describe('late replies', () => {
    function deferred<T>() {
      let resolve!: (value: T) => void; let reject!: (failure: unknown) => void;
      const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
      return { promise, resolve, reject };
    }
    const issuedSecret = { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: `/api/v3/integrations/webhook/${id}/deliveries` };
    function held(extra: (method: string) => unknown = () => undefined) {
      const connect = deferred<unknown>();
      const { client, calls } = createFakeControlClient((method) => {
        if (method === 'rpc.discover') return { methods: ['integrations.connections', 'integrations.connect', 'integrations.disconnect', 'integrations.events'].map((name) => ({ name })) };
        if (method === 'integrations.connect') return connect.promise;
        if (method === 'integrations.connections') return { items: [row] };
        return extra(method) ?? { items: [], truncated: false };
      });
      const stores = createStoreHarness(client);
      return { stores, calls, connect };
    }

    it('never publishes a secret that arrives after the panel reset, and does not leave it working', async () => {
      const { stores, connect, calls } = held();
      await stores.use(() => useCapabilityStore()).refresh('host-a');
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
      expect(calls.filter((call) => call.method === 'integrations.connect')).toHaveLength(1);
    });

    it('never publishes a secret that arrives after the account changed, including logout and login of the same account', async () => {
      const { stores, connect } = held();
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const account = stores.use(() => useAccountStore());
      account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
      const integrations = stores.use(() => useIntegrationsStore());
      const created = integrations.connect('Alerts');
      account.status = { origin: 'https://cloud.example', state: 'signed-out', persistence: 'memory-only' };
      account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
      connect.resolve(issuedSecret);
      expect(await created).toBe(false);
      expect(integrations.issued).toBeNull();
      expect(integrations.working).toBe(false);
    });

    it('drops the previous account list and a stale failure when the account changes', async () => {
      const list = deferred<unknown>();
      let lists = 0;
      const { client } = createFakeControlClient((method) => {
        if (method === 'rpc.discover') return { methods: [{ name: 'integrations.connections' }] };
        // The first list is Alice's and stays pending; the one for the new account answers.
        return (lists += 1) === 1 ? list.promise : { items: [] };
      });
      const stores = createStoreHarness(client);
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const account = stores.use(() => useAccountStore());
      account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
      const integrations = stores.use(() => useIntegrationsStore());
      integrations.connections = [row];
      const loading = integrations.refresh();
      expect(integrations.loading).toBe(true);
      account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'bob', persistence: 'memory-only' };
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
      let notice = () => {}; let gap = () => {};
      let listeners = 0;
      const { client } = createFakeControlClient((method) => (method === 'rpc.discover' ? { methods: [{ name: 'integrations.connections' }] } : { items: [] }));
      client.onTopic = (topic, handler) => { if (topic === 'cloud.context.invalidated') { notice = () => handler({} as never); listeners += 1; } return () => { listeners -= 1; }; };
      client.onGap = (handler) => { gap = () => handler(1); listeners += 1; return () => { listeners -= 1; }; };
      const stores = createStoreHarness(client);
      const integrations = stores.use(() => useIntegrationsStore());
      integrations.issued = { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: '/x' } as never;
      notice();
      expect(integrations.issued).toBeNull();
      integrations.issued = { connectionId: id, provider: 'webhook', label: 'Alerts', secret, deliveryPath: '/x' } as never;
      gap();
      expect(integrations.issued).toBeNull();
      expect(listeners).toBe(2);
    });

    it('a nested operation does not release the activity of the outer one', async () => {
      const { stores, connect } = held();
      await stores.use(() => useCapabilityStore()).refresh('host-a');
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

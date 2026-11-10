import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
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
});

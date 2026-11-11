import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudConnections from './CloudConnections.vue';

enableAutoUnmount(afterEach);
afterEach(async () => { await activateLocale('en'); });

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const secret = 'c'.repeat(64);
const active = { id, provider: 'webhook', label: 'Alerts', status: 'active', createdAt: '2026-11-10T00:00:00Z' };

function mount(rows: unknown[] = [active]) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'integrations.connections') return { items: rows };
    if (method === 'integrations.connect') return { connectionId: id, provider: 'webhook', label: 'New', secret, deliveryPath: `/api/v3/integrations/webhook/${id}/deliveries` };
    if (method === 'integrations.disconnect') return { id, status: 'revoked' };
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudConnections, { global: { plugins: storePlugins(client) } });
  return { wrapper, calls };
}

const button = (wrapper: ReturnType<typeof mountWithPlugins>, text: string) =>
  wrapper.findAll('button').find((node) => node.text() === text)!;

describe('CloudConnections', () => {
  it('keeps create disabled for an invalid label and shows the secret once', async () => {
    const { wrapper, calls } = mount();
    await flushPromises();
    expect(button(wrapper, 'Create webhook').attributes('disabled')).toBeDefined();

    await wrapper.get('input').setValue('New');
    await flushPromises();
    expect(button(wrapper, 'Create webhook').attributes('disabled')).toBeUndefined();

    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === 'integrations.connect')?.params).toEqual({ label: 'New' });
    expect(wrapper.text()).toContain(secret);
    expect(wrapper.text()).toContain('It will not be shown again');

    await button(wrapper, 'Done').trigger('click');
    await flushPromises();
    expect(wrapper.text()).not.toContain(secret);
  });

  it('asks for a second click before revoking a connection', async () => {
    const { wrapper, calls } = mount();
    await flushPromises();
    await button(wrapper, 'Disconnect').trigger('click');
    expect(calls.some((call) => call.method === 'integrations.disconnect')).toBe(false);
    await button(wrapper, 'Confirm disconnect').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'integrations.disconnect')?.params).toEqual({ id });
  });

  it('shows the empty state when there are no connections', async () => {
    const { wrapper } = mount([]);
    await flushPromises();
    expect(wrapper.text()).toContain('No connections yet.');
  });
});

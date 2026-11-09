import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { ConnectionStatus, LiveChannel } from '@/types.ts';
import ConnectionsPage from './ConnectionsPage.vue';

enableAutoUnmount(afterEach);

beforeEach(() => {
  localStorage.clear();
});

const rooms: LiveChannel[] = [
  { uniqueId: 'luna', roomId: '1', nickname: 'Luna', title: 'Late night chill', viewers: 1520 },
  { uniqueId: 'mango', roomId: '2', nickname: 'Mango', title: '', viewers: 0 },
];

async function mountPage(status: ConnectionStatus = 'idle') {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'live.channels') return { rooms, fetchedAt: Date.now(), source: 'test' };
    if (method === 'live.connect' || method === 'live.pick') return { connected: true, uniqueId: 'luna', roomId: '1', connectionId: 'c', native: true };
    return {};
  });
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/:any(.*)*', component: { render: () => null } }] });
  const wrapper = mountWithPlugins(ConnectionsPage, { attachTo: document.body, global: { plugins: [router, ...storePlugins(client)] } });
  const connection = useConnectionStore();
  connection.status = status;
  connection.recents = ['luna', 'mango'];
  if (status === 'connected') connection.activeCreator = 'luna';
  await flushPromises();
  calls.length = 0;
  return { wrapper, connection, calls };
}

function buttonByText(wrapper: VueWrapper, text: string) {
  return wrapper.findAll('button').find((button) => button.text() === text)!;
}

describe('ConnectionsPage', () => {
  it('connects to the typed creator', async () => {
    const { wrapper, connection, calls } = await mountPage();
    expect(buttonByText(wrapper, 'Connect').attributes('disabled')).toBeDefined();
    await wrapper.get('#live-handle').setValue('luna');
    expect(connection.uniqueId).toBe('luna');
    await buttonByText(wrapper, 'Connect').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'live.connect')?.params).toMatchObject({ uniqueId: 'luna' });
  });

  it('offers recent creators and a random room', async () => {
    const { wrapper, connection, calls } = await mountPage();
    await wrapper.get('button[aria-label="Recent creators"]').trigger('click');
    await nextTick();
    const items = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'));
    expect(items.map((item) => item.textContent?.trim())).toEqual(['@luna', '@mango']);
    items[1]!.click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'live.connect')?.params).toMatchObject({ uniqueId: 'mango' });
    connection.status = 'idle';
    await nextTick();
    await wrapper.get('button[aria-label="Connect to a random LIVE"]').trigger('click');
    await flushPromises();
    expect(calls.some((call) => call.method === 'live.pick')).toBe(true);
  });

  it('reveals the cookie field on demand', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.find('#live-cookie').exists()).toBe(false);
    await wrapper.get('input[type="checkbox"]').setValue(true);
    expect(wrapper.get('#live-cookie').attributes('type')).toBe('password');
  });

  it('locks the form and offers disconnect while live', async () => {
    const { wrapper, calls } = await mountPage('connected');
    expect(wrapper.get('#live-handle').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('Receiving events from @luna.');
    await buttonByText(wrapper, 'Disconnect').trigger('click');
    await flushPromises();
    expect(calls.some((call) => call.method === 'live.disconnect')).toBe(true);
  });

  it.each<ConnectionStatus>(['connecting', 'retrying'])('cancels while %s and unlocks the form', async (status) => {
    const { wrapper, connection, calls } = await mountPage(status);
    expect(wrapper.get('#live-handle').attributes('disabled')).toBeDefined();
    const cancel = buttonByText(wrapper, 'Cancel');
    expect(cancel.attributes('disabled')).toBeUndefined();
    await cancel.trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === 'live.disconnect')).toHaveLength(1);
    expect(connection.status).toBe('disconnected');
    expect(wrapper.get('#live-handle').attributes('disabled')).toBeUndefined();
    expect(buttonByText(wrapper, 'Connect')).toBeDefined();
  });

  it('keeps the explorer visible but only browses after explicit request', async () => {
    const { wrapper, calls } = await mountPage();
    expect(calls).toHaveLength(0);
    expect(buttonByText(wrapper, 'Explore').isVisible()).toBe(true);
    await buttonByText(wrapper, 'Explore').trigger('click');
    await flushPromises();
    expect(calls[0]?.method).toBe('live.channels');
    const rows = wrapper.findAll('.live-explorer tbody tr');
    expect(rows.map((row) => row.text())).toEqual([expect.stringContaining('@luna'), expect.stringContaining('@mango')]);
    expect(rows[0]!.text()).toContain('1,520');
    await rows[1]!.get('button').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'live.connect')?.params).toMatchObject({ uniqueId: 'mango' });
  });

  it('points to plugins when no server connection exists', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.text()).toContain('No server connections yet');
    expect(buttonByText(wrapper, 'Open plugins').exists()).toBe(true);
  });
});

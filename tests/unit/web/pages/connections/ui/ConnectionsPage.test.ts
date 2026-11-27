import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import type { LiveMonitorStatus } from '@/entities/connection/model/live-monitor-state.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { ConnectionStatus, LiveChannel } from '@/types.ts';
import ConnectionsPage from '../../../../../../src/web/pages/connections/ui/ConnectionsPage.vue';
import { RPC } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

beforeEach(() => {
  localStorage.clear();
});

const rooms: LiveChannel[] = [
  { uniqueId: 'luna', roomId: '1', nickname: 'Luna', title: 'Late night chill', viewers: 1520 },
  { uniqueId: 'mango', roomId: '2', nickname: 'Mango', title: '', viewers: 0 },
];

async function mountPage(status: ConnectionStatus = 'idle', announce: string[] = []) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === RPC.rpcDiscover) return { methods: announce.map((name) => ({ name })) };
    if (method === RPC.integrationsConnections) return { items: [] };
    if (method === RPC.liveChannels) return { rooms, fetchedAt: Date.now(), source: t('automations.runs.testBadge') };
    if (method === RPC.liveConnect || method === 'live.pick') return { connected: true, uniqueId: 'luna', roomId: '1', connectionId: 'c', native: true };
    return {};
  });
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/:any(.*)*', component: { render: () => null } }] });
  const stores = createStoreHarness(client);
  if (announce.length > 0) await stores.use(() => useCapabilityStore()).refresh('test-host');
  const wrapper = mountWithPlugins(ConnectionsPage, { attachTo: document.body, global: { plugins: [router, ...stores.plugins] } });
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
    expect(buttonByText(wrapper, t('connection.connect')).attributes('disabled')).toBeDefined();
    await wrapper.get('#live-handle').setValue('luna');
    expect(connection.uniqueId).toBe('luna');
    await buttonByText(wrapper, t('connection.connect')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.liveConnect)?.params).toMatchObject({ uniqueId: 'luna' });
  });

  it('offers recent creators and a random room', async () => {
    const { wrapper, connection, calls } = await mountPage();
    await wrapper.get('button[aria-label="Recent creators"]').trigger('click');
    await nextTick();
    const items = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'));
    expect(items.map((item) => item.textContent?.trim())).toEqual(['@luna', '@mango']);
    items[1]!.click();
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.liveConnect)?.params).toMatchObject({ uniqueId: 'mango' });
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
    // The creator lives in the (locked) field and the status in the panel header: no sentence repeats them.
    expect(wrapper.text()).not.toContain('Receiving events from');
    await buttonByText(wrapper, 'Disconnect').trigger('click');
    await flushPromises();
    expect(calls.some((call) => call.method === 'live.disconnect')).toBe(true);
  });

  it.each<ConnectionStatus>(['connecting', 'retrying'])('cancels while %s and unlocks the form', async (status) => {
    const { wrapper, connection, calls } = await mountPage(status);
    expect(wrapper.get('#live-handle').attributes('disabled')).toBeDefined();
    const cancel = buttonByText(wrapper, t('common.cancel'));
    expect(cancel.attributes('disabled')).toBeUndefined();
    await cancel.trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === 'live.disconnect')).toHaveLength(1);
    expect(connection.status).toBe('disconnected');
    expect(wrapper.get('#live-handle').attributes('disabled')).toBeUndefined();
    expect(buttonByText(wrapper, t('connection.connect'))).toBeDefined();
  });

  it('keeps the explorer visible but only browses after explicit request', async () => {
    const { wrapper, calls } = await mountPage();
    expect(calls).toHaveLength(0);
    expect(buttonByText(wrapper, t('connections.explore.open')).isVisible()).toBe(true);
    await buttonByText(wrapper, t('connections.explore.open')).trigger('click');
    await flushPromises();
    expect(calls[0]?.method).toBe(RPC.liveChannels);
    const rows = wrapper.findAll('.live-explorer tbody tr');
    expect(rows.map((row) => row.text())).toEqual([expect.stringContaining('@luna'), expect.stringContaining('@mango')]);
    expect(rows[0]!.text()).toContain('1,520');
    await rows[1]!.get('button').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.liveConnect)?.params).toMatchObject({ uniqueId: 'mango' });
  });

  it('points to plugins when no server connection exists', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.text()).toContain(t('connections.servers.emptyTitle'));
    expect(buttonByText(wrapper, t('connections.servers.openPlugins')).exists()).toBe(true);
  });

  it('keeps Cloud webhooks inside the bottom panel and out of the page otherwise', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.find('button[aria-label="Webhooks"]').exists()).toBe(false);
    expect(wrapper.find('.cloud-connections').exists()).toBe(false);

    const announced = await mountPage('idle', [RPC.integrationsConnections]);
    const panels = announced.wrapper.findAll('.base-panel').length;
    await announced.wrapper.get('button[aria-label="Webhooks"]').trigger('click');
    await flushPromises();
    const servers = announced.wrapper.get('.connections-page__servers');
    expect(servers.find('.cloud-connections').exists()).toBe(true);
    expect(servers.get('.base-panel__title').text()).toBe(t('connections.tabs.webhooks'));
    // The same panel switched area: no third panel was stacked.
    expect(announced.wrapper.findAll('.base-panel')).toHaveLength(panels);
    expect(announced.calls.some((call) => call.method === RPC.integrationsConnections)).toBe(true);
  });

  it('starts the monitor with the typed creator and follows the host when it stops', async () => {
    const { wrapper, calls, connection } = await mountPage();
    await wrapper.get('#live-handle').setValue('mango');
    const monitorSwitch = wrapper.findAll<HTMLInputElement>('input[role="switch"]')[1]!;
    await monitorSwitch.setValue(true);
    await buttonByText(wrapper, t('connection.connect')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'live.monitor.start')?.params).toMatchObject({ uniqueId: 'mango' });
    expect(calls.some((call) => call.method === RPC.liveConnect)).toBe(false);
    expect(monitorSwitch.element.checked).toBe(true);

    const status = (enabled: boolean): LiveMonitorStatus => ({
      enabled, uniqueId: 'mango', phase: enabled ? 'waiting' : 'stopped',
      pollIntervalSeconds: 30, checks: 0, failures: 0,
      lastCheckedAt: null, nextCheckAt: null, lastError: null,
    });
    connection.monitorStatus = status(true);
    await nextTick();
    expect(monitorSwitch.element.checked).toBe(true);
    // The host can stop on its own (e.g. a manual disconnect elsewhere).
    connection.monitorStatus = status(false);
    await nextTick();
    expect(monitorSwitch.element.checked).toBe(false);
  });
});

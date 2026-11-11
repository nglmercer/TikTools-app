import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginServersPanel from './PluginServersPanel.vue';

enableAutoUnmount(afterEach);

const plugin = (id: string, name: string) => ({
  installed: true,
  enabled: true,
  descriptor: { id, name, hasConnectionProbe: true },
});

vi.mock('@/features/manage-plugins/model/use-plugin-controls.ts', () => ({
  usePluginControls: () => ({
    behavior: ref({ plugins: [plugin('peak', 'PEAK Game'), plugin('repo', 'R.E.P.O. Game')] }),
    connections: ref({ peak: { ok: true, latencyMs: 4, at: 1 }, repo: { ok: false, reachable: false, latencyMs: 0, at: 1 } }),
    settings: ref({}),
    provision: ref({}),
    actionOptions: ref({}),
    openMediaPicker: undefined,
    loadSettings: () => {},
    loadOptions: () => {},
    saveSettings: () => {},
    testConnection: () => {},
    importToken: () => {},
    provisionToken: () => {},
  }),
}));

const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/:any(.*)*', component: { render: () => null } }] });
const editorStub = { name: 'PluginConnectionEditor', props: ['pluginId'], template: '<div class="editor-stub">{{ pluginId }}</div>' };

function mountPanel(selected: string | null = null, announced: string[] = []) {
  const { client } = createFakeControlClient((method) => {
    if (method === 'rpc.discover') return { methods: announced.map((name) => ({ name })) };
    if (method === 'integrations.connections') return { items: [] };
    return {};
  });
  const stores = createStoreHarness(client);
  const wrapper = mountWithPlugins(PluginServersPanel, {
    props: { selected, 'onUpdate:selected': (value: string | null) => void wrapper.setProps({ selected: value }) } as never,
    global: { plugins: [router, ...stores.plugins], stubs: { PluginConnectionEditor: editorStub } },
  });
  return { wrapper, stores };
}

async function mountAnnounced(announced: string[]) {
  const mounted = mountPanel(null, announced);
  await mounted.stores.use(() => useCapabilityStore()).refresh('test-host');
  await flushPromises();
  return mounted.wrapper;
}

describe('PluginServersPanel', () => {
  it('lists the servers with one status each and no inline editors', () => {
    const { wrapper } = mountPanel();
    const rows = wrapper.findAll('.plugin-servers__item');
    expect(rows.map((row) => row.get('.plugin-servers__name').text())).toEqual(['PEAK Game', 'R.E.P.O. Game']);
    expect(wrapper.find('.editor-stub').exists()).toBe(false);
    expect(wrapper.find('[aria-expanded]').exists()).toBe(false);
  });

  it('opens a server inside the same panel and goes back to the list', async () => {
    const { wrapper } = mountPanel();
    await wrapper.findAll('.plugin-servers__open')[1]!.trigger('click');
    await flushPromises();
    expect(wrapper.emitted('update:selected')?.at(-1)).toEqual(['repo']);
    expect(wrapper.get('.editor-stub').text()).toBe('repo');
    expect(wrapper.find('.plugin-servers').exists()).toBe(false);
    expect(wrapper.text()).toContain('R.E.P.O. Game');
    await wrapper.get('button[aria-label="Back to server connections"]').trigger('click');
    await flushPromises();
    expect(wrapper.emitted('update:selected')?.at(-1)).toEqual([null]);
    expect(wrapper.findAll('.plugin-servers__item')).toHaveLength(2);
  });

  it('falls back to the list when the selected server is gone', () => {
    const { wrapper } = mountPanel('missing');
    expect(wrapper.findAll('.plugin-servers__item')).toHaveLength(2);
    expect(wrapper.find('.editor-stub').exists()).toBe(false);
  });

  it('shows the connection status and the probe action in the panel header', async () => {
    const { wrapper } = mountPanel('peak');
    expect(wrapper.text()).toContain('Connected in 4 ms');
    const testButton = wrapper.findAll('button').find((button) => button.text() === 'Test');
    expect(testButton).toBeDefined();
    await testButton!.trigger('click');
    expect(wrapper.text()).toContain('Testing…');
  });

  it('offers webhooks as another area of the same panel only when the host announces them', async () => {
    const without = await mountAnnounced([]);
    expect(without.find('button[aria-label="Webhooks"]').exists()).toBe(false);

    const wrapper = await mountAnnounced(['integrations.connections']);
    expect(wrapper.get('button[aria-label="Servers"]').attributes('aria-pressed')).toBe('true');
    await wrapper.get('button[aria-label="Webhooks"]').trigger('click');
    await flushPromises();
    expect(wrapper.get('.base-panel__title').text()).toBe('Webhooks');
    expect(wrapper.find('.cloud-connections').exists()).toBe(true);
    expect(wrapper.find('.plugin-servers').exists()).toBe(false);
    await wrapper.get('button[aria-label="Servers"]').trigger('click');
    expect(wrapper.findAll('.plugin-servers__item')).toHaveLength(2);
    expect(wrapper.find('.cloud-connections').exists()).toBe(false);
  });
});

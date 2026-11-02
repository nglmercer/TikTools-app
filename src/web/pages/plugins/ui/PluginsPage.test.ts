import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import type { ActionTypeDefinition, BehaviorSnapshot, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import type { ProcessorStatusEntry } from '../../../../shared/messages.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginsPage from './PluginsPage.vue';

enableAutoUnmount(afterEach);

const text = (value: string) => ({ default: value, i18key: '' });

beforeEach(() => {
  localStorage.clear();
});

function plugin(id: string, overrides: Partial<PluginStatus> = {}, descriptor: Partial<PluginStatus['descriptor']> = {}): PluginStatus {
  return {
    installed: true,
    enabled: true,
    available: true,
    ...overrides,
    descriptor: {
      id,
      source: 'user',
      name: text(id.toUpperCase()),
      version: '1.2.0',
      description: text(`The **${id}** integration.`),
      dependency: text('Local server'),
      permissions: ['network'],
      actionTypeIds: [`${id}.run`],
      eventTypeIds: [],
      ...descriptor,
    },
  };
}

const actionTypes = [
  { id: 'deck.run', title: 'Switch scene', tag: 'scenes', source: { kind: 'plugin', pluginId: 'deck' } },
] as unknown as ActionTypeDefinition[];

const actions = [{ id: 'a1', typeId: 'deck.run' }, { id: 'a2', typeId: 'deck.run' }] as unknown as LiveAction[];

const processor: ProcessorStatusEntry = {
  pluginId: 'deck',
  processorId: 'enrich',
  eventTypes: ['tiktok.chat'],
  timeoutMs: 250,
  priority: 1,
  status: 'ready',
  metrics: {
    calls: 12,
    successes: 11,
    failures: 1,
    timeouts: 0,
    skippedCircuitOpen: 0,
    skippedOverloaded: 0,
    averageLatencyMs: 4,
    maxLatencyMs: 9,
    lastError: 'boom',
  },
} as ProcessorStatusEntry;

const remoteEntry = { registry: 'official', id: 'acme.tts', name: 'Acme TTS', latest: '1.2.0', runtime: 'process', updatedAt: 0 };

const remoteDocument = {
  id: 'acme.tts',
  name: 'Acme TTS',
  publisher: 'acme',
  description: 'Reads chat aloud.',
  'dist-tags': { stable: '1.2.0' },
  releases: [{ version: '1.2.0', channel: 'stable', publishedAt: 0, yanked: false, runtime: 'process', permissions: [], capabilities: [] }],
};

const preview = {
  id: 'x.demo',
  name: 'X Demo',
  version: '1.0.0',
  runtime: 'process',
  securityModel: 'isolated',
  permissions: ['audio.output'],
  capabilities: [],
  size: 2048,
  digest: 'a'.repeat(64),
  origin: { kind: 'file' },
  signature: { status: 'unsigned' },
  change: 'install',
  permissionDiff: { addedPermissions: [], removedPermissions: [], addedCapabilities: [], removedCapabilities: [] },
  requiresSideloadConsent: true,
  metadata: {},
  consentToken: 'token-1',
};

async function mountPage(plugins: PluginStatus[] = [plugin('deck'), plugin('tts', { installed: false, enabled: false })]) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'processors.status') return { processors: [processor] };
    if (method === 'processors.test') {
      return { pluginId: 'deck', processorId: 'enrich', ok: true, durationMs: 3, result: { enriched: true } };
    }
    if (method === 'media.pick') return { selection: { type: 'file', file: { path: 'C:/x.plugin', name: 'x.plugin' } } };
    if (method === 'plugins.packages.receipts') return { receipts: [] };
    if (method === 'plugins.jobs.list') return { jobs: [] };
    if (method === 'plugins.updates.list') return { updates: [], checkedAt: 0 };
    if (method === 'registry.search') return { entries: [remoteEntry], total: 1, page: 1, pages: 1 };
    if (method === 'registry.package') return { registry: 'official', document: remoteDocument };
    if (method === 'plugins.packages.inspect') return preview;
    if (method === 'plugins.packages.install') {
      return { job: { id: 'job-1', kind: 'install', stage: 'queued', status: 'queued', createdAt: 1, updatedAt: 1 } };
    }
    return {};
  });
  const stores = createStoreHarness(client);
  const automation = stores.use(() => useAutomationStore()) as unknown as { behavior: BehaviorSnapshot };
  automation.behavior = { plugins, actions, actionTypes, events: [], translations: {} };
  const pluginsStore = stores.use(() => usePluginsStore());
  const wrapper = mountWithPlugins(PluginsPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await flushPromises();
  return { wrapper, calls, pluginsStore };
}

type Wrapper = VueWrapper;

function buttonByText(wrapper: Wrapper, text: string) {
  return wrapper.findAll('button').find((button) => button.text() === text)!;
}

async function selectPlugin(wrapper: Wrapper, name: string) {
  const option = wrapper.findAll('[role="option"]').find((item) => item.text().includes(name))!;
  await option.trigger('pointerdown');
}

describe('PluginsPage', () => {
  it('opens plugin details and returns to the catalog on narrow screens', async () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener: () => {}, removeEventListener: () => {} }));
    try {
      const { wrapper } = await mountPage();
      await wrapper.get('.plugin-catalog__item').trigger('pointerdown');
      expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
      expect(wrapper.find('.plugin-catalog').exists()).toBe(false);
      await wrapper.get('.plugins-page__back').trigger('click');
      expect(wrapper.find('.plugin-details').exists()).toBe(false);
      expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
      await wrapper.get('.plugin-catalog__item').trigger('pointerdown');
      expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
    } finally {
      vi.unstubAllGlobals();
    }
  });
  it('lists installed plugins apart from bundled and registry plugins', async () => {
    const { wrapper, calls } = await mountPage();
    expect(wrapper.findAll('[role="option"]').map((item) => item.text())).toEqual([expect.stringContaining('DECK')]);
    await buttonByText(wrapper, 'Browse').trigger('click');
    await flushPromises();
    expect(calls.map((call) => call.method)).toContain('registry.search');
    expect(wrapper.findAll('[role="option"]').map((item) => item.text())).toEqual([
      expect.stringContaining('TTS'),
      expect.stringContaining('Acme TTS'),
    ]);
  });

  it('filters the catalog by search', async () => {
    const { wrapper } = await mountPage([plugin('deck'), plugin('vts')]);
    await wrapper.get('input[type="search"]').setValue('scenes');
    expect(wrapper.findAll('[role="option"]')).toHaveLength(1);
    await wrapper.get('input[type="search"]').setValue('nothing');
    expect(wrapper.text()).toContain('No plugins match “nothing”.');
  });

  it('shows details and toggles the plugin', async () => {
    const { wrapper, calls } = await mountPage();
    expect(wrapper.text()).toContain('Select a plugin to see its details.');
    await selectPlugin(wrapper, 'DECK');
    expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
    expect(wrapper.find('.base-markdown strong').text()).toBe('deck');
    expect(wrapper.text()).toContain('Switch scene');
    await wrapper.get('.plugin-details__controls input[type="checkbox"]').setValue(false);
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });
  });

  it('confirms uninstall with the action usage', async () => {
    const { wrapper, calls } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    await buttonByText(wrapper, 'Uninstall…').trigger('click');
    await nextTick();
    expect(document.body.textContent).toContain('2 actions use it and will stop working.');
    const confirm = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.textContent?.trim() === 'Uninstall')!;
    confirm.click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.uninstall')?.params).toEqual({ pluginId: 'deck', purgeData: false });
  });

  it('activates a bundled plugin', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'Browse').trigger('click');
    await flushPromises();
    await selectPlugin(wrapper, 'TTS');
    await buttonByText(wrapper, 'Activate').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.install.set')?.params).toEqual({ pluginId: 'tts', installed: true });
  });

  it('reviews a package file and requires sideload consent before installing', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'Install .plugin…').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.packages.inspect')?.params).toEqual({
      spec: { kind: 'file', path: 'C:/x.plugin' },
    });
    expect(wrapper.text()).toContain('Install X Demo');
    expect(wrapper.text()).toContain('Play audio');
    expect(wrapper.text()).toContain('Not signed');
    expect(buttonByText(wrapper, 'Install').attributes('disabled')).toBeDefined();
    await wrapper.findAll('input[type="checkbox"]').at(-1)!.setValue(true);
    await buttonByText(wrapper, 'Install').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.packages.install')?.params).toEqual({
      spec: { kind: 'file', path: 'C:/x.plugin' },
      consentToken: 'token-1',
    });
  });

  it('reviews registry plugins from the browse tab', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'Browse').trigger('click');
    await flushPromises();
    await selectPlugin(wrapper, 'Acme TTS');
    await flushPromises();
    expect(calls.find((call) => call.method === 'registry.package')?.params).toEqual({ id: 'acme.tts', registry: 'official' });
    expect(wrapper.text()).toContain('Reads chat aloud.');
    await buttonByText(wrapper, 'Review and install').trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === 'plugins.packages.inspect').at(-1)?.params).toEqual({
      spec: { kind: 'registry', id: 'acme.tts', registry: 'official' },
    });
  });

  it('shows processor metrics and runs a test', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'Processors').trigger('click');
    await flushPromises();
    expect(calls.map((call) => call.method)).toContain('processors.status');
    const row = wrapper.get('.processors tbody tr');
    expect(row.text()).toContain('enrich');
    expect(row.text()).toContain('Ready');
    await row.trigger('pointerdown');
    expect(wrapper.text()).toContain('boom');
    await buttonByText(wrapper, 'Test with sample event').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'processors.test')?.params).toMatchObject({ pluginId: 'deck', processorId: 'enrich' });
    expect(wrapper.text()).toContain('Succeeded in 3 ms');
  });

  it('restores the selected plugin from the view state', async () => {
    localStorage.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, plugins: { selected: 'deck', view: 'plugins' } }));
    const { wrapper } = await mountPage();
    expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
  });
});

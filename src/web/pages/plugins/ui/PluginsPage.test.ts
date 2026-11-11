import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import type { ActionTypeDefinition, BehaviorSnapshot, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import type { ProcessorStatusEntry } from '../../../../shared/messages.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
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

async function mountPage(
  plugins: PluginStatus[] = [plugin('deck'), plugin('tts', { installed: false, enabled: false })],
  registries: unknown[] | undefined = undefined,
  liveStatuses: Record<string, string> = { '1.2.0': 'approved' },
  /** Methods the host announces through `rpc.discover`, as a host with the unified catalog does. */
  announce: string[] = [],
) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'rpc.discover') return { methods: announce.map((name) => ({ name })) };
    if (method === 'catalog.items') return { items: [], nextCursor: null };
    if (method === 'registry.list' && registries) return { registries };
    if (method === 'processors.status') return { processors: [processor] };
    if (method === 'processors.test') {
      return { pluginId: 'deck', processorId: 'enrich', ok: true, durationMs: 3, result: { enriched: true } };
    }
    if (method === 'media.pick') return { selection: { type: 'file', file: { path: 'C:/x.plugin', name: 'x.plugin' } } };
    if (method === 'plugins.packages.receipts') return { receipts: [] };
    if (method === 'plugins.jobs.list') return { jobs: [] };
    if (method === 'plugins.packages.versions') return { id: 'deck', installed: '1.2.0', retained: [], available: [] };
    if (method === 'plugins.updates.list') return { updates: [], checkedAt: 0 };
    if (method === 'registry.search') return { entries: [remoteEntry], total: 1, page: 1, pages: 1 };
    if (method === 'registry.cloudCatalogPaginationCapabilities') return {
      schemaVersion: 1, summaryPath: '/api/v2/catalog/plugin-summaries',
      versionPagePath: '/api/v2/catalog/plugins/{id}/version-pages', maxItems: 50,
      maxBytes: 524288, versionOrder: 'ascii-version-target',
    };
    if (method === 'registry.cloudCatalogSummaries') return {
      generation: '9007199254740993', nextCursor: null,
      items: [{ id: 'cloud.test', metadataPath: '/metadata/cloud.test', latestStableVersion: null }],
    };
    if (method === 'registry.cloudCatalog') return {
      generation: '9007199254740993', nextCursor: null,
      items: [{ id: 'cloud.test', latestStableVersion: null, versions: [{
        version: '1.0.0', target: 'any', size: '1', yanked: false,
        keyRevoked: false, publisherApproved: false, advisory: null,
      }] }],
    };
    if (method === 'registry.package') return { registry: 'official', document: remoteDocument, liveStatuses };
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
  if (announce.length > 0) await stores.use(() => useCapabilityStore()).refresh('test-host');
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

async function openItemMenu(wrapper: Wrapper, name: string) {
  const option = wrapper.findAll('[role="option"]').find((item) => item.text().includes(name))!;
  await option.trigger('contextmenu');
  await nextTick();
}

function menuItems(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"], [role="menuitemcheckbox"]'));
}

function menuItem(label: string): HTMLElement {
  return menuItems().find((item) => item.textContent?.trim() === label)!;
}

describe('PluginsPage', () => {
  it('opens Cloud catalog discovery without an installation request', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'Cloud catalog').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('cloud.test');
    expect(calls.find((call) => call.method === 'registry.cloudCatalogSummaries')?.params).toEqual({ query: '' });
    expect(calls.some((call) => call.method === 'plugins.packages.install')).toBe(false);
  });

  it('opens plugin details and returns to the catalog on narrow screens', async () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener: () => {}, removeEventListener: () => {} }));
    try {
      const { wrapper } = await mountPage();
      await wrapper.get('.plugin-catalog__item').trigger('pointerdown');
      expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
      expect(wrapper.get('.master-detail__master').attributes('hidden')).toBeDefined();
      await wrapper.get('.master-detail__back button').trigger('click');
      expect(wrapper.get('.master-detail__detail').attributes('hidden')).toBeDefined();
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
    await buttonByText(wrapper, 'Explore').trigger('click');
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

  it('offers plugin commands from the context menu on a list item', async () => {
    const { wrapper, calls } = await mountPage();
    await openItemMenu(wrapper, 'DECK');
    expect(menuItems().map((item) => item.textContent?.trim())).toEqual([
      'Open', 'Enabled', 'Settings', 'Connection', 'Versions', 'Test connection', 'Uninstall…',
    ]);
    menuItem('Enabled').click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });

    await openItemMenu(wrapper, 'DECK');
    menuItem('Uninstall…').click();
    await flushPromises();
    expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
    expect(document.body.textContent).toContain('2 actions use it and will stop working.');
    Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.textContent?.trim() === 'Uninstall')!.click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.uninstall')?.params).toEqual({ pluginId: 'deck', purgeData: false });
  });

  it('opens a details tab from the context menu', async () => {
    const { wrapper } = await mountPage([plugin('deck'), plugin('vts')]);
    await selectPlugin(wrapper, 'DECK');
    await openItemMenu(wrapper, 'VTS');
    menuItem('Versions').click();
    await flushPromises();
    expect(wrapper.get('.plugin-details__name').text()).toBe('VTS');
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Versions');
  });

  it('removes non-package plugins from the installed list instead of uninstalling', async () => {
    const dev = plugin('devkit', { available: false, enabled: false }, { source: 'development' });
    const { wrapper, calls } = await mountPage([dev]);
    await selectPlugin(wrapper, 'DEVKIT');
    expect(buttonByText(wrapper, 'Uninstall…')).toBeUndefined();
    await buttonByText(wrapper, 'Remove from installed').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.install.set')?.params).toEqual({ pluginId: 'devkit', installed: false });

    await openItemMenu(wrapper, 'DEVKIT');
    expect(menuItems().map((item) => item.textContent?.trim())).toContain('Remove from installed');
    expect(menuItems().map((item) => item.textContent?.trim())).not.toContain('Uninstall…');
  });

  it('activates a bundled plugin', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'Explore').trigger('click');
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
    await buttonByText(wrapper, 'Explore').trigger('click');
    await flushPromises();
    await selectPlugin(wrapper, 'Acme TTS');
    await flushPromises();
    expect(calls.find((call) => call.method === 'registry.package')?.params).toEqual({ id: 'acme.tts', registry: 'official' });
    expect(wrapper.text()).toContain('Reads chat aloud.');
    await buttonByText(wrapper, 'Review and install').trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === 'plugins.packages.inspect').at(-1)?.params).toEqual({
      spec: { kind: 'registry', id: 'acme.tts', version: '1.2.0', registry: 'official' },
    });
  });

  it('blocks remote review when current status is missing or the key is revoked', async () => {
    for (const [liveStatuses, expected] of [
      [{}, 'Current status unavailable'],
      [{ '1.2.0': 'key-revoked' }, 'Signing key revoked'],
    ] as const) {
      const { wrapper, calls } = await mountPage(undefined, undefined, liveStatuses);
      await buttonByText(wrapper, 'Explore').trigger('click');
      await flushPromises();
      await selectPlugin(wrapper, 'Acme TTS');
      await flushPromises();
      expect(wrapper.text()).toContain(expected);
      expect(buttonByText(wrapper, 'Review and install').attributes('disabled')).toBeDefined();
      await buttonByText(wrapper, 'Review and install').trigger('click');
      expect(calls.some((call) => call.method === 'plugins.packages.inspect')).toBe(false);
      wrapper.unmount();
    }
  });

  it('explains that the marketplace is unpublished when no registry is available', async () => {
    const { wrapper, calls } = await mountPage(undefined, []);
    await buttonByText(wrapper, 'Explore').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain("The official marketplace isn't published yet.");
    expect(wrapper.find('input[type="search"]').exists()).toBe(false);
    expect(calls.map((call) => call.method)).toContain('registry.list');
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

  describe('on a host that announces the unified catalog', () => {
    const unified = (plugins?: PluginStatus[]) => mountPage(plugins, undefined, undefined, ['catalog.items']);
    const savedView = () => JSON.parse(localStorage.getItem('tiktok-live-view-state') ?? '{}').plugins?.view;

    it('opens on the catalog with no view selector and no standing paragraph', async () => {
      const { wrapper } = await unified();
      expect(wrapper.find('.catalog-toolbar').exists()).toBe(true);
      expect(wrapper.find('[role="radiogroup"], .segmented').exists()).toBe(false);
      expect(wrapper.text()).not.toContain('Plugins bring an external dependency');
      expect(wrapper.find('.plugins-page__toolbar').exists()).toBe(false);
    });

    it('reaches processors and installed plugins from the menu and returns to the catalog', async () => {
      const { wrapper } = await unified();
      await buttonByText(wrapper, 'Manage processors').trigger('click');
      await flushPromises();
      expect(wrapper.find('.processors').exists()).toBe(true);
      expect(wrapper.find('.catalog-toolbar').exists()).toBe(false);
      await buttonByText(wrapper, '‹ Catalog').trigger('click');
      expect(wrapper.find('.catalog-toolbar').exists()).toBe(true);
      await buttonByText(wrapper, 'Manage installed plugins').trigger('click');
      await flushPromises();
      expect(wrapper.text()).toContain('DECK');
      expect(wrapper.find('.catalog-toolbar').exists()).toBe(false);
    });

    it('moves the saved Cloud view to the catalog and ignores a stale default', async () => {
      localStorage.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, plugins: { view: 'cloud' } }));
      expect((await unified()).wrapper.find('.catalog-toolbar').exists()).toBe(true);
      localStorage.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, plugins: { view: 'plugins' } }));
      expect((await unified()).wrapper.find('.catalog-toolbar').exists()).toBe(true);
    });

    it('keeps the plugin workspace when a plugin was open', async () => {
      localStorage.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, plugins: { view: 'plugins', selected: 'deck' } }));
      const { wrapper } = await unified();
      expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
    });

    it('does not write the implicit view as a choice', async () => {
      const { wrapper } = await unified();
      expect(savedView()).toBeUndefined();
      await buttonByText(wrapper, 'Manage processors').trigger('click');
      await new Promise((resolve) => setTimeout(resolve, 400));
      expect(savedView()).toBe('processors');
    });
  });

  it('restores the selected plugin from the view state', async () => {
    localStorage.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, plugins: { selected: 'deck', view: 'plugins' } }));
    const { wrapper } = await mountPage();
    expect(wrapper.get('.plugin-details__name').text()).toBe('DECK');
  });
});

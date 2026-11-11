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
  updates: unknown[] = [],
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
    if (method === 'plugins.updates.list') return { updates, checkedAt: 0 };
    if (method === 'plugins.packages.update') {
      return { job: { id: 'job-2', kind: 'update', stage: 'queued', status: 'queued', createdAt: 1, updatedAt: 1 } };
    }
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

/** Icon buttons have no text: they are found by the accessible name they carry. */
function buttonByLabel(wrapper: Wrapper, label: string) {
  return wrapper.find<HTMLButtonElement>(`button[aria-label="${label}"]`);
}

function tabNames(wrapper: Wrapper, name = 'tab') {
  return wrapper.findAll(`[role="${name}"]`).map((item) => item.text());
}

async function openTab(wrapper: Wrapper, label: string) {
  await wrapper.findAll('[role="tab"]').find((tab) => tab.text().startsWith(label))!.trigger('click');
  await flushPromises();
}

function row(wrapper: Wrapper, name: string) {
  return wrapper.findAll('.plugin-catalog__row').find((item) => item.text().includes(name))!;
}

async function selectPlugin(wrapper: Wrapper, name: string) {
  await row(wrapper, name).get('.plugin-catalog__open').trigger('click');
  await flushPromises();
}

async function openItemMenu(wrapper: Wrapper, name: string) {
  await row(wrapper, name).trigger('contextmenu');
  await nextTick();
}

/** The name of the open plugin: the title of the panel that replaced the list. */
function detailsTitle(wrapper: Wrapper): string {
  return wrapper.get('.base-panel__title').text();
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
    await buttonByLabel(wrapper, 'Cloud catalog').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('cloud.test');
    expect(calls.find((call) => call.method === 'registry.cloudCatalogSummaries')?.params).toEqual({ query: '' });
    expect(calls.some((call) => call.method === 'plugins.packages.install')).toBe(false);
  });

  it('opens plugin details in the same panel and returns to the list, on narrow screens too', async () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener: () => {}, removeEventListener: () => {} }));
    try {
      const { wrapper } = await mountPage();
      await selectPlugin(wrapper, 'DECK');
      expect(detailsTitle(wrapper)).toBe('DECK');
      expect(wrapper.find('.plugin-catalog').exists()).toBe(false);
      await buttonByLabel(wrapper, 'Back').trigger('click');
      expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
      expect(wrapper.find('.plugin-details').exists()).toBe(false);
      await selectPlugin(wrapper, 'DECK');
      expect(detailsTitle(wrapper)).toBe('DECK');
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('shows one navigation area with icon tabs, a search field and icon tools', async () => {
    const { wrapper } = await mountPage();
    expect(tabNames(wrapper)).toEqual(['Explore', 'Installed 1', 'Updates', 'Processors']);
    expect(wrapper.find('.plugins-page__toolbar').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('Plugins bring an external dependency');
    expect(wrapper.get('h2').text()).toBe('Plugins');
    expect(wrapper.get('input[type="search"]').attributes('aria-label')).toBe('Search plugins…');
    expect(buttonByLabel(wrapper, 'Install .plugin…').exists()).toBe(true);
    expect(buttonByLabel(wrapper, 'Check for updates').exists()).toBe(false);
    // No master/detail, so no placeholder waiting for a selection either.
    expect(wrapper.text()).not.toContain('Select a plugin to see its details.');
    expect(wrapper.find('.master-detail').exists()).toBe(false);
  });

  it('lists installed plugins with a switch and counters, and flags only the ones that cannot load', async () => {
    const broken = plugin('hotkeys', { available: false, unavailableReason: 'No input access' }, { eventTypeIds: ['hotkey.pressed'], actionTypeIds: [] });
    const { wrapper, calls } = await mountPage([plugin('deck'), broken]);
    const rows = wrapper.findAll('.plugin-catalog__row');
    expect(rows).toHaveLength(2);
    expect(rows[0]!.text()).toContain('Plugin package · 1.2.0');
    expect(rows[0]!.get('[role="img"][aria-label="Actions: 1"]').text()).toBe('1');
    expect(rows[1]!.get('[role="img"][aria-label="Events: 1"]').text()).toBe('1');
    expect(rows[0]!.text()).not.toContain('Unavailable');
    expect(rows[1]!.text()).toContain('Unavailable');
    // One status per fact: the switch says enabled, so no "Active" text beside it.
    expect(wrapper.text()).not.toContain('Active');
    await rows[0]!.get('input[role="switch"]').setValue(false);
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });
    expect(rows[1]!.find('input[role="switch"][aria-label="Enable HOTKEYS"]').exists()).toBe(true);
  });

  it('lists installed plugins apart from bundled and registry plugins', async () => {
    const { wrapper, calls } = await mountPage();
    expect(wrapper.findAll('.plugin-catalog__row').map((item) => item.text())).toEqual([expect.stringContaining('DECK')]);
    await openTab(wrapper, 'Explore');
    expect(calls.map((call) => call.method)).toContain('registry.search');
    const rows = wrapper.findAll('.plugin-catalog__row');
    expect(rows.map((item) => item.text())).toEqual([expect.stringContaining('TTS'), expect.stringContaining('Acme TTS')]);
    expect(rows[0]!.find('input[role="switch"]').exists()).toBe(false);
    expect(buttonByLabel(wrapper, 'Install Acme TTS').exists()).toBe(true);
  });

  it('lists the updates with the versions and updates one with its icon button', async () => {
    const update = { id: 'deck', registry: 'official', current: '1.2.0', latest: '1.3.0', channel: 'stable', pinned: false, permissionDiff: {}, autoApplicable: false, currentStatus: 'approved', latestStatus: 'approved' };
    const { wrapper, calls } = await mountPage(undefined, undefined, undefined, [], [update]);
    expect(tabNames(wrapper)).toContain('Updates 1');
    await openTab(wrapper, 'Updates');
    expect(wrapper.get('.plugin-catalog__row').text()).toContain('1.2.0 → 1.3.0');
    expect(buttonByLabel(wrapper, 'Check for updates').exists()).toBe(true);
    expect(buttonByLabel(wrapper, 'Update all').attributes('disabled')).toBeUndefined();
    await buttonByLabel(wrapper, 'Update DECK').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.packages.update')?.params).toEqual({ pluginId: 'deck', version: '1.3.0' });
  });

  it('filters the catalog by search', async () => {
    const { wrapper } = await mountPage([plugin('deck'), plugin('vts')]);
    await wrapper.get('input[type="search"]').setValue('scenes');
    expect(wrapper.findAll('.plugin-catalog__row')).toHaveLength(1);
    await wrapper.get('input[type="search"]').setValue('nothing');
    expect(wrapper.text()).toContain('No plugins match “nothing”.');
  });

  it('shows details and toggles the plugin', async () => {
    const { wrapper, calls } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    expect(detailsTitle(wrapper)).toBe('DECK');
    expect(wrapper.get('.plugin-details .base-panel__status').text()).toBe('1.2.0');
    expect(wrapper.find('.base-markdown strong').text()).toBe('deck');
    expect(wrapper.text()).toContain('Switch scene');
    // The version is in the header, not repeated among the facts.
    expect(wrapper.text().match(/1\.2\.0/g)).toHaveLength(1);
    await wrapper.get('.plugin-details input[role="switch"]').setValue(false);
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });
  });

  it('offers only the detail tabs the plugin has use for, each with its icon', async () => {
    const server = plugin('srv', {}, { hasSettings: true, hasConnectionProbe: true });
    const builtin = plugin('core', {}, { source: 'builtin' });
    const { wrapper } = await mountPage([server, builtin]);
    await selectPlugin(wrapper, 'SRV');
    expect(tabNames(wrapper)).toEqual(['General', 'Settings', 'Connection', 'Versions']);
    expect(wrapper.findAll('[role="tab"] svg')).toHaveLength(4);
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text() === 'Connection')!.trigger('click');
    // The same editor the Connections page uses, with its own status and test button.
    expect(wrapper.find('.connection-editor--sections').exists()).toBe(true);
    expect(wrapper.text()).toContain('Not configured');
    await buttonByLabel(wrapper, 'Back').trigger('click');
    await selectPlugin(wrapper, 'CORE');
    expect(tabNames(wrapper)).toEqual(['General']);
  });

  it('lays the general tab out as origin and permissions beside what the plugin adds', async () => {
    const { wrapper } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    const columns = wrapper.findAll('.plugin-details__column');
    expect(columns).toHaveLength(2);
    expect(columns[0]!.text()).toContain('Plugin package');
    expect(columns[0]!.text()).toContain('Local server');
    expect(columns[0]!.text()).toContain('network');
    expect(columns[1]!.text()).toContain('Switch scene');
  });

  it('explains in the open plugin why it cannot load, once', async () => {
    const broken = plugin('hotkeys', { available: false, unavailableReason: 'No input access' });
    const { wrapper } = await mountPage([broken]);
    await selectPlugin(wrapper, 'HOTKEYS');
    expect(wrapper.get('[role="alert"]').text()).toBe('This plugin cannot load on this machine: No input access');
    expect(wrapper.text().match(/No input access/g)).toHaveLength(1);
  });

  it('confirms uninstall with the action usage', async () => {
    const { wrapper, calls } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    await buttonByLabel(wrapper, 'Uninstall…').trigger('click');
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
    expect(detailsTitle(wrapper)).toBe('DECK');
    expect(document.body.textContent).toContain('2 actions use it and will stop working.');
    Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.textContent?.trim() === 'Uninstall')!.click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.uninstall')?.params).toEqual({ pluginId: 'deck', purgeData: false });
  });

  it('opens a details tab from the context menu', async () => {
    const { wrapper } = await mountPage([plugin('deck'), plugin('vts')]);
    await openItemMenu(wrapper, 'VTS');
    menuItem('Versions').click();
    await flushPromises();
    expect(detailsTitle(wrapper)).toBe('VTS');
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Versions');
  });

  it('removes non-package plugins from the installed list instead of uninstalling', async () => {
    const dev = plugin('devkit', { available: false, enabled: false }, { source: 'development' });
    const { wrapper, calls } = await mountPage([dev]);
    await selectPlugin(wrapper, 'DEVKIT');
    expect(buttonByLabel(wrapper, 'Uninstall…').exists()).toBe(false);
    await buttonByLabel(wrapper, 'Remove from installed').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.install.set')?.params).toEqual({ pluginId: 'devkit', installed: false });

    await buttonByLabel(wrapper, 'Back').trigger('click');
    await openItemMenu(wrapper, 'DEVKIT');
    expect(menuItems().map((item) => item.textContent?.trim())).toContain('Remove from installed');
    expect(menuItems().map((item) => item.textContent?.trim())).not.toContain('Uninstall…');
  });

  it('activates a bundled plugin', async () => {
    const { wrapper, calls } = await mountPage();
    await openTab(wrapper, 'Explore');
    await selectPlugin(wrapper, 'TTS');
    await buttonByText(wrapper, 'Activate').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.install.set')?.params).toEqual({ pluginId: 'tts', installed: true });
  });

  it('reviews a package file and requires sideload consent before installing', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByLabel(wrapper, 'Install .plugin…').trigger('click');
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
    await openTab(wrapper, 'Explore');
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
      await openTab(wrapper, 'Explore');
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
    await openTab(wrapper, 'Explore');
    expect(wrapper.text()).toContain("The official marketplace isn't published yet.");
    expect(wrapper.find('input[type="search"]').exists()).toBe(false);
    expect(calls.map((call) => call.method)).toContain('registry.list');
  });

  it('shows processor metrics and runs a test', async () => {
    const { wrapper, calls } = await mountPage();
    await openTab(wrapper, 'Processors');
    expect(calls.map((call) => call.method)).toContain('processors.status');
    const entry = wrapper.get('.processors__row');
    expect(entry.text()).toContain('enrich');
    expect(entry.text()).toContain('DECK');
    // A healthy processor needs no status in the list; its detail has one.
    expect(entry.text()).not.toContain('Ready');
    await entry.get('.processors__open').trigger('click');
    // The detail replaces the list and the tabs, with a way back.
    expect(wrapper.find('.processors').exists()).toBe(false);
    expect(wrapper.find('[role="tablist"]').exists()).toBe(false);
    expect(detailsTitle(wrapper)).toBe('enrich');
    expect(wrapper.get('.base-panel__status').text()).toContain('Ready');
    expect(wrapper.text()).toContain('boom');
    await buttonByText(wrapper, 'Test with sample event').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'processors.test')?.params).toMatchObject({ pluginId: 'deck', processorId: 'enrich' });
    expect(wrapper.text()).toContain('Succeeded in 3 ms');
    await buttonByLabel(wrapper, 'Back').trigger('click');
    expect(wrapper.find('.processors__row').exists()).toBe(true);
  });

  describe('on a host that announces the unified catalog', () => {
    const unified = (plugins?: PluginStatus[]) => mountPage(plugins, undefined, undefined, ['catalog.items']);
    const savedView = () => JSON.parse(localStorage.getItem('tiktok-live-view-state') ?? '{}').plugins?.view;

    it('opens on the catalog with no view selector and no standing paragraph', async () => {
      const { wrapper } = await unified();
      expect(wrapper.find('.catalog-toolbar').exists()).toBe(true);
      expect(wrapper.find('[role="radiogroup"], .segmented').exists()).toBe(false);
      expect(wrapper.text()).not.toContain('Plugins bring an external dependency');
    });

    it('reaches processors and installed plugins from the menu and returns to the catalog', async () => {
      const { wrapper } = await unified();
      await buttonByText(wrapper, 'Manage processors').trigger('click');
      await flushPromises();
      expect(wrapper.find('.processors').exists()).toBe(true);
      expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Processors');
      expect(wrapper.find('.catalog-toolbar').exists()).toBe(false);
      await buttonByLabel(wrapper, 'Catalog').trigger('click');
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
      expect(detailsTitle(wrapper)).toBe('DECK');
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
    expect(detailsTitle(wrapper)).toBe('DECK');
  });
});

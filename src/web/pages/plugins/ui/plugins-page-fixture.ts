import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import type { ActionTypeDefinition, BehaviorSnapshot, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import type { ProcessorStatusEntry } from '../../../../shared/messages.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginsPage from './PluginsPage.vue';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

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

/** A host that announces its methods but not the unified catalog: Explore falls back to the registry. */
const REGISTRY_HOST = ['plugins.updates.list', 'sync.status', 'sync.v2.changes.list', 'sync.v2.section', 'cloud.status'];

async function mountPage(
  plugins: PluginStatus[] = [plugin('deck'), plugin('tts', { installed: false, enabled: false })],
  registries: unknown[] | undefined = undefined,
  liveStatuses: Record<string, string> = { '1.2.0': 'approved' },
  /** Methods the host announces through `rpc.discover`, as a host with the unified catalog does. */
  announce: string[] = REGISTRY_HOST,
  updates: unknown[] = [],
  /** Route query the page is opened with, as a link from elsewhere would. */
  query = '',
  /** Answers first, for methods a test wants to control (the Cloud catalog). */
  handle?: (method: string, params: Record<string, unknown> | undefined) => unknown,
) {
  const { client, calls } = createFakeControlClient((method, params) => {
    const handled = handle?.(method, params);
    if (handled !== undefined) return handled;
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
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/plugins', component: { render: () => null } }] });
  await router.push(`/plugins${query}`);
  const wrapper = mountWithPlugins(PluginsPage, { attachTo: document.body, global: { plugins: [router, ...stores.plugins] } });
  await flushPromises();
  return { wrapper, calls, pluginsStore, router };
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


export { testMessage, enableAutoUnmount, flushPromises, afterEach, beforeEach, describe, expect, it, vi, nextTick, createMemoryHistory, createRouter, useAutomationStore, usePluginsStore, useCapabilityStore, ControlCallError, createFakeControlClient, createStoreHarness, mountWithPlugins, PluginsPage, type VueWrapper, type ActionTypeDefinition, type BehaviorSnapshot, type LiveAction, type PluginStatus, type ProcessorStatusEntry };

export { REGISTRY_HOST, t, text, plugin, actionTypes, actions, remoteEntry, remoteDocument, preview, mountPage, buttonByText, buttonByLabel, tabNames, openTab, row, selectPlugin, openItemMenu, detailsTitle, menuItems, menuItem };

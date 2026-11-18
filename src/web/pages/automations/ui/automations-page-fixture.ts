import { resetResolutionPreference } from '@/features/review-shared/model/resolution-preference.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, expect, vi } from 'vitest';
import { nextTick } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import type { ActionTypeDefinition, BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { ControlCallError, type ControlCallOptions, type RequestMetadata, type RpcParams } from '@/shared/api/control-client.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import OwnerLibraryReview from '@/features/recover-owner-library/ui/OwnerLibraryReview.vue';
import AutomationsPage from './AutomationsPage.vue';
import { testMessage } from '@/shared/i18n/test-message.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

beforeEach(() => {
  localStorage.clear();
  resetResolutionPreference('manual');
});
afterEach(() => resetResolutionPreference('automatic'));

const text = (value: string) => ({ default: value, i18key: '' });

const actionTypes = [
  {
    id: 'core.log',
    title: text('Write to log'),
    description: text('Writes a line to the console.'),
    tag: 'log',
    source: { kind: 'builtin' },
    requiredCapabilities: [],
    configSchema: { type: 'object', properties: { message: { type: 'string', title: 'Message', default: 'hello' } } },
    uiHints: { fields: { message: { template: true } } },
  },
  ...['core.points', 'core.fetch'].map((id) => ({
    id,
    title: text(id),
    description: text(id),
    tag: id,
    source: { kind: 'builtin' },
    requiredCapabilities: [],
  })),
] as unknown as ActionTypeDefinition[];

const logAction: LiveAction = { schemaVersion: 2, id: 'a1', name: 'Log gift', typeId: 'core.log', enabled: true, config: { message: 'gift!' } };
const giftEvent: LiveEvent = {
  schemaVersion: 1,
  id: 'e1',
  name: 'Big gifts',
  enabled: true,
  trigger: 'tiktok.gift',
  filters: [{ path: 'event.data.giftName', operator: 'eq', value: 'Rose' }],
  cooldownMs: 0,
  cooldownScope: 'user',
  actionIds: ['a1'],
  runMode: 'all',
};
const chatEvent: LiveEvent = { ...giftEvent, id: 'e2', name: 'Chat hello', trigger: 'tiktok.chat', filters: [], actionIds: [] };

const runs = [
  { id: 'r1', at: Date.now(), status: 'ok', actionId: 'a1', actionName: 'Log gift', eventName: 'Big gifts', summary: 'logged', durationMs: 3, test: false, logs: [] },
];

interface MountExtras {
  /** Methods the host announces through `rpc.discover`. */
  announce?: string[];
  /** Route query the page is opened with, as a link from elsewhere would. */
  query?: string;
}

async function mountPage(intercept?: (method: string, params: RpcParams | undefined, options?: ControlCallOptions) => unknown, waitForInitial = true, extras: MountExtras = {}) {
  const snapshot: BehaviorSnapshot = { actions: [logAction], events: [giftEvent, chatEvent], plugins: [], actionTypes, translations: {} };
  const versions = new Map([['action:a1', '1'], ['event:e1', '1'], ['event:e2', '1']]);
  const requests: Array<{ method: string; meta?: RequestMetadata }> = [];
  const { client, calls } = createFakeControlClient(nativeDefinitionHost((method, params, options) => {
    requests.push({ method, meta: options?.meta });
    const overridden = intercept?.(method, params, options);
    if (overridden !== undefined) return overridden;
    if (method === 'rpc.discover') return { methods: (extras.announce ?? []).map((name) => ({ name })) };
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'automation.snapshot') {
      options?.onMetadata?.({ resourceVersions: [
        ...snapshot.actions.map((r) => ({ kind: 'automation', scope: 'action', id: r.id, revision: versions.get(`action:${r.id}`)! })),
        ...snapshot.events.map((r) => ({ kind: 'automation', scope: 'event', id: r.id, revision: versions.get(`event:${r.id}`)! })),
      ] });
      return structuredClone(snapshot);
    }
    if (method === 'automation.runs') return { runs };
    if (method === 'app.state.get') return { state: {} };
    if (['automation.get', 'automation.create', 'automation.update', 'automation.delete', 'automation.enable', 'automation.disable'].includes(method)) {
      const kind = params?.kind as 'event' | 'action';
      const record = params?.record as LiveAction | LiveEvent | undefined;
      const id = (params?.id ?? record?.id) as string;
      const list = (kind === 'action' ? snapshot.actions : snapshot.events) as Array<LiveAction | LiveEvent>;
      const key = `${kind}:${id}`;
      const version = versions.get(key) ?? '0';
      const index = list.findIndex((r) => r.id === id);
      const metadata = () => ({ resourceVersions: [{ kind: 'automation', scope: kind, id, revision: versions.get(key) ?? '0' }] });
      if (method === 'automation.get') {
        if (index < 0) throw new ControlCallError('automation_not_found', 'Deleted on host', metadata());
        options?.onMetadata?.(metadata()); return list[index];
      }
      if (options?.meta?.expectedRevision !== version) throw new ControlCallError('revision_conflict', 'Host changed', metadata());
      if (method === 'automation.create' && index >= 0) throw new ControlCallError('revision_conflict', 'Already present', metadata());
      if (method === 'automation.update' && index < 0) throw new ControlCallError('automation_not_found', 'Deleted on host', metadata());
      versions.set(key, String(BigInt(version) + 1n));
      if (method === 'automation.delete') {
        if (index >= 0) list.splice(index, 1);
        options?.onMetadata?.(metadata()); return { id, deleted: index >= 0 };
      }
      const next = structuredClone(record ?? { ...list[index]!, enabled: method === 'automation.enable' });
      if (index < 0) list.push(next); else list[index] = next;
      options?.onMetadata?.(metadata()); return next;
    }
    return {};
  }).handle);
  const stores = createStoreHarness(client);
  const automation = stores.use(() => useAutomationStore());
  const initializing = automation.refresh();
  if (waitForInitial) await initializing;
  stores.use(() => useLiveFeedStore()).giftCatalog = [{ id: '1', name: 'Rose', diamondCount: 1 }];
  if (extras.announce?.length) await stores.use(() => useCapabilityStore()).refresh('test-host');
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/automations', component: { render: () => null } }] });
  await router.push(`/automations${extras.query ?? ''}`);
  const wrapper = mountWithPlugins(AutomationsPage, { attachTo: document.body, global: { plugins: [router, ...stores.plugins] } });
  // The app shell mounts the one library review dialog; the page only starts the requests.
  mountWithPlugins(OwnerLibraryReview, { attachTo: document.body, global: { plugins: [router, ...stores.plugins] } });
  await flushPromises();
  calls.length = 0;
  return { wrapper, calls, requests, snapshot, versions, automation, client, initializing, router };
}

// Icon-only buttons are found by their accessible label, the rest by their text.
function buttonByText(wrapper: VueWrapper, label: string) {
  return wrapper.findAll('button').find((button) => button.text() === label || button.attributes('aria-label') === label)!;
}

// Icon-only buttons are named by their accessible label.
function buttonByLabel(wrapper: VueWrapper, label: string) {
  return wrapper.get(`button[aria-label="${label}"]`);
}

// The open record replaces the library; this is the way back (it asks first when the draft is dirty).
async function goBack(wrapper: VueWrapper) {
  await buttonByLabel(wrapper, t('common.back')).trigger('click');
  await nextTick();
}

function documentButton(label: string): HTMLButtonElement {
  return Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.textContent?.trim() === label)!;
}

async function selectRow(wrapper: VueWrapper, name: string) {
  await wrapper.findAll('.automation-library tbody tr').find((row) => row.text().includes(name))!.trigger('pointerdown');
  await nextTick();
}


export { resetResolutionPreference, enableAutoUnmount, flushPromises, afterEach, beforeEach, expect, vi, nextTick, createMemoryHistory, createRouter, useCapabilityStore, useAutomationStore, libraryJournal, ControlCallError, useLiveFeedStore, createFakeControlClient, createStoreHarness, mountWithPlugins, AutomationsPage, OwnerLibraryReview, testMessage, type VueWrapper, type ActionTypeDefinition, type BehaviorSnapshot, type LiveAction, type LiveEvent, type ControlCallOptions, type RequestMetadata, type RpcParams };

export { t, text, actionTypes, logAction, giftEvent, chatEvent, runs, mountPage, buttonByText, buttonByLabel, goBack, documentButton, selectRow };

export { describe, it } from 'vitest';

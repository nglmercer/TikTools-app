import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import type { ActionTypeDefinition, BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { ControlCallError, type ControlCallOptions, type RequestMetadata, type RpcParams } from '@/shared/api/control-client.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { proposalFixture } from '@/features/review-automation-proposal/testing/fixture.ts';
import AutomationsPage from './AutomationsPage.vue';

enableAutoUnmount(afterEach);

beforeEach(() => {
  localStorage.clear();
});

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

async function mountPage(intercept?: (method: string, params: RpcParams | undefined, options?: ControlCallOptions) => unknown) {
  const snapshot: BehaviorSnapshot = { actions: [logAction], events: [giftEvent, chatEvent], plugins: [], actionTypes, translations: {} };
  const versions = new Map([['action:a1', '1'], ['event:e1', '1'], ['event:e2', '1']]);
  const requests: Array<{ method: string; meta?: RequestMetadata }> = [];
  const { client, calls } = createFakeControlClient(nativeDefinitionHost((method, params, options) => {
    requests.push({ method, meta: options?.meta });
    const overridden = intercept?.(method, params, options);
    if (overridden !== undefined) return overridden;
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
  await automation.refresh();
  stores.use(() => useLiveFeedStore()).giftCatalog = [{ id: '1', name: 'Rose', diamondCount: 1 }];
  const wrapper = mountWithPlugins(AutomationsPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await flushPromises();
  calls.length = 0;
  return { wrapper, calls, requests, snapshot, versions, automation, client };
}

function buttonByText(wrapper: VueWrapper, label: string) {
  return wrapper.findAll('button').find((button) => button.text() === label)!;
}

function documentButton(label: string): HTMLButtonElement {
  return Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.textContent?.trim() === label)!;
}

async function selectRow(wrapper: VueWrapper, name: string) {
  await wrapper.findAll('.automation-library tbody tr').find((row) => row.text().includes(name))!.trigger('pointerdown');
  await nextTick();
}

describe('AutomationsPage', () => {
  it('lists events and opens the editor with a readable summary', async () => {
    const { wrapper } = await mountPage();
    const rows = wrapper.findAll('.automation-library tbody tr');
    expect(rows.map((row) => row.text())).toEqual([expect.stringContaining('Big gifts'), expect.stringContaining('Chat hello')]);
    expect(wrapper.text()).toContain('Nothing selected');
    await selectRow(wrapper, 'Big gifts');
    expect(wrapper.get('.event-editor__summary').text()).toBe('When someone sends a gift and Gift Name is Rose, run Log gift.');
  });

  it('sorts the library from the filter menu', async () => {
    const { wrapper } = await mountPage();
    const filter = wrapper.get('.automation-library__sort');
    expect(filter.classes()).not.toContain('automation-library__sort--active');
    await filter.trigger('click');
    await nextTick();
    const item = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitemcheckbox"]'))
      .find((entry) => entry.textContent?.trim() === 'Name Z–A')!;
    item.click();
    await flushPromises();
    expect(wrapper.findAll('.automation-library tbody tr').map((row) => row.text())).toEqual([
      expect.stringContaining('Chat hello'),
      expect.stringContaining('Big gifts'),
    ]);
    expect(wrapper.get('.automation-library__sort').classes()).toContain('automation-library__sort--active');
  });

  it('toggles a record from the table', async () => {
    const { wrapper, calls, requests } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    expect(calls.some((call) => call.method === 'automation.disable')).toBe(false);
    expect(document.body.textContent).toContain('e2 · version 1');
    documentButton('Confirm change').click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'automation.definition.submit')?.params).toMatchObject({ operation: 'disable', id: 'e2', kind: 'event', expectedRevision: '1' });
    expect(requests.find((call) => call.method === 'automation.disable')?.meta).toEqual({ expectedRevision: '1' });
  });

  it('creates and saves a new event', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'New event').trigger('click');
    await wrapper.get('.event-editor input[type="text"]').setValue('Followers');
    await buttonByText(wrapper, 'Save').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'automation.definition.submit')?.params).toMatchObject({ operation: 'create',
      kind: 'event',
      record: { name: 'Followers', trigger: 'tiktok.gift' },
    });
  });

  it('picks an action type and saves the new action', async () => {
    const { wrapper, calls } = await mountPage();
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text().startsWith('Actions'))!.trigger('click');
    await buttonByText(wrapper, 'New action…').trigger('click');
    await nextTick();
    Array.from(document.querySelectorAll<HTMLButtonElement>('.type-picker__card'))[0]!.click();
    await nextTick();
    expect(wrapper.find('.action-editor').exists()).toBe(true);
    await buttonByText(wrapper, 'Save').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'automation.definition.submit')?.params).toMatchObject({ operation: 'create',
      kind: 'action',
      record: { typeId: 'core.log', name: 'Write to log' },
    });
  });

  it('asks before leaving unsaved changes', async () => {
    const { wrapper } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Renamed');
    await selectRow(wrapper, 'Chat hello');
    expect(document.body.textContent).toContain('Discard changes?');
    documentButton('Discard').click();
    await nextTick();
    expect(wrapper.get('.event-editor .base-panel__title').text()).toBe('Chat hello');
  });

  it('confirms deleting an event', async () => {
    const { wrapper, calls } = await mountPage();
    await selectRow(wrapper, 'Chat hello');
    await buttonByText(wrapper, 'Delete…').trigger('click');
    await nextTick();
    documentButton('Delete').click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'automation.definition.submit')?.params).toMatchObject({ operation: 'delete', id: 'e2', kind: 'event', expectedRevision: '1' });
  });

  it('applies a built-in rule template', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'From template…').trigger('click');
    await nextTick();
    const card = Array.from(document.querySelectorAll<HTMLButtonElement>('.rule-templates__card')).find((entry) =>
      entry.textContent?.includes('Chat points'),
    )!;
    card.click();
    await nextTick();
    documentButton('Create event and actions').click();
    await flushPromises();
    const created = calls.filter((call) => call.method === 'automation.create').map((call) => call.params?.kind);
    expect(created).toEqual(['action', 'event']);
  });

  it('explains Run test and Fire now on hover', async () => {
    const { wrapper } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    vi.useFakeTimers();
    try {
      const runTest = buttonByText(wrapper, 'Run test');
      runTest.element.dispatchEvent(new Event('focusin', { bubbles: true }));
      vi.advanceTimersByTime(600);
      expect(document.getElementById('app-tooltip')?.textContent).toBe(
        'Dry run with sample data. No actions are executed.',
      );
      runTest.element.dispatchEvent(new Event('focusout', { bubbles: true }));
      buttonByText(wrapper, 'Fire now').element.dispatchEvent(new Event('focusin', { bubbles: true }));
      vi.advanceTimersByTime(600);
      expect(document.getElementById('app-tooltip')?.textContent).toBe(
        'Execute this event’s actions now, for real.',
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it('shows one type icon per action row', async () => {
    const { wrapper } = await mountPage();
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text().startsWith('Actions'))!.trigger('click');
    await nextTick();
    expect(wrapper.findAll('.automation-library tbody tr')).toHaveLength(1);
    expect(wrapper.findAll('.automation-library__type-icon')).toHaveLength(1);
  });

  it('shows recent runs', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.get('.automation-runs').text()).toContain('Log gift');
    expect(wrapper.get('.automation-runs').text()).toContain('logged');
  });

  it('opens the editor as a full-screen sheet with a back button on narrow screens', async () => {
    vi.stubGlobal('matchMedia', () => ({
      matches: true,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
    }));
    try {
      const { wrapper } = await mountPage();
      expect(wrapper.find('.automations-page__workspace').exists()).toBe(false);
      expect(wrapper.find('.automations-page__sheet').exists()).toBe(false);
      await selectRow(wrapper, 'Big gifts');
      const sheet = wrapper.find('.automations-page__sheet');
      expect(sheet.exists()).toBe(true);
      expect(sheet.attributes('role')).toBe('dialog');
      expect(sheet.get('.automations-page__sheet-title').text()).toBe('Big gifts');
      expect(wrapper.find('.automations-page__sheet .event-editor').exists()).toBe(true);
      await buttonByText(wrapper, 'Back').trigger('click');
      await nextTick();
      expect(wrapper.find('.automations-page__sheet').exists()).toBe(false);
      expect(wrapper.find('.automations-page__narrow').exists()).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('pins the base across background refreshes, preserves a conflicting draft and rebases without resubmitting', async () => {
    const { wrapper, snapshot, versions, automation, requests, calls } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('My draft');
    snapshot.events[0] = { ...giftEvent, name: 'Edited on host', enabled: false };
    versions.set('event:e1', '2');
    await automation.refresh(); await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('My draft');
    expect(wrapper.text()).toContain('changed since you opened it');
    await buttonByText(wrapper, 'Save').trigger('click'); await flushPromises();
    expect(requests.find((call) => call.method === 'automation.update')?.meta).toEqual({ expectedRevision: '1' });
    expect(wrapper.text()).toContain('Your draft is retained');
    expect((buttonByText(wrapper, 'Save').element as HTMLButtonElement).disabled).toBe(true);
    await buttonByText(wrapper, 'Compare with host').trigger('click'); await flushPromises();
    expect(document.body.textContent).toContain('Starting version');
    expect(document.body.textContent).toContain('Edited on host');
    expect(document.body.textContent).toContain('My draft');
    const writes = calls.filter((call) => call.method === 'automation.update').length;
    documentButton('Use this version as the draft base').click(); await flushPromises();
    expect(calls.filter((call) => call.method === 'automation.update')).toHaveLength(writes);
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('My draft');
    await buttonByText(wrapper, 'Save').trigger('click'); await flushPromises();
    expect(requests.filter((call) => call.method === 'automation.update').at(-1)?.meta).toEqual({ expectedRevision: '2' });
    expect(versions.get('event:e1')).toBe('3');
    expect(wrapper.text()).not.toContain('Unsaved changes');
  });

  it('does not reset even an untouched opening base when the host changes it', async () => {
    const { wrapper, snapshot, versions, automation, requests } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    snapshot.events[0] = { ...giftEvent, name: 'New host name' }; versions.set('event:e1', '2');
    await automation.refresh(); await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Edited from old base');
    await buttonByText(wrapper, 'Save').trigger('click'); await flushPromises();
    expect(requests.find((r) => r.method === 'automation.update')?.meta?.expectedRevision).toBe('1');
  });

  it('keeps a deleted record draft and never falls back from update to create', async () => {
    const { wrapper, snapshot, versions, automation, calls, requests } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Keep this text');
    snapshot.events.splice(0, 1); versions.set('event:e1', '2');
    await automation.refresh(); await flushPromises();
    expect(wrapper.find('.event-editor').exists()).toBe(true);
    await buttonByText(wrapper, 'Save').trigger('click'); await flushPromises();
    expect(calls.some((r) => r.method === 'automation.create')).toBe(false);
    await buttonByText(wrapper, 'Compare with host').trigger('click'); await flushPromises();
    expect(document.body.textContent).toContain('This ID is absent');
    documentButton('Prepare recreation with this draft').click(); await flushPromises();
    expect(calls.some((r) => r.method === 'automation.create')).toBe(false);
    await buttonByText(wrapper, 'Save').trigger('click'); await flushPromises();
    expect(requests.find((r) => r.method === 'automation.create')?.meta?.expectedRevision).toBe('2');
    expect(snapshot.events.some((r) => r.id === 'e1' && r.name === 'Keep this text')).toBe(true);
  });

  it('retains dirty text on a failed save and asks before navigation', async () => {
    const { wrapper, calls } = await mountPage((method) => {
      if (method === 'automation.update') return Promise.reject(new ControlCallError('timeout', 'Acknowledgement lost'));
      return undefined;
    });
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Unconfirmed draft');
    await buttonByText(wrapper, 'Save').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('outcome is unconfirmed');
    expect(wrapper.text()).toContain('Unsaved changes');
    expect(calls.filter((r) => r.method === 'automation.definition.submit' && r.params?.operation === 'update')).toHaveLength(1);
    await selectRow(wrapper, 'Chat hello');
    expect(document.body.textContent).toContain('Discard changes?');
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Unconfirmed draft');
  });

  it('preserves newer form edits when a delayed save is acknowledged', async () => {
    let finish!: (record: LiveEvent) => void;
    let submitted!: LiveEvent;
    const pending = new Promise<LiveEvent>((resolve) => { finish = resolve; });
    const { wrapper, snapshot, versions } = await mountPage((method, params, options) => {
      if (method !== 'automation.update') return undefined;
      submitted = params?.record as LiveEvent;
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'event', id: 'e1', revision: '2' }] });
      return pending;
    });
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Submitted text');
    await buttonByText(wrapper, 'Save').trigger('click');
    await wrapper.get('.event-editor input[type="text"]').setValue('Newer text');
    snapshot.events[0] = submitted; versions.set('event:e1', '2'); finish(submitted);
    await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Newer text');
    expect(wrapper.text()).toContain('Unsaved changes');
    expect((buttonByText(wrapper, 'Save').element as HTMLButtonElement).disabled).toBe(false);
  });

  it('a delayed acknowledgement cannot select or clear another editor after explicit navigation', async () => {
    let finish!: (record: LiveEvent) => void;
    let submitted!: LiveEvent;
    const pending = new Promise<LiveEvent>((resolve) => { finish = resolve; });
    const { wrapper, snapshot, versions } = await mountPage((method, params, options) => {
      if (method !== 'automation.update') return undefined;
      submitted = params?.record as LiveEvent;
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'event', id: 'e1', revision: '2' }] });
      return pending;
    });
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Pending first edit');
    await buttonByText(wrapper, 'Save').trigger('click');
    await selectRow(wrapper, 'Chat hello'); documentButton('Discard').click(); await flushPromises();
    await wrapper.get('.event-editor input[type="text"]').setValue('Second draft');
    snapshot.events[0] = submitted; versions.set('event:e1', '2'); finish(submitted); await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Second draft');
    expect(wrapper.text()).toContain('Unsaved changes');
  });

  it('failed deletion keeps the editor and its unsaved candidate', async () => {
    const { wrapper } = await mountPage((method) => method === 'automation.delete'
      ? Promise.reject(new ControlCallError('revision_conflict', 'Changed before delete')) : undefined);
    await selectRow(wrapper, 'Chat hello');
    await wrapper.get('.event-editor input[type="text"]').setValue('Keep after delete failed');
    await buttonByText(wrapper, 'Delete…').trigger('click'); await nextTick(); documentButton('Delete').click(); await flushPromises();
    expect(wrapper.find('.event-editor').exists()).toBe(true);
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Keep after delete failed');
    expect(wrapper.text()).toContain('revision_conflict');
  });

  it('canceling enablement restores the committed checkbox without a write', async () => {
    const { wrapper, calls } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    await nextTick(); documentButton('Cancel').click(); await flushPromises();
    expect((wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').element as HTMLInputElement).checked).toBe(true);
    expect(calls.some((r) => r.method === 'automation.disable')).toBe(false);
  });

  it('an enablement conflict requires a fresh read and another explicit confirmation', async () => {
    const { wrapper, snapshot, versions, requests } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    snapshot.events[1] = { ...chatEvent, name: 'New name before confirmation' }; versions.set('event:e2', '2');
    documentButton('Confirm change').click(); await flushPromises();
    expect(requests.find((r) => r.method === 'automation.disable')?.meta?.expectedRevision).toBe('1');
    expect(document.body.textContent).toContain('revision_conflict');
    documentButton('Read current host version').click(); await flushPromises();
    expect(document.body.textContent).toContain('New name before confirmation');
    expect(requests.filter((r) => r.method === 'automation.disable')).toHaveLength(1);
    documentButton('Confirm change').click(); await flushPromises();
    expect(requests.filter((r) => r.method === 'automation.disable').at(-1)?.meta?.expectedRevision).toBe('2');
  });

  it('reviews saved owner authority and adopts without saving, replacing or enabling the unsaved editor candidate', async () => {
    const native = proposalFixture();
    native.source.id = 'e1';
    native.source.record = structuredClone(giftEvent) as unknown as Record<string, unknown>;
    native.source.dependencies = [{ id: 'a1', record: structuredClone(logAction) as unknown as Record<string, unknown>,
      provenance: { authority: { kind: 'owner' }, generation: '1' } }];
    native.versions.splice(0, native.versions.length,
      { kind: 'automation', scope: 'event', id: 'e1', revision: '1' },
      { kind: 'automation', scope: 'action', id: 'a1', revision: '1' });
    const { wrapper, calls } = await mountPage((method, params, options) =>
      method === 'automation.review' || method === 'automation.approval.get' || method.startsWith('automation.adoption.') ? native.handle(method, params, options) : undefined);
    expect(buttonByText(wrapper, 'Review saved authority')).toBeUndefined();
    expect(buttonByText(wrapper, 'Adoption history').exists()).toBe(true);
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('My untouched unsaved draft');
    await buttonByText(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    const review = wrapper.get('.proposal-dialog');
    expect(review.text()).toContain('Big gifts');
    expect(review.text()).not.toContain('My untouched unsaved draft');
    await review.get('[data-test="adoption-acknowledge"]').setValue(true);
    await review.get('[data-test="adoption-submit"]').trigger('click'); await flushPromises();
    expect(calls.filter((r) => r.method === 'automation.adoption.submit')).toHaveLength(1);
    expect(calls.some((r) => ['automation.update', 'automation.create', 'automation.enable'].includes(r.method))).toBe(false);
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('My untouched unsaved draft');
    expect(wrapper.text()).toContain('Unsaved changes');
  });
});

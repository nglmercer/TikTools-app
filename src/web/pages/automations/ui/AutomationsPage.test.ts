import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import type { ActionTypeDefinition, BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
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

async function mountPage() {
  const snapshot: BehaviorSnapshot = { actions: [logAction], events: [giftEvent, chatEvent], plugins: [], actionTypes, translations: {} };
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'automation.snapshot') return snapshot;
    if (method === 'automation.runs') return { runs };
    if (method === 'app.state.get') return { state: {} };
    return {};
  });
  const stores = createStoreHarness(client);
  const automation = stores.use(() => useAutomationStore());
  await automation.refresh();
  stores.use(() => useLiveFeedStore()).giftCatalog = [{ id: '1', name: 'Rose', diamondCount: 1 }];
  const wrapper = mountWithPlugins(AutomationsPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await flushPromises();
  calls.length = 0;
  return { wrapper, calls };
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
    const { wrapper, calls } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    await flushPromises();
    expect(calls.find((call) => call.method === 'automation.disable')?.params).toEqual({ id: 'e2', kind: 'event' });
  });

  it('creates and saves a new event', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, 'New event').trigger('click');
    await wrapper.get('.event-editor input[type="text"]').setValue('Followers');
    await buttonByText(wrapper, 'Save').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'automation.update')?.params).toMatchObject({
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
    expect(calls.find((call) => call.method === 'automation.update')?.params).toMatchObject({
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
    expect(calls.find((call) => call.method === 'automation.delete')?.params).toEqual({ id: 'e2', kind: 'event' });
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
});

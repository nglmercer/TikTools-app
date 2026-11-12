import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import AutomationLibrary from './AutomationLibrary.vue';

enableAutoUnmount(afterEach);

const action: LiveAction = { schemaVersion: 2, id: 'action', name: 'Action', typeId: 'core.log', enabled: false, config: { message: 'hello' } };
const event: LiveEvent = { schemaVersion: 1, id: 'event', name: 'Event', enabled: false, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all', actionIds: ['action'] };

it.each(['events', 'actions'] as const)('a %s checkbox requests a toggle without selecting the row before its change event', async (tab) => {
  const wrapper = mountWithPlugins(AutomationLibrary, {
    attachTo: document.body,
    props: { tab, events: [event], actions: [action], actionTypes: [], availableTypes: new Set(['core.log']), eventTypes: [], runs: [] },
  });

  // A row selection opens the mobile sheet immediately, before pointerup/click.
  // Both the checkbox label hit area and its input must keep that from happening.
  for (const selector of ['.base-checkbox', 'input[type="checkbox"]']) {
    await wrapper.get(selector).trigger('pointerdown');
    expect(wrapper.emitted('update:selected')).toBeUndefined();
  }
  await wrapper.get('input[type="checkbox"]').setValue(true);
  expect(wrapper.emitted('setEnabled')).toEqual([[tab, tab === 'events' ? event.id : action.id, true]]);
  expect(wrapper.emitted('update:selected')).toBeUndefined();
  expect((wrapper.get('input[type="checkbox"]').element as HTMLInputElement).checked).toBe(false);

  await wrapper.get('.automation-library__name').trigger('pointerdown');
  expect(wrapper.emitted('update:selected')).toEqual([[tab === 'events' ? event.id : action.id]]);
});

it.each(['events', 'actions'] as const)('the %s table distinguishes an unmatched query from an empty library', async (tab) => {
  const wrapper = mountWithPlugins(AutomationLibrary, { props: {
    tab, events: [event], actions: [action], actionTypes: [], availableTypes: new Set(['core.log']),
    eventTypes: [], runs: [], eventQuery: 'missing', actionQuery: 'missing',
  } });
  expect(wrapper.findAll('.base-empty-state')).toHaveLength(1);
  expect(wrapper.get('.base-empty-state__title').text()).toBe(i18n.global.t('automations.library.noMatches', { query: 'missing' }));
  expect(wrapper.find('.base-empty-state__description').exists()).toBe(false);
  await wrapper.setProps({ events: [], actions: [], eventQuery: '', actionQuery: '' });
  expect(wrapper.findAll('.base-empty-state')).toHaveLength(1);
  const key = tab === 'events' ? 'emptyEvents' : 'emptyActions';
  expect(wrapper.get('.base-empty-state__title').text()).toBe(i18n.global.t(`automations.library.${key}`));
  expect(wrapper.find('.base-empty-state__description').exists()).toBe(true);
});

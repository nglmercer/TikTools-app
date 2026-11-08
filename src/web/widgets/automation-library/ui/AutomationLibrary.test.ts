import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
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

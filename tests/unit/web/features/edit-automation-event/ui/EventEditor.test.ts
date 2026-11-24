import { DOMWrapper, enableAutoUnmount, type VueWrapper } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { nextTick } from 'vue';
import type { EventFilter, LiveAction, LiveEvent } from '../../../../../../src/automation/behavior/types.ts';
import { cloneRecord, createEvent } from '@/entities/automation/lib/automation-records.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import EventEditor from '../../../../../../src/web/features/edit-automation-event/ui/EventEditor.vue';

enableAutoUnmount(afterEach);
const t = (key: string) => i18n.global.t(key);
function button(wrapper: VueWrapper, key: string) {
  return wrapper.findAll('button').find((item) => item.attributes('aria-label') === t(key))!;
}
function mountEditor() {
  const event: LiveEvent = { ...createEvent('Original'), trigger: 'tiktok.chat', actionIds: ['first', 'second'] };
  const actions: LiveAction[] = ['first', 'second'].map((id) => ({
    schemaVersion: 2, id, name: id, enabled: true, typeId: 'core.log', config: { message: id },
  }));
  const wrapper = mountWithPlugins(EventEditor, { attachTo: document.body, props: {
    event, baseline: event, actions, eventTypes: [], gifts: [], viewers: [], testRuns: [],
  } });
  return { event, wrapper };
}

it('combines action ordering, run mode and cooldown edits into one draft before explicit save', async () => {
  const { event, wrapper } = mountEditor();
  await wrapper.get('input[placeholder="' + t('automations.editor.name') + '"]').setValue('Edited');
  await button(wrapper, 'automations.event.reorder').trigger('keydown', { key: 'ArrowDown' });
  await button(wrapper, 'automations.event.random').trigger('click');
  const cooldown = wrapper.get('input[role="spinbutton"]');
  await cooldown.setValue('5');
  await cooldown.trigger('blur');
  await button(wrapper, 'automations.event.scopeGlobal').trigger('click');
  expect(wrapper.emitted('save')).toBeUndefined();
  await button(wrapper, 'automations.editor.save').trigger('click');
  const saved = wrapper.emitted('save')![0]![0] as LiveEvent;
  expect(saved).toMatchObject({ name: 'Edited', actionIds: ['second', 'first'], runMode: 'random', cooldownMs: 5000, cooldownScope: 'global' });
  expect(event).toMatchObject({ name: 'Original', actionIds: ['first', 'second'], runMode: 'all', cooldownMs: 0 });
  await wrapper.setProps({ event: cloneRecord(saved) });
  expect(button(wrapper, 'automations.editor.save').attributes('disabled')).toBeUndefined();
  await button(wrapper, 'automations.editor.discard').trigger('click');
  expect(button(wrapper, 'automations.editor.save').attributes('disabled')).toBeDefined();
  expect(wrapper.emitted('change')!.at(-1)![0]).toEqual(event);
});

it('reorders actions by dragging the grip with a pointer', async () => {
  const { wrapper } = mountEditor();
  const rows = wrapper.findAll('[data-action-id]');
  expect(rows).toHaveLength(2);
  rows.forEach((row, index) => {
    row.element.getBoundingClientRect = () => ({ top: index * 40, bottom: index * 40 + 40 }) as DOMRect;
  });
  const grip = button(wrapper, 'automations.event.reorder');
  await grip.trigger('pointerdown', { button: 0, pointerId: 1, clientY: 0 });
  await grip.trigger('pointermove', { clientY: 50 });
  await grip.trigger('pointerup', {});
  expect(wrapper.findAll('[data-action-id]').map((row) => row.attributes('data-action-id'))).toEqual(['second', 'first']);
  await button(wrapper, 'automations.editor.save').trigger('click');
  expect((wrapper.emitted('save')![0]![0] as LiveEvent).actionIds).toEqual(['second', 'first']);
});

it('duplicates a condition right below itself from the row action menu', async () => {
  const filters: EventFilter[] = [
    { path: 'event.data.text', operator: 'starts-with', value: '!play' },
    { path: 'event.data.text', operator: 'starts-with', value: '/play' },
  ];
  const event: LiveEvent = { ...createEvent('Original'), enabled: true, trigger: 'tiktok.chat', filters };
  const wrapper = mountWithPlugins(EventEditor, { attachTo: document.body, props: {
    event, baseline: event, actions: [], eventTypes: [], gifts: [], viewers: [], testRuns: [],
  } });
  await button(wrapper, 'automations.conditions.actions').trigger('click');
  await nextTick();
  const item = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
    .find((entry) => entry.textContent === t('automations.conditions.duplicate'))!;
  item.click();
  await nextTick();
  const changed = wrapper.emitted('change')!.at(-1)![0] as LiveEvent;
  expect(changed.filters).toEqual([filters[0], filters[0], filters[1]]);
  expect(changed.filters[0]).not.toBe(changed.filters[1]);
});

it('configures a condition template and discards it with the other unsaved edits', async () => {
  const { event, wrapper } = mountEditor();
  await wrapper.get('input[placeholder="' + t('automations.editor.name') + '"]').setValue(t('packages.categories.moderation'));
  await button(wrapper, 'automations.conditions.templates').trigger('click');
  const choice = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
    .find((item) => item.textContent?.trim() === t('automations.moderation.high'))!;
  choice.click();
  await nextTick();
  const dialog = new DOMWrapper(document.querySelector<HTMLDialogElement>('dialog[open]')!);
  await dialog.get('input').setValue('0.75');
  await dialog.findAll('button').find((item) => item.text() === t('automations.conditions.apply'))!.trigger('click');
  const changed = wrapper.emitted('change')!.at(-1)![0] as LiveEvent;
  expect(changed.name).toBe(t('packages.categories.moderation'));
  expect(changed.filters).toEqual([
    { path: 'event.intel.comment.composition.repetitionScore', operator: 'gte', value: '0.75' },
    { path: 'event.intel.comment.spam.detected', operator: 'is-true', value: '' },
  ]);
  expect(event.filters).toEqual([]);
  expect(wrapper.emitted('save')).toBeUndefined();
  await button(wrapper, 'automations.editor.discard').trigger('click');
  expect(wrapper.emitted('change')!.at(-1)![0]).toEqual(event);
  expect(wrapper.emitted('update:dirty')!.at(-1)).toEqual([false]);
});

it('does not mark a brand-new event as unsaved until it is edited', async () => {
  const event = createEvent('New event');
  const wrapper = mountWithPlugins(EventEditor, { attachTo: document.body, props: {
    event, baseline: event, isNew: true, actions: [], eventTypes: [], gifts: [], viewers: [], testRuns: [],
  } });
  await nextTick();
  expect(wrapper.text()).not.toContain(t('automations.editor.unsaved'));
  expect(button(wrapper, 'automations.editor.save').attributes('disabled')).toBeUndefined();
  await wrapper.get('input[placeholder="' + t('automations.editor.name') + '"]').setValue('Renamed');
  await nextTick();
  expect(wrapper.emitted('update:dirty')!.at(-1)).toEqual([true]);
  expect(wrapper.text()).toContain(t('automations.editor.unsaved'));
});

it('groups dry-run, real fire and custom data behind the flask menu', async () => {
  const { wrapper } = mountEditor();
  await button(wrapper, 'automations.runMenu.label').trigger('click');
  await nextTick();
  // MenuList teleports to body.
  const items = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];
  const labels = items.map((item) => item.textContent ?? '');
  expect(labels).toContain(t('automations.runMenu.test'));
  expect(labels).toContain(t('automations.runMenu.fire'));
  expect(labels).toContain(t('automations.runMenu.custom'));
  const dryRun = items.find((item) => item.textContent === t('automations.runMenu.test'))!;
  dryRun.click();
  await nextTick();
  expect(wrapper.emitted('test')).toHaveLength(1);
});

it('filters the action list without changing selection on save', async () => {
  const { wrapper } = mountEditor();
  await wrapper.get('input[placeholder="' + t('automations.editor.name') + '"]').setValue('Renamed');
  const search = wrapper.get<HTMLInputElement>('input[type="search"]');
  await search.setValue('second');
  expect(wrapper.findAll('[data-action-id]').map((row) => row.attributes('data-action-id'))).toEqual(['second']);
  await button(wrapper, 'automations.editor.save').trigger('click');
  expect((wrapper.emitted('save')![0]![0] as LiveEvent).actionIds).toEqual(['first', 'second']);
  await search.setValue('nothing matches');
  expect(wrapper.findAll('[data-action-id]')).toHaveLength(0);
  expect(wrapper.text()).toContain(t('automations.event.noMatchingActions'));
});

it('keeps the enable switch in the General section header and mutes an inactive editor', async () => {
  const event: LiveEvent = { ...createEvent('Original'), enabled: true, trigger: 'tiktok.chat' };
  const wrapper = mountWithPlugins(EventEditor, { attachTo: document.body, props: {
    event, baseline: event, actions: [], eventTypes: [], gifts: [], viewers: [], testRuns: [],
  } });
  const input = wrapper.get<HTMLInputElement>('input[role="switch"]');
  expect(input.element.closest('.base-section__actions')).not.toBeNull();
  expect(input.element.closest('.base-section__header')?.textContent).toContain(t('automations.event.general'));
  expect(wrapper.get('.event-editor__columns').classes()).not.toContain('event-editor__columns--inactive');
  await input.setValue(false);
  expect(wrapper.text()).toContain(t('automations.editor.disabled'));
  expect(wrapper.get('.event-editor__columns').classes()).toContain('event-editor__columns--inactive');
});

it('removes a condition through the row action menu', async () => {
  const filters: EventFilter[] = [
    { path: 'event.data.text', operator: 'starts-with', value: '!play' },
    { path: 'event.data.text', operator: 'starts-with', value: '/play' },
  ];
  const event: LiveEvent = { ...createEvent('Original'), enabled: true, trigger: 'tiktok.chat', filters };
  const wrapper = mountWithPlugins(EventEditor, { attachTo: document.body, props: {
    event, baseline: event, actions: [], eventTypes: [], gifts: [], viewers: [], testRuns: [],
  } });
  await button(wrapper, 'automations.conditions.actions').trigger('click');
  await nextTick();
  // MenuList teleports to body.
  const items = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
    .filter((item) => item.textContent === t('automations.conditions.remove'));
  expect(items).toHaveLength(1);
  items[0]!.click();
  await nextTick();
  const changed = wrapper.emitted('change')!.at(-1)![0] as LiveEvent;
  expect(changed.filters).toEqual([filters[1]]);
});

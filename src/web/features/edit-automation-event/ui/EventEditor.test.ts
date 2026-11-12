import { DOMWrapper, enableAutoUnmount, type VueWrapper } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { nextTick } from 'vue';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { cloneRecord, createEvent } from '@/entities/automation/lib/automation-records.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import EventEditor from './EventEditor.vue';

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
  await button(wrapper, 'automations.event.moveDown').trigger('click');
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

it('configures a condition template and discards it with the other unsaved edits', async () => {
  const { event, wrapper } = mountEditor();
  await wrapper.get('input[placeholder="' + t('automations.editor.name') + '"]').setValue('Moderation');
  await button(wrapper, 'automations.conditions.templates').trigger('click');
  const choice = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
    .find((item) => item.textContent?.trim() === t('automations.moderation.high'))!;
  choice.click();
  await nextTick();
  const dialog = new DOMWrapper(document.querySelector<HTMLDialogElement>('dialog[open]')!);
  await dialog.get('input').setValue('0.75');
  await dialog.findAll('button').find((item) => item.text() === t('automations.conditions.apply'))!.trigger('click');
  const changed = wrapper.emitted('change')!.at(-1)![0] as LiveEvent;
  expect(changed.name).toBe('Moderation');
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

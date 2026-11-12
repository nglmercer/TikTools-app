import { enableAutoUnmount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { afterEach, expect, it, vi } from 'vitest';
import type { MediaSelectionHandler, OpenMediaPicker } from '../../../../shared/messages.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { SchemaFieldDescriptor } from './lib/schema-fields.ts';
import SchemaFieldControl from './SchemaFieldControl.vue';

enableAutoUnmount(afterEach);
const field = (patch: Partial<SchemaFieldDescriptor> = {}): SchemaFieldDescriptor => ({
  key: 'value', kind: 'text', label: 'Value', hint: '', options: [], arrayJson: false,
  monospace: false, template: false, advanced: false, ...patch,
});

it('anchors variable suggestions to the control and inserts into the current text', async () => {
  const wrapper = mountWithPlugins(SchemaFieldControl, { attachTo: document.body, props: {
    field: field({ template: true }), value: 'Hello ', controlId: 'text-field',
    suggestions: [{ value: 'event.user.uniqueId', label: 'Viewer name' }],
  } });
  await wrapper.get('button').trigger('click');
  expect(wrapper.getComponent(BaseMenu).props('anchor')).toBeInstanceOf(HTMLElement);
  document.querySelector<HTMLElement>('[role="menuitem"]')!.click();
  await nextTick();
  expect(wrapper.emitted('update')!.at(-1)).toEqual(['Hello {{ event.user.uniqueId }}']);
  await wrapper.setProps({ field: field({ kind: 'json', arrayJson: true }), value: '' });
  await wrapper.get('textarea').setValue('["one","two"]');
  expect(wrapper.emitted('update')!.at(-1)).toEqual([['one', 'two']]);
});

it('retains media errors across control-kind changes, clears them on typing and preserves selected file metadata', async () => {
  let selected: MediaSelectionHandler | undefined;
  const picker = vi.fn<OpenMediaPicker>((_options, callback) => { selected = callback; });
  const media = field({ kind: 'media', media: { mode: 'file', kind: 'audio' } });
  const wrapper = mountWithPlugins(SchemaFieldControl, { props: {
    field: media, value: '', controlId: 'media-field', openMediaPicker: picker,
  } });
  await wrapper.get('button').trigger('click');
  expect(picker.mock.calls[0]![0]).toMatchObject({ mode: 'file', kind: 'audio', title: 'Value', extensions: expect.arrayContaining(['wav', 'mp3']) });
  selected?.(null, 'Unreadable file');
  await nextTick();
  expect(wrapper.get('[role="alert"]').text()).toBe('Unreadable file');
  await wrapper.setProps({ field: field() });
  await wrapper.setProps({ field: media });
  expect(wrapper.get('[role="alert"]').text()).toBe('Unreadable file');
  await wrapper.get('input').setValue('/manual.wav');
  expect(wrapper.find('[role="alert"]').exists()).toBe(false);
  expect(wrapper.emitted('update')!.at(-1)).toEqual(['/manual.wav']);
  await wrapper.get('button').trigger('click');
  const file = { referenceVersion: 1, path: '/sound.wav', directory: '/', name: 'sound.wav', extension: 'wav', sizeBytes: 42 };
  selected?.({ type: 'file', file });
  await nextTick();
  expect(wrapper.emitted('update')!.at(-1)).toEqual([file]);
});

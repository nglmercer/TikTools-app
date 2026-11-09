import { defineComponent, ref } from 'vue';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import MasterDetailLayout from './MasterDetailLayout.vue';
enableAutoUnmount(afterEach);
const Draft = defineComponent({ setup: () => ({ value: ref('original') }), template: '<input v-model="value" aria-label="Draft">' });
test('keeps collection and draft instances when resizing or returning to the list', async () => {
  const wrapper = mountWithPlugins(MasterDetailLayout, {
    props: { narrow:false, detailOpen:true, backLabel:'Back' },
    slots: { master:'<input value="filter" aria-label="Search">', detail:Draft },
  });
  const editor = wrapper.get('input[aria-label="Draft"]');
  await editor.setValue('unsaved draft');
  await wrapper.setProps({ narrow:true });
  expect(wrapper.get('input[aria-label="Draft"]').element).toBe(editor.element);
  expect((editor.element as HTMLInputElement).value).toBe('unsaved draft');
  await wrapper.get('.master-detail__back button').trigger('click');
  expect(wrapper.emitted('back')).toHaveLength(1);
  // Parent owns closing, so a pending discard review keeps the editor open.
  expect(wrapper.get('.master-detail__detail').attributes('hidden')).toBeUndefined();
  await wrapper.setProps({ detailOpen:false });
  expect(wrapper.get('.master-detail__detail').attributes('hidden')).toBeDefined();
  expect(wrapper.get('.master-detail__master').attributes('hidden')).toBeUndefined();
  await wrapper.setProps({ narrow:false });
  expect(wrapper.get('input[aria-label="Draft"]').element).toBe(editor.element);
  expect((editor.element as HTMLInputElement).value).toBe('unsaved draft');
});

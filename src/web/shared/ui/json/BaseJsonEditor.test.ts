import { testMessage } from '@/shared/i18n/test-message.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import BaseJsonEditor from './BaseJsonEditor.vue';

enableAutoUnmount(afterEach);

it('validates the object and emits the parsed value', async () => {
  const wrapper = mountWithPlugins(BaseJsonEditor, { props: { modelValue: '' } });
  expect(wrapper.emitted('validation')?.[0]?.[0]).toMatchObject({
    error: testMessage('en', 'jsonView.editor.empty'),
  });
  await wrapper.get('textarea').setValue('{"comment":"!play vaundy"}');
  expect(wrapper.emitted('validation')?.at(-1)?.[0]).toMatchObject({
    value: { comment: '!play vaundy' },
    error: '',
  });
});

it('rejects arrays and renders the valid document in the preview', async () => {
  const wrapper = mountWithPlugins(BaseJsonEditor, { props: { modelValue: '[1, 2]' } });
  expect(wrapper.text()).toContain(testMessage('en', 'jsonView.editor.objectRequired'));
  await wrapper.get('textarea').setValue('{"a":1}');
  const preview = wrapper.findAll('button').find((button) => button.text() === testMessage('en', 'jsonView.editor.preview'))!;
  await preview.trigger('click');
  expect(wrapper.get('pre').text()).toContain('"a": 1');
});

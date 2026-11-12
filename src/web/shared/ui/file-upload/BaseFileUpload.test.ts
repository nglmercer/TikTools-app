import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseFileUpload from './BaseFileUpload.vue';

function mount(disabled = false) {
  return mountWithPlugins(BaseFileUpload, { props: { label: 'Drop image', hint: 'PNG only', accept: 'image/png,.png', maxBytes: 16, disabled } });
}
const image = () => new File(['image'], 'sample.png', { type: 'image/png' });

describe('BaseFileUpload', () => {
  it('accepts dropped files and does not upload automatically', async () => {
    const wrapper = mount();
    const file = image();
    await wrapper.get('[role="group"]').trigger('drop', { dataTransfer: { files: [file] } });
    expect(wrapper.emitted('update:modelValue')).toEqual([[file]]);
    expect(wrapper.emitted('error')).toBeUndefined();
  });

  it('validates file count, format and size for dropped files', async () => {
    for (const files of [[image(), image()], [new File(['x'], 'script.exe')], [new File(['x'.repeat(17)], 'big.png')], [new File([], 'empty.png')]]) {
      const wrapper = mount();
      await wrapper.setProps({ modelValue: image() });
      await wrapper.get('[role="group"]').trigger('drop', { dataTransfer: { files } });
      expect(wrapper.emitted('error')).toHaveLength(1);
      expect(wrapper.emitted('update:modelValue')).toEqual([[null]]);
      expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    }
  });

  it('uses the same validation for the file picker and permits choosing the same file again', async () => {
    const wrapper = mount();
    const input = wrapper.get('input');
    const file = image();
    Object.defineProperty(input.element, 'files', { value: [file], configurable: true });
    await input.trigger('change');
    expect(wrapper.emitted('update:modelValue')).toEqual([[file]]);
    expect((input.element as HTMLInputElement).value).toBe('');
    Object.defineProperty(input.element, 'files', { value: [new File(['x'], 'wrong.txt')], configurable: true });
    await input.trigger('change');
    expect(wrapper.emitted('error')).toHaveLength(1);
  });

  it('blocks both drop and picker while disabled', async () => {
    const wrapper = mount(true);
    await wrapper.get('[role="group"]').trigger('drop', { dataTransfer: { files: [image()] } });
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
    expect(wrapper.get('input').attributes('disabled')).toBeDefined();
  });

  it('shows the selected filename and allows clearing it', async () => {
    const wrapper = mount();
    await wrapper.setProps({ modelValue: image() });
    expect(wrapper.get('[role="status"]').text()).toContain('sample.png');
    await wrapper.get('[role="status"] button').trigger('click');
    expect(wrapper.emitted('update:modelValue')).toEqual([[null]]);
  });
});

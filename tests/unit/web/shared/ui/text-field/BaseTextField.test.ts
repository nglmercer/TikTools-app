import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { describe, expect, it } from 'vitest';
import BaseTextField from '../../../../../../src/web/shared/ui/text-field/BaseTextField.vue';

describe('BaseTextField', () => {
  it('binds v-model to the input', async () => {
    const wrapper = mountWithPlugins(BaseTextField, {
      props: { modelValue: 'alpha', 'onUpdate:modelValue': (value: string) => wrapper.setProps({ modelValue: value }) },
    });
    const input = wrapper.get('input');
    expect(input.element.value).toBe('alpha');
    await input.setValue('beta');
    expect(wrapper.props('modelValue')).toBe('beta');
  });

  it('forwards native attributes to the input and layout attributes to the root', () => {
    const wrapper = mountWithPlugins(BaseTextField, { attrs: { id: 'name', class: 'wide', 'aria-label': 'Name' } });
    expect(wrapper.classes()).toContain('wide');
    expect(wrapper.get('input').attributes()).toMatchObject({ id: 'name', 'aria-label': 'Name' });
    expect(wrapper.get('input').classes()).not.toContain('wide');
  });

  it('marks invalid state for assistive technology', () => {
    const wrapper = mountWithPlugins(BaseTextField, { props: { invalid: true } });
    expect(wrapper.get('input').attributes('aria-invalid')).toBe('true');
    expect(wrapper.classes()).toContain('base-text-field--invalid');
  });

  it('toggles password visibility when revealable', async () => {
    const wrapper = mountWithPlugins(BaseTextField, { props: { type: 'password', revealable: true } });
    expect(wrapper.get('input').attributes('type')).toBe('password');
    await wrapper.get('button[aria-label="Show value"]').trigger('click');
    expect(wrapper.get('input').attributes('type')).toBe('text');
    expect(wrapper.find('button[aria-label="Hide value"]').exists()).toBe(true);
  });
});

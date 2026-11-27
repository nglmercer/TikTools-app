import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import BaseRadio from '../../../../../../src/web/shared/ui/radio/BaseRadio.vue';
import BaseCheckbox from '../../../../../../src/web/shared/ui/checkbox/BaseCheckbox.vue';

describe('BaseCheckbox', () => {
  it('toggles v-model and shows the check mark', async () => {
    const wrapper = mount(BaseCheckbox, {
      props: { modelValue: false, 'onUpdate:modelValue': (value: boolean) => wrapper.setProps({ modelValue: value }) },
      slots: { default: 'Capture cursor' },
    });
    expect(wrapper.find('.base-checkbox__mark').exists()).toBe(false);
    await wrapper.get('input').setValue(true);
    expect(wrapper.props('modelValue')).toBe(true);
    expect(wrapper.find('.base-checkbox__mark').exists()).toBe(true);
    expect(wrapper.text()).toBe('Capture cursor');
  });

  it('reflects the indeterminate state on the native input', () => {
    const wrapper = mount(BaseCheckbox, { props: { indeterminate: true } });
    expect(wrapper.get<HTMLInputElement>('input').element.indeterminate).toBe(true);
  });
});

describe('BaseRadio', () => {
  it('selects its value through v-model', async () => {
    const wrapper = mount(BaseRadio<string>, {
      props: {
        value: 'auto',
        modelValue: 'manual',
        'onUpdate:modelValue': (value: string | undefined) => wrapper.setProps({ modelValue: value }),
      },
    });
    const input = wrapper.get<HTMLInputElement>('input');
    expect(input.element.checked).toBe(false);
    await input.setValue(true);
    expect(wrapper.props('modelValue')).toBe('auto');
  });
});

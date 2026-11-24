import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import BaseSlider from '../../../../../../src/web/shared/ui/slider/BaseSlider.vue';

describe('BaseSlider', () => {
  it('emits numeric values and tracks the filled portion', async () => {
    const wrapper = mount(BaseSlider, {
      props: { modelValue: 25, 'onUpdate:modelValue': (value: number) => wrapper.setProps({ modelValue: value }) },
    });
    expect(wrapper.attributes('style')).toContain('--slider-fill: 25%');
    await wrapper.find('input').setValue('60');
    expect(wrapper.props('modelValue')).toBe(60);
    expect(wrapper.attributes('style')).toContain('--slider-fill: 60%');
  });

  it('maps the fill to a custom range', () => {
    const wrapper = mount(BaseSlider, { props: { modelValue: -30, min: -60, max: 0 } });
    expect(wrapper.attributes('style')).toContain('--slider-fill: 50%');
  });
});

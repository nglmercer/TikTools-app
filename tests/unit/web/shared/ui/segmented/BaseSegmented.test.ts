import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import BaseDateField from '../../../../../../src/web/shared/ui/date-field/BaseDateField.vue';
import BaseSegmented from '../../../../../../src/web/shared/ui/segmented/BaseSegmented.vue';
import { defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);

const options = [
  { value: 'today', label: 'Today' },
  { value: '7d', label: '7 days' },
  { value: '30d', label: '30 days', disabled: true },
  { value: '90d', label: '90 days' },
];

function mountSegmented() {
  const wrapper = mount(BaseSegmented<string>, {
    attachTo: document.body,
    props: {
      options,
      modelValue: '7d',
      'onUpdate:modelValue': (value: string) => wrapper.setProps({ modelValue: value }),
    },
  });
  return wrapper;
}

describe('BaseSegmented', () => {
  it('exposes radio semantics with one tab stop on the selection', () => {
    const wrapper = mountSegmented();
    const radios = wrapper.findAll('[role="radio"]');
    expect(wrapper.attributes('role')).toBe('radiogroup');
    expect(radios.map((radio) => radio.attributes('aria-checked'))).toEqual(['false', 'true', 'false', 'false']);
    expect(radios.map((radio) => radio.attributes('tabindex'))).toEqual(['-1', '0', '-1', '-1']);
  });

  it('selects by click and arrows past disabled options', async () => {
    const wrapper = mountSegmented();
    await defined(wrapper.findAll('[role="radio"]')[0]).trigger('click');
    expect(wrapper.props('modelValue')).toBe('today');
    await wrapper.trigger('keydown', { key: 'End' });
    expect(wrapper.props('modelValue')).toBe('90d');
    await wrapper.trigger('keydown', { key: 'ArrowLeft' });
    expect(wrapper.props('modelValue')).toBe('7d');
  });
});

describe('BaseDateField', () => {
  it('binds an ISO date and its bounds', async () => {
    const wrapper = mount(BaseDateField, {
      props: {
        modelValue: '2026-10-19',
        max: '2026-10-31',
        'onUpdate:modelValue': (value: string) => wrapper.setProps({ modelValue: value }),
      },
    });
    expect(wrapper.attributes()).toMatchObject({ type: 'date', max: '2026-10-31' });
    await wrapper.setValue('2026-10-01');
    expect(wrapper.props('modelValue')).toBe('2026-10-01');
  });
});

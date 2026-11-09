import { mount } from '@vue/test-utils';
import { expect, test } from 'vitest';
import SettingsRow from './SettingsRow.vue';
test('labels the input and gives its group a stable description without hiding details', () => {
  const wrapper = mount(SettingsRow, {
    props: { label:'Background updates', for:'updates', description:'Check automatically' },
    slots: { default:'<input id="updates" type="checkbox">', status:'<span>Available</span>', details:'<p>Advanced options</p>' },
  });
  const label = wrapper.get('label');
  expect(label.attributes('for')).toBe(wrapper.get('input').attributes('id'));
  expect(wrapper.get('[role="group"]').attributes('aria-labelledby')).toBe(label.attributes('id'));
  expect(wrapper.get('[role="group"]').attributes('aria-describedby')).toBe(wrapper.get('.settings-row__description').attributes('id'));
  expect(wrapper.text()).toContain('Advanced options');
  expect(wrapper.text()).toContain('Available');
});

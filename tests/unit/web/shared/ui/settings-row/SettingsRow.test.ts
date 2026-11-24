const t = (key: string): string => testMessage('en', key);
import { testMessage } from '@/shared/i18n/test-message.ts';
import { mount } from '@vue/test-utils';
import { expect, test } from 'vitest';
import SettingsRow from '../../../../../../src/web/shared/ui/settings-row/SettingsRow.vue';
test('labels the input and gives its group a stable description without hiding details', () => {
  const wrapper = mount(SettingsRow, {
    props: { label:'Background updates', for:'updates', description:t('appUpdater.automaticChecks') },
    slots: { default:'<input id="updates" type="checkbox">', status:'<span>Available</span>', details:'<p>Advanced options</p>' },
  });
  const label = wrapper.get('label');
  expect(label.attributes('for')).toBe(wrapper.get('input').attributes('id'));
  expect(wrapper.get('[role="group"]').attributes('aria-labelledby')).toBe(label.attributes('id'));
  expect(wrapper.get('[role="group"]').attributes('aria-describedby')).toBe(wrapper.get('.settings-row__description').attributes('id'));
  expect(wrapper.text()).toContain(t('schemaForm.advanced'));
  expect(wrapper.text()).toContain('Available');
});

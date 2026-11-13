import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import BaseComboBox from './BaseComboBox.vue';

const options = [
  { value: 'fade', label: 'Fade' },
  { value: 'cut', label: 'Cut' },
  { value: 'slide', label: 'Slide', disabled: true },
  { value: 'stinger', label: 'Stinger' },
];

function mountComboBox(modelValue?: string) {
  const wrapper = mount(BaseComboBox<string>, {
    attachTo: document.body,
    props: {
      options,
      modelValue,
      placeholder: 'Choose',
      'onUpdate:modelValue': (value: string | undefined) => wrapper.setProps({ modelValue: value }),
    },
  });
  return wrapper;
}

function listbox(): HTMLElement | null {
  return document.body.querySelector('[role="listbox"]');
}

enableAutoUnmount(afterEach);

describe('BaseComboBox', () => {
  it('shows the selected label or the placeholder', async () => {
    const wrapper = mountComboBox();
    expect(wrapper.text()).toContain('Choose');
    await wrapper.setProps({ modelValue: 'cut' });
    expect(wrapper.text()).toContain('Cut');
  });

  it('opens a listbox with the selected option active', async () => {
    const wrapper = mountComboBox('cut');
    const trigger = wrapper.get('[role="combobox"]');
    await trigger.trigger('click');
    expect(trigger.attributes('aria-expanded')).toBe('true');
    const active = document.getElementById(trigger.attributes('aria-activedescendant')!);
    expect(active?.textContent?.trim()).toBe('Cut');
    expect(active?.getAttribute('aria-selected')).toBe('true');
  });

  it('navigates past disabled options and selects with Enter', async () => {
    const wrapper = mountComboBox('cut');
    const trigger = wrapper.get('[role="combobox"]');
    await trigger.trigger('keydown', { key: 'ArrowDown' });
    await trigger.trigger('keydown', { key: 'ArrowDown' });
    await trigger.trigger('keydown', { key: 'Enter' });
    expect(wrapper.props('modelValue')).toBe('stinger');
    expect(listbox()).toBeNull();
  });

  it('selects an option by click and ignores disabled ones', async () => {
    const wrapper = mountComboBox('fade');
    await wrapper.get('[role="combobox"]').trigger('click');
    const items = listbox()!.querySelectorAll<HTMLElement>('[role="option"]');
    items[2]!.click();
    expect(wrapper.props('modelValue')).toBe('fade');
    items[1]!.click();
    await wrapper.vm.$nextTick();
    expect(wrapper.props('modelValue')).toBe('cut');
  });

  it('closes on Escape and on outside pointer down', async () => {
    const wrapper = mountComboBox();
    const trigger = wrapper.get('[role="combobox"]');
    await trigger.trigger('click');
    await trigger.trigger('keydown', { key: 'Escape' });
    expect(listbox()).toBeNull();
    await trigger.trigger('click');
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(listbox()).toBeNull();
  });

  it('clears the typeahead timeout when a selector is unmounted', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    try {
      const baseline = vi.getTimerCount();
      const wrapper = mountComboBox('fade');
      await wrapper.get('[role="combobox"]').trigger('keydown', { key: 's' });
      expect(vi.getTimerCount()).toBeGreaterThan(baseline);
      wrapper.unmount();
      expect(vi.getTimerCount()).toBe(baseline);
    } finally { vi.useRealTimers(); }
  });

  it('changes the value by typeahead while closed', async () => {
    const wrapper = mountComboBox('fade');
    await wrapper.get('[role="combobox"]').trigger('keydown', { key: 's' });
    expect(wrapper.props('modelValue')).toBe('stinger');
  });
});

 it('keeps its popup inside a modal dialog so it stays interactive', async () => {
  const dialog = document.createElement('dialog');
  dialog.setAttribute('open', '');
  document.body.append(dialog);
  const wrapper = mount(BaseComboBox, { props: { options, modelValue: 'fade' }, attachTo: dialog });
  await wrapper.get('[role="combobox"]').trigger('click');
  expect(dialog.querySelector('[role="listbox"]')).not.toBeNull();
  (dialog.querySelectorAll('[role="option"]')[1] as HTMLElement).click();
  expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['cut']);
  wrapper.unmount();
  dialog.remove();
 });

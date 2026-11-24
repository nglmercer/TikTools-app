import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import BaseButton from '../../../../../../src/web/shared/ui/button/BaseButton.vue';
import BaseIconButton from '../../../../../../src/web/shared/ui/button/BaseIconButton.vue';

describe('BaseButton', () => {
  it('renders a non-submitting button with its label', () => {
    const wrapper = mount(BaseButton, { slots: { default: 'Start' } });
    expect(wrapper.attributes('type')).toBe('button');
    expect(wrapper.text()).toBe('Start');
    expect(wrapper.attributes('aria-pressed')).toBeUndefined();
  });

  it('applies variant, size and block modifiers', () => {
    const wrapper = mount(BaseButton, { props: { variant: 'primary', size: 'sm', block: true } });
    expect(wrapper.classes()).toEqual(
      expect.arrayContaining(['base-button--primary', 'base-button--sm', 'base-button--block']),
    );
  });

  it('exposes toggle state and blocks clicks when disabled', async () => {
    const wrapper = mount(BaseButton, { props: { pressed: true, disabled: true } });
    expect(wrapper.attributes('aria-pressed')).toBe('true');
    await wrapper.trigger('click');
    expect(wrapper.emitted('click')).toBeUndefined();
  });
});

describe('BaseIconButton', () => {
  it('uses the label as accessible name', () => {
    const wrapper = mount(BaseIconButton, { props: { label: 'Add source' }, slots: { default: '<svg />' } });
    expect(wrapper.attributes('aria-label')).toBe('Add source');
    expect(wrapper.classes()).toContain('base-icon-button--ghost');
  });
});

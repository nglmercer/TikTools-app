import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseSwitch from '../../../../../../src/web/shared/ui/switch/BaseSwitch.vue';

describe('BaseSwitch', () => {
  it('exposes an accessible switch and emits changes', async () => {
    const wrapper = mountWithPlugins(BaseSwitch, { props: { modelValue: false }, slots: { default: 'Automatic connection' } });
    const input = wrapper.get('input[role="switch"]');
    expect(wrapper.text()).toContain('Automatic connection');
    await input.setValue(true);
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([true]);
  });
  it('disables its input when requested', () => {
    const wrapper = mountWithPlugins(BaseSwitch, { props: { disabled: true } });
    expect(wrapper.get('input').attributes('disabled')).toBeDefined();
  });
});

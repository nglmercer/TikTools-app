import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseSpinBox from './BaseSpinBox.vue';

function mountSpinBox(props: Record<string, unknown> = {}) {
  const wrapper = mountWithPlugins(BaseSpinBox, {
    props: {
      modelValue: 300,
      'onUpdate:modelValue': (value: number) => wrapper.setProps({ modelValue: value }),
      ...props,
    },
  });
  return wrapper;
}

describe('BaseSpinBox', () => {
  it('renders the value with its suffix and spinbutton semantics', () => {
    const wrapper = mountSpinBox({ suffix: 'ms', min: 50, max: 20000 });
    const input = wrapper.get('input');
    expect(input.element.value).toBe('300');
    expect(input.attributes()).toMatchObject({ role: 'spinbutton', 'aria-valuenow': '300', 'aria-valuemin': '50' });
    expect(wrapper.text()).toContain('ms');
  });

  it('steps with arrow keys and jumps with page keys', async () => {
    const wrapper = mountSpinBox({ step: 50 });
    const input = wrapper.get('input');
    await input.trigger('keydown', { key: 'ArrowUp' });
    expect(wrapper.props('modelValue')).toBe(350);
    await input.trigger('keydown', { key: 'PageDown' });
    expect(wrapper.props('modelValue')).toBe(-150);
  });

  it('commits typed text on blur and clamps to the range', async () => {
    const wrapper = mountSpinBox({ min: 0, max: 1000 });
    const input = wrapper.get('input');
    await input.setValue('5000');
    await input.trigger('blur');
    expect(wrapper.props('modelValue')).toBe(1000);
    expect(input.element.value).toBe('1000');
  });

  it('reverts invalid text and escape to the current value', async () => {
    const wrapper = mountSpinBox();
    const input = wrapper.get('input');
    await input.setValue('abc');
    await input.trigger('blur');
    expect(wrapper.props('modelValue')).toBe(300);
    expect(input.element.value).toBe('300');
    await input.setValue('42');
    await input.trigger('keydown', { key: 'Escape' });
    expect(input.element.value).toBe('300');
  });

  it('disables the arrow at the range limit', () => {
    const wrapper = mountSpinBox({ max: 300 });
    const [increase, decrease] = wrapper.findAll('button');
    expect(increase!.attributes('disabled')).toBeDefined();
    expect(decrease!.attributes('disabled')).toBeUndefined();
    expect(increase!.attributes('aria-label')).toBe('Increase');
  });

  it('steps once per arrow press', async () => {
    const wrapper = mountSpinBox();
    await wrapper.findAll('button')[1]!.trigger('pointerdown');
    await wrapper.findAll('button')[1]!.trigger('pointerup');
    expect(wrapper.props('modelValue')).toBe(299);
  });

  it('renders flat −/+ steppers when horizontal', async () => {
    const wrapper = mountSpinBox({ orientation: 'horizontal', modelValue: 5, min: 1, max: 10 });
    expect(wrapper.classes()).toContain('base-spin-box--horizontal');
    const buttons = wrapper.findAll('button');
    expect(buttons.map((button) => button.attributes('aria-label'))).toEqual(['Decrease', 'Increase']);
    await buttons[1]!.trigger('pointerdown');
    await buttons[1]!.trigger('pointerup');
    expect(wrapper.props('modelValue')).toBe(6);
  });
});

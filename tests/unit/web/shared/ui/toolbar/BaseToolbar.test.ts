import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import BaseSplitter from '../../../../../../src/web/shared/ui/splitter/BaseSplitter.vue';
import BaseToolbar from '../../../../../../src/web/shared/ui/toolbar/BaseToolbar.vue';

enableAutoUnmount(afterEach);

describe('BaseToolbar', () => {
  it('keeps a single tab stop and moves focus with arrow keys', async () => {
    const wrapper = mount(BaseToolbar, {
      attachTo: document.body,
      attrs: { 'aria-label': 'Sources' },
      slots: { default: '<button>Add</button><button disabled>Paste</button><button>Remove</button>' },
    });
    const [add, , remove] = wrapper.findAll('button').map((button) => button.element);
    expect(add!.tabIndex).toBe(0);
    expect(remove!.tabIndex).toBe(-1);
    add!.focus();
    await wrapper.trigger('keydown', { key: 'ArrowRight' });
    expect(document.activeElement).toBe(remove);
    expect(remove!.tabIndex).toBe(0);
    expect(add!.tabIndex).toBe(-1);
    await wrapper.trigger('keydown', { key: 'ArrowRight' });
    expect(document.activeElement).toBe(add);
  });
});

describe('BaseSplitter', () => {
  it('resizes with the keyboard within its bounds', async () => {
    const wrapper = mount(BaseSplitter, {
      props: {
        size: 200,
        min: 120,
        max: 300,
        step: 50,
        'onUpdate:size': (value: number) => wrapper.setProps({ size: value }),
      },
    });
    const handle = wrapper.get('[role="separator"]');
    expect(handle.attributes()).toMatchObject({ 'aria-valuenow': '200', 'aria-valuemin': '120', 'aria-valuemax': '300' });
    await handle.trigger('keydown', { key: 'ArrowLeft' });
    expect(wrapper.props('size')).toBe(150);
    await handle.trigger('keydown', { key: 'ArrowLeft' });
    expect(wrapper.props('size')).toBe(120);
    await handle.trigger('keydown', { key: 'Home' });
    expect(wrapper.props('size')).toBe(120);
  });
});

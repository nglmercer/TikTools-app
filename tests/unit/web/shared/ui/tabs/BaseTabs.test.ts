import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import BaseTabs from '../../../../../../src/web/shared/ui/tabs/BaseTabs.vue';
import { defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);

const tabs = [
  { value: 'general', label: 'General' },
  { value: 'stream', label: 'Stream' },
  { value: 'output', label: 'Output', disabled: true },
  { value: 'audio', label: 'Audio' },
];

function mountTabs() {
  const wrapper = mount(BaseTabs<string>, {
    attachTo: document.body,
    props: {
      tabs,
      modelValue: 'general',
      'onUpdate:modelValue': (value: string) => wrapper.setProps({ modelValue: value }),
    },
    slots: { default: '<template #default="{ value }"><p class="panel">{{ value }}</p></template>' },
  });
  return wrapper;
}

describe('BaseTabs', () => {
  it('links the selected tab to its panel with a single tab stop', () => {
    const wrapper = mountTabs();
    const [general, stream] = wrapper.findAll('[role="tab"]');
    expect(defined(general).attributes()).toMatchObject({ 'aria-selected': 'true', tabindex: '0' });
    expect(defined(stream).attributes('tabindex')).toBe('-1');
    expect(wrapper.get('[role="tabpanel"]').attributes('aria-labelledby')).toBe(defined(general).attributes('id'));
    expect(wrapper.get('.panel').text()).toBe('general');
  });

  it('moves selection with arrow keys skipping disabled tabs', async () => {
    const wrapper = mountTabs();
    const list = wrapper.get('[role="tablist"]');
    await list.trigger('keydown', { key: 'ArrowRight' });
    expect(wrapper.props('modelValue')).toBe('stream');
    await list.trigger('keydown', { key: 'ArrowRight' });
    expect(wrapper.props('modelValue')).toBe('audio');
    await list.trigger('keydown', { key: 'Home' });
    expect(wrapper.props('modelValue')).toBe('general');
  });

  it('shows an icon and a badge on a tab and a bar slot beside the tab list', () => {
    const wrapper = mount(BaseTabs<string>, {
      props: {
        tabs: [{ value: 'a', label: 'Alpha', icon: IconPuzzle, badge: 3 }, { value: 'b', label: 'Beta' }],
        modelValue: 'a',
      },
      slots: { bar: '<input aria-label="Search">' },
    });
    const [alpha, beta] = wrapper.findAll('[role="tab"]');
    expect(defined(alpha).find('svg').exists()).toBe(true);
    expect(defined(alpha).text()).toBe('Alpha 3');
    expect(defined(beta).find('svg').exists()).toBe(false);
    expect(defined(beta).text()).toBe('Beta');
    // The slot sits beside the tab list, never inside it.
    expect(wrapper.get('[role="tablist"]').find('input').exists()).toBe(false);
    expect(wrapper.find('.base-tabs__bar input').exists()).toBe(true);
  });
});

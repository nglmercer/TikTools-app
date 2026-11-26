import { describe, expect, it } from 'vitest';
import { ref } from 'vue';
import { flushPromises } from '@vue/test-utils';
import AnalyticsRangeBar from '../../../../../src/web/features/select-analytics-range/ui/AnalyticsRangeBar.vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';

function mountBar(range: 'today' | '7d' | '30d' | '90d' | 'custom') {
  const state = { start: ref('2026-11-20'), end: ref('2026-11-26') };
  const wrapper = mountWithPlugins(AnalyticsRangeBar, {
    attachTo: document.body,
    props: {
      range,
      start: state.start.value,
      end: state.end.value,
      'onUpdate:start': (value: string) => (state.start.value = value),
      'onUpdate:end': (value: string) => (state.end.value = value),
    },
  });
  return { wrapper, state };
}

describe('AnalyticsRangeBar', () => {
  it('hides the custom trigger behind preset ranges', () => {
    const { wrapper } = mountBar('7d');
    expect(wrapper.find('.range-bar__trigger').exists()).toBe(false);
  });

  it('opens the popover when the custom range activates', async () => {
    const { wrapper } = mountBar('7d');
    await wrapper.setProps({ range: 'custom' });
    await flushPromises();
    expect(wrapper.find('.range-bar__popover').exists()).toBe(true);
    expect(wrapper.findAll('input[type="date"]')).toHaveLength(2);
  });

  it('stays closed when mounted straight into a custom range', () => {
    const { wrapper } = mountBar('custom');
    expect(wrapper.find('.range-bar__popover').exists()).toBe(false);
  });

  it('shows the picked span compactly on the trigger', () => {
    const { wrapper } = mountBar('custom');
    const trigger = wrapper.get('.range-bar__trigger');
    expect(trigger.text()).toContain('20');
    expect(trigger.text()).toContain('26');
  });

  it('closes the popover on Escape and outside clicks', async () => {
    const { wrapper } = mountBar('custom');
    await wrapper.get('.range-bar__trigger').trigger('click');
    expect(wrapper.find('.range-bar__popover').exists()).toBe(true);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await flushPromises();
    expect(wrapper.find('.range-bar__popover').exists()).toBe(false);
    // Reopen, then click outside the bar.
    await wrapper.get('.range-bar__trigger').trigger('click');
    expect(wrapper.find('.range-bar__popover').exists()).toBe(true);
    document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    await flushPromises();
    expect(wrapper.find('.range-bar__popover').exists()).toBe(false);
  });

  it('keeps edits flowing through the models', async () => {
    const { wrapper, state } = mountBar('custom');
    await wrapper.get('.range-bar__trigger').trigger('click');
    const [from, to] = wrapper.findAll('input[type="date"]');
    await from!.setValue('2026-11-01');
    await to!.setValue('2026-11-30');
    expect(state.start.value).toBe('2026-11-01');
    expect(state.end.value).toBe('2026-11-30');
  });
});

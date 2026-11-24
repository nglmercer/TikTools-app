import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, h, withDirectives } from 'vue';
import { vTooltip } from '../../../../../../src/web/shared/ui/tooltip/v-tooltip.ts';

enableAutoUnmount(afterEach);

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

function mountTarget(text: string, label?: string) {
  return mount(
    defineComponent(() => () => withDirectives(h('button', { 'aria-label': label }, 'x'), [[vTooltip, text]])),
    { attachTo: document.body },
  );
}

function tooltip(): HTMLElement | null {
  return document.getElementById('app-tooltip');
}

describe('vTooltip', () => {
  it('shows after the hover delay and hides on leave', async () => {
    const wrapper = mountTarget('Scene filters');
    const button = wrapper.get('button');
    await button.trigger('pointerenter');
    expect(tooltip()?.hidden ?? true).toBe(true);
    vi.advanceTimersByTime(500);
    expect(tooltip()?.hidden).toBe(false);
    expect(tooltip()?.textContent).toBe('Scene filters');
    expect(button.attributes('aria-describedby')).toBe('app-tooltip');
    await button.trigger('pointerleave');
    expect(tooltip()?.hidden).toBe(true);
    expect(button.attributes('aria-describedby')).toBeUndefined();
  });

  it('does not duplicate an identical accessible name', async () => {
    const wrapper = mountTarget('Add', 'Add');
    const button = wrapper.get('button');
    await button.trigger('focusin');
    vi.advanceTimersByTime(500);
    expect(tooltip()?.hidden).toBe(false);
    expect(button.attributes('aria-describedby')).toBeUndefined();
  });

  it('hides when the target unmounts', async () => {
    const wrapper = mountTarget('Remove');
    await wrapper.get('button').trigger('pointerenter');
    vi.advanceTimersByTime(500);
    wrapper.unmount();
    expect(tooltip()?.hidden).toBe(true);
  });
});

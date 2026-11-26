import { afterEach, describe, expect, it, vi } from 'vitest';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import type { AnalyticsTotals } from '../../../../../../src/shared/messages.ts';
import AnalyticsMetricStrip from '../../../../../../src/web/entities/analytics/ui/AnalyticsMetricStrip.vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';

enableAutoUnmount(afterEach);

const totals: AnalyticsTotals = {
  chats: 100,
  giftEvents: 0,
  gifts: 0,
  diamonds: 0,
  likeEvents: 0,
  likes: 0,
  joins: 0,
  follows: 0,
  shares: 0,
  peakViewers: 0,
};

function mountStrip() {
  return mountWithPlugins(AnalyticsMetricStrip, {
    attachTo: document.body,
    props: {
      totals,
      previousTotals: null,
      days: [{ day: 20281, ...totals }],
      startDay: 20275,
      endDay: 20281,
    },
  });
}

describe('AnalyticsMetricStrip preview', () => {
  it('opens the metric preview after the hover delay and closes on leave', async () => {
    vi.useFakeTimers();
    const wrapper = mountStrip();
    const card = wrapper.findAll('.kpi-strip__card')[0]!;
    await card.trigger('pointerenter');
    vi.advanceTimersByTime(300);
    await flushPromises();
    const preview = document.body.querySelector('.kpi-preview');
    expect(preview).toBeTruthy();
    expect(preview!.textContent).toContain('100');
    await card.trigger('pointerleave');
    await flushPromises();
    expect(document.body.querySelector('.kpi-preview')).toBeNull();
    vi.useRealTimers();
  });

  it('closes the preview on Escape', async () => {
    vi.useFakeTimers();
    const wrapper = mountStrip();
    await wrapper.findAll('.kpi-strip__card')[0]!.trigger('pointerenter');
    vi.advanceTimersByTime(300);
    await flushPromises();
    expect(document.body.querySelector('.kpi-preview')).toBeTruthy();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await flushPromises();
    expect(document.body.querySelector('.kpi-preview')).toBeNull();
    vi.useRealTimers();
  });

  it('derives the best day and average from the series', async () => {
    vi.useFakeTimers();
    const wrapper = mountStrip();
    await wrapper.findAll('.kpi-strip__card')[0]!.trigger('pointerenter');
    vi.advanceTimersByTime(300);
    await flushPromises();
    const stats = document.body.querySelector('.kpi-preview__stats');
    expect(stats!.textContent).toContain('100');
    vi.useRealTimers();
  });
});

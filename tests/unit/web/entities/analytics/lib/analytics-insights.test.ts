import { describe, expect, it } from 'vitest';
import type { AnalyticsDayRow, AnalyticsHourRow, AnalyticsTopViewer, AnalyticsTotals } from '../../../../../../src/shared/messages.ts';
import {
  activeShare,
  engagementMix,
  hourTotals,
  summarizeDaySeries,
  summarizeHourlySeries,
  topFans,
  totalsInteractions,
} from '../../../../../../src/web/entities/analytics/lib/analytics-insights.ts';

const zero: AnalyticsTotals = {
  chats: 0,
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

function viewer(patch: Partial<AnalyticsTopViewer>): AnalyticsTopViewer {
  return { uniqueId: 'a', chats: 0, gifts: 0, diamonds: 0, likes: 0, shares: 0, interactions: 0, lastSeen: 0, ...patch };
}

describe('analytics insights', () => {
  it('summarizes a day series with peak and average', () => {
    const days: AnalyticsDayRow[] = [
      { ...zero, day: 10, chats: 4 },
      { ...zero, day: 12, chats: 7 },
    ];
    const summary = summarizeDaySeries(days, 10, 13, 'chats');
    expect(summary.total).toBe(11);
    expect(summary.average).toBeCloseTo(11 / 4);
    expect(summary.peakValue).toBe(7);
    expect(summary.peakDay).toBe(12);
    expect(summary.peakIndex).toBe(2);
  });

  it('handles empty spans', () => {
    const summary = summarizeDaySeries([], 10, 9, 'likes');
    expect(summary.total).toBe(0);
    expect(summary.peakDay).toBeNull();
  });

  it('summarizes hourly totals with peak hour', () => {
    const summary = summarizeHourlySeries([0, 5, 12, 3]);
    expect(summary.total).toBe(20);
    expect(summary.peakValue).toBe(12);
    expect(summary.peakHour).toBe(2);
  });

  it('computes an engagement mix that sums to one', () => {
    const mix = engagementMix({ ...zero, chats: 10, gifts: 5, likes: 5, diamonds: 0 });
    const sum = mix.reduce((acc, slice) => acc + slice.share, 0);
    expect(sum).toBeCloseTo(1);
    expect(mix.find((slice) => slice.metric === 'chats')?.share).toBeCloseTo(0.5);
  });

  it('returns an empty mix when there is no activity', () => {
    expect(engagementMix(zero).every((slice) => slice.share === 0)).toBe(true);
  });

  it('picks top fans and interaction shares', () => {
    const viewers = [viewer({ uniqueId: 'a', interactions: 100 }), viewer({ uniqueId: 'b', interactions: 30 })];
    expect(topFans(viewers, 1).map((row) => row.uniqueId)).toEqual(['a']);
    expect(totalsInteractions(viewers)).toBe(130);
    expect(activeShare(65, 130)).toBeCloseTo(0.5);
    expect(activeShare(10, 0)).toBe(0);
  });

  it('maps hour rows onto a fixed 24-slot series', () => {
    const hours: AnalyticsHourRow[] = [{ ...zero, hour: 9, chats: 6 }];
    const series = hourTotals(hours, 'chats');
    expect(series).toHaveLength(24);
    expect(series[9]).toBe(6);
    expect(series[0]).toBe(0);
  });
});

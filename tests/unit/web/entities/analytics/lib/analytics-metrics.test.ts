import { describe, expect, it } from 'vitest';
import type { AnalyticsDayRow, AnalyticsHourRow, AnalyticsTopViewer, AnalyticsTotals } from '../../../../../../src/shared/messages.ts';
import {
  fillDaySeries,
  formatHourLabel,
  hourBucketOfTimestamp,
  hourRowsToSeries,
  isAnalyticsMetric,
  viewersToHourlySeries,
} from '../../../../../../src/web/entities/analytics/lib/analytics-metrics.ts';

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

describe('analytics metrics', () => {
  it('fills missing days with zero', () => {
    const days: AnalyticsDayRow[] = [
      { ...zero, day: 10, chats: 4 },
      { ...zero, day: 12, chats: 7 },
    ];
    expect(fillDaySeries(days, 10, 13, 'chats')).toEqual([4, 0, 7, 0]);
  });

  it('maps host hour rows into per-metric series', () => {
    const hours: AnalyticsHourRow[] = [
      { ...zero, hour: 3, chats: 5, peakViewers: 40 },
      { ...zero, hour: 30, chats: 9 },
    ];
    const series = hourRowsToSeries(hours);
    expect(series.chats[3]).toBe(5);
    expect(series.peakViewers[3]).toBe(40);
    expect(series.chats.reduce((sum, value) => sum + value, 0)).toBe(5);
  });

  it('buckets timestamps into local hours of the requested day', () => {
    const day = 20_000;
    const offset = -5 * 3600;
    const localNinePm = day * 86_400 + 21 * 3600 - offset;
    expect(hourBucketOfTimestamp(localNinePm, day, offset)).toBe(21);
    expect(hourBucketOfTimestamp(localNinePm + 4 * 3600, day, offset)).toBe(-1);
    expect(hourBucketOfTimestamp(-1, day, offset)).toBe(-1);
  });

  it('approximates hourly series from viewer last activity', () => {
    const day = 20_000;
    const at = (hour: number) => day * 86_400 + hour * 3600;
    const series = viewersToHourlySeries(
      [viewer({ chats: 3, gifts: 1, lastSeen: at(8) }), viewer({ uniqueId: 'b', chats: 2, lastSeen: at(8) })],
      day,
      0,
    );
    expect(series.chats[8]).toBe(5);
    expect(series.gifts[8]).toBe(1);
    expect(series.peakViewers[8]).toBe(2);
  });

  it('validates metric names and formats hour ticks', () => {
    expect(isAnalyticsMetric('likes')).toBe(true);
    expect(isAnalyticsMetric('shares')).toBe(false);
    expect(formatHourLabel(7)).toBe('07:00');
    expect(formatHourLabel(40)).toBe('23:00');
  });
});

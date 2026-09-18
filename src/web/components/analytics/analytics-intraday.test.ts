import { expect, test } from 'bun:test';

import type { AnalyticsTopViewer } from '../../../shared/messages.ts';
import {
  buildHourlySeries,
  formatHourLabel,
  formatTime,
  hourBucketOfTimestamp,
  hourRowsToSeries,
  HOURS_PER_DAY,
} from './analytics-intraday.ts';
import { ANALYTICS_METRICS, METRIC_COLOR_VAR, METRIC_TONE } from './analytics-chart.ts';
import { formatZoneShort, systemDay, tzOffsetSecsForDay } from './analytics-range.ts';

const OFFSET = 0;
const DAY = 20_300; // label-based day index
const hourTs = (hour: number, minute = 0): number => DAY * 86_400 + hour * 3_600 + minute * 60;

function viewer(uniqueId: string, lastSeen: number, chats = 0, likes = 0): AnalyticsTopViewer {
  return { uniqueId, chats, gifts: 0, diamonds: 0, likes, shares: 0, interactions: chats + likes, lastSeen };
}

test('hourBucketOfTimestamp buckets inside the day and rejects outside', () => {
  expect(hourBucketOfTimestamp(hourTs(0), DAY, OFFSET)).toBe(0);
  expect(hourBucketOfTimestamp(hourTs(23, 59), DAY, OFFSET)).toBe(23);
  expect(hourBucketOfTimestamp(hourTs(12, 30), DAY, OFFSET)).toBe(12);
  expect(hourBucketOfTimestamp(hourTs(5) - 1, DAY, OFFSET)).toBe(4);
  expect(hourBucketOfTimestamp((DAY - 1) * 86_400, DAY, OFFSET)).toBe(-1);
  expect(hourBucketOfTimestamp((DAY + 1) * 86_400, DAY, OFFSET)).toBe(-1);
  expect(hourBucketOfTimestamp(-5, DAY, OFFSET)).toBe(-1);
});

test('hourBucketOfTimestamp shifts with a zone offset', () => {
  // 23:00 UTC sits in the next local day at UTC+2.
  expect(hourBucketOfTimestamp(hourTs(23), DAY + 1, 7_200)).toBe(1);
  expect(hourBucketOfTimestamp(hourTs(23), DAY, 7_200)).toBe(-1);
  // 01:00 UTC sits in the previous local day at UTC-5.
  expect(hourBucketOfTimestamp(hourTs(1), DAY - 1, -18_000)).toBe(20);
});

test('buildHourlySeries attributes the active metric to the last-active hour', () => {
  const viewers = [
    viewer('alice', hourTs(9, 10), 4, 0),
    viewer('bob', hourTs(9, 45), 6, 0),
    viewer('carol', hourTs(21), 0, 7),
    viewer('mallory', (DAY - 1) * 86_400 + 3_600, 100, 100), // other day: ignored
  ];
  const chats = buildHourlySeries(viewers, DAY, 'chats', OFFSET);
  expect(chats).toHaveLength(HOURS_PER_DAY);
  expect(chats[9]).toBe(10);
  expect(chats[21]).toBe(0);
  expect(chats.reduce((a, b) => a + b, 0)).toBe(10);

  const likes = buildHourlySeries(viewers, DAY, 'likes', OFFSET);
  expect(likes[21]).toBe(7);

  const presence = buildHourlySeries(viewers, DAY, 'peakViewers', OFFSET);
  expect(presence[9]).toBe(2);
  expect(presence[21]).toBe(1);
});

test('hour labels are deterministic 24h ticks', () => {
  expect(formatHourLabel(0)).toBe('00:00');
  expect(formatHourLabel(13)).toBe('13:00');
  expect(formatHourLabel(23)).toBe('23:00');
});

test('formatTime renders system-zone clock times', () => {
  expect(formatTime(0, 'en')).toBe('—');
  // Self-consistent with the runtime zone: recompute the expected wall time.
  const ts = Date.UTC(2026, 8, 17, 12, 5) / 1_000;
  const expected = new Date(ts * 1_000).toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
  });
  expect(formatTime(ts, 'en')).toBe(expected);
  expect(formatTime(ts, 'en')).toMatch(/^\d{2}:\d{2}$/);
});

test('systemDay and tzOffsetSecsForDay agree with the runtime zone', () => {
  const probe = new Date(2026, 8, 17, 19, 34, 0); // local wall time
  const day = systemDay(probe.getTime());
  const roundTrip = new Date(day * 86_400_000);
  expect(roundTrip.getUTCFullYear()).toBe(2026);
  expect(roundTrip.getUTCMonth()).toBe(8);
  expect(roundTrip.getUTCDate()).toBe(17);
  expect(tzOffsetSecsForDay(day)).toBe(-probe.getTimezoneOffset() * 60);
});

test('formatZoneShort exposes the system zone label', () => {
  expect(typeof formatZoneShort('en')).toBe('string');
  expect(typeof formatZoneShort('es')).toBe('string');
});

test('every metric has a distinct tone and theme color', () => {
  const tones = new Set(ANALYTICS_METRICS.map((metric) => METRIC_TONE[metric]));
  expect(tones.size).toBe(ANALYTICS_METRICS.length);
  for (const metric of ANALYTICS_METRICS) {
    expect(METRIC_COLOR_VAR[metric]).toMatch(/^var\(--tt-\w+\)$/);
  }
});

test('hourRowsToSeries maps true backend hours per metric', () => {
  const series = hourRowsToSeries([
    { hour: 9, chats: 4, giftEvents: 0, gifts: 1, diamonds: 10, likeEvents: 0, likes: 0, joins: 0, follows: 0, shares: 0, peakViewers: 50 },
    { hour: 21, chats: 0, giftEvents: 0, gifts: 0, diamonds: 0, likeEvents: 0, likes: 7, joins: 0, follows: 0, shares: 0, peakViewers: 0 },
  ]);
  expect(series.chats[9]).toBe(4);
  expect(series.gifts[9]).toBe(1);
  expect(series.diamonds[9]).toBe(10);
  expect(series.peakViewers[9]).toBe(50);
  expect(series.likes[21]).toBe(7);
  expect(series.chats[10]).toBe(0);
  expect(series.chats).toHaveLength(HOURS_PER_DAY);
});

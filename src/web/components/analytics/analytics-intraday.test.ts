import { expect, test } from 'bun:test';

import type { AnalyticsTopViewer } from '../../../shared/messages.ts';
import {
  buildHourlySeries,
  formatHourLabel,
  formatTime,
  hourBucketOfTimestamp,
} from './analytics-intraday.ts';

const DAY = 20_300; // arbitrary UTC day bucket
const hourTs = (hour: number, minute = 0): number => DAY * 86_400 + hour * 3_600 + minute * 60;

function viewer(uniqueId: string, lastSeen: number, chats = 0, likes = 0): AnalyticsTopViewer {
  return { uniqueId, chats, gifts: 0, diamonds: 0, likes, shares: 0, interactions: chats + likes, lastSeen };
}

test('hourBucketOfTimestamp buckets inside the day and rejects outside', () => {
  expect(hourBucketOfTimestamp(hourTs(0), DAY)).toBe(0);
  expect(hourBucketOfTimestamp(hourTs(23, 59), DAY)).toBe(23);
  expect(hourBucketOfTimestamp(hourTs(12, 30), DAY)).toBe(12);
  expect(hourBucketOfTimestamp(hourTs(5) - 1, DAY)).toBe(4);
  expect(hourBucketOfTimestamp((DAY - 1) * 86_400, DAY)).toBe(-1);
  expect(hourBucketOfTimestamp((DAY + 1) * 86_400, DAY)).toBe(-1);
  expect(hourBucketOfTimestamp(-5, DAY)).toBe(-1);
});

test('buildHourlySeries attributes the active metric to the last-active hour', () => {
  const viewers = [
    viewer('alice', hourTs(9, 10), 4, 0),
    viewer('bob', hourTs(9, 45), 6, 0),
    viewer('carol', hourTs(21), 0, 7),
    viewer('mallory', (DAY - 1) * 86_400 + 3_600, 100, 100), // yesterday: ignored
  ];
  const chats = buildHourlySeries(viewers, DAY, 'chats');
  expect(chats).toHaveLength(24);
  expect(chats[9]).toBe(10);
  expect(chats[21]).toBe(0);
  expect(chats.reduce((a, b) => a + b, 0)).toBe(10);

  const likes = buildHourlySeries(viewers, DAY, 'likes');
  expect(likes[21]).toBe(7);

  const presence = buildHourlySeries(viewers, DAY, 'peakViewers');
  expect(presence[9]).toBe(2);
  expect(presence[21]).toBe(1);
});

test('hour and time labels are stable UTC strings', () => {
  expect(formatHourLabel(0, 'en')).toBe('00:00');
  expect(formatHourLabel(13, 'en')).toBe('13:00');
  expect(formatTime(0, 'en')).toBe('—');
  expect(formatTime(hourTs(8, 5), 'en')).toBe('08:05');
});

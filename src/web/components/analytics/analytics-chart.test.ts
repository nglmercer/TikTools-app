import { expect, test } from 'bun:test';

import { buildLineGeometry, fillDaySeries, type AnalyticsMetric } from './analytics-chart.ts';
import type { AnalyticsDayRow } from '../../../shared/messages.ts';

function row(day: number, chats: number): AnalyticsDayRow {
  return {
    day,
    chats,
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
}

test('fillDaySeries fills gaps with zeros', () => {
  const values = fillDaySeries([row(10, 3), row(12, 5)], 10, 13, 'chats' as AnalyticsMetric);
  expect(values).toEqual([3, 0, 5, 0]);
});

test('buildLineGeometry scales against the max value', () => {
  const geometry = buildLineGeometry([0, 50, 100], 120, 60, 10);
  expect(geometry.max).toBe(100);
  expect(geometry.points).toHaveLength(3);
  // Baseline, middle, top.
  expect(geometry.points[0]?.y).toBe(50);
  expect(geometry.points[1]?.y).toBe(30);
  expect(geometry.points[2]?.y).toBe(10);
  expect(geometry.points[0]?.x).toBe(10);
  expect(geometry.points[2]?.x).toBe(110);
  expect(geometry.line.startsWith('M10,50')).toBe(true);
  expect(geometry.area.endsWith('Z')).toBe(true);
});

test('buildLineGeometry renders flat and empty series safely', () => {
  const flat = buildLineGeometry([0, 0], 120, 60, 10);
  expect(flat.max).toBe(0);
  expect(flat.points.map((point) => point.y)).toEqual([50, 50]);

  const single = buildLineGeometry([7], 120, 60, 10);
  expect(single.points[0]).toMatchObject({ x: 60, y: 10 });

  const empty = buildLineGeometry([], 120, 60, 10);
  expect(empty.line).toBe('');
  expect(empty.area).toBe('');
  expect(empty.points).toEqual([]);
});

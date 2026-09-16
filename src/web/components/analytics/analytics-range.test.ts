import { expect, test } from 'bun:test';

import {
  dayToIsoDate,
  formatCount,
  isoDateToDay,
  resolveAnalyticsRange,
  utcDay,
} from './analytics-range.ts';

test('utcDay buckets whole UTC days', () => {
  expect(utcDay(0)).toBe(0);
  expect(utcDay(86_399_999)).toBe(0);
  expect(utcDay(86_400_000)).toBe(1);
  expect(utcDay(1_751_616_000_000)).toBe(20_273);
});

test('resolveAnalyticsRange covers presets inclusively', () => {
  expect(resolveAnalyticsRange('today', 100)).toEqual({ startDay: 100, endDay: 100 });
  expect(resolveAnalyticsRange('7d', 100)).toEqual({ startDay: 94, endDay: 100 });
  expect(resolveAnalyticsRange('30d', 100)).toEqual({ startDay: 71, endDay: 100 });
  expect(resolveAnalyticsRange('90d', 100)).toEqual({ startDay: 11, endDay: 100 });
});

test('resolveAnalyticsRange normalizes custom spans', () => {
  expect(resolveAnalyticsRange('custom', 100, { startDay: 90, endDay: 95 })).toEqual({ startDay: 90, endDay: 95 });
  expect(resolveAnalyticsRange('custom', 100, { startDay: 95, endDay: 90 })).toEqual({ startDay: 90, endDay: 95 });
  expect(resolveAnalyticsRange('custom', 100)).toEqual({ startDay: 100, endDay: 100 });
});

test('iso dates round-trip through day buckets', () => {
  expect(dayToIsoDate(isoDateToDay('2026-07-06') ?? 0)).toBe('2026-07-06');
  expect(isoDateToDay('2026-13-01')).toBeNull();
  expect(isoDateToDay('2026-02-30')).toBeNull();
  expect(isoDateToDay('not-a-date')).toBeNull();
  expect(isoDateToDay('')).toBeNull();
});

test('formatCount groups large integers', () => {
  expect(formatCount(999, 'en')).toBe('999');
  expect(formatCount(1234, 'en')).toBe('1,234');
  expect(formatCount(12.6, 'en')).toBe('13');
});

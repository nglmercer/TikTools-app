import { describe, expect, it } from 'vitest';
import {
  dayToIsoDate,
  formatClockTime,
  formatSpan,
  isoDateToDay,
  isSingleDaySpan,
  resolveAnalyticsRange,
  systemDay,
} from '../../../../../../src/web/entities/analytics/lib/analytics-range.ts';
import { formatCount } from '@/shared/lib/intl.ts';

describe('analytics range', () => {
  it('resolves presets as inclusive spans ending today', () => {
    expect(resolveAnalyticsRange('today', 100)).toEqual({ startDay: 100, endDay: 100 });
    expect(resolveAnalyticsRange('7d', 100)).toEqual({ startDay: 94, endDay: 100 });
    expect(resolveAnalyticsRange('30d', 100)).toEqual({ startDay: 71, endDay: 100 });
    expect(resolveAnalyticsRange('90d', 100)).toEqual({ startDay: 11, endDay: 100 });
  });

  it('orders custom spans and falls back to today', () => {
    expect(resolveAnalyticsRange('custom', 100, { startDay: 90, endDay: 80 })).toEqual({ startDay: 80, endDay: 90 });
    expect(resolveAnalyticsRange('custom', 100)).toEqual({ startDay: 100, endDay: 100 });
  });

  it('round-trips ISO dates and rejects malformed ones', () => {
    const day = isoDateToDay('2026-10-19')!;
    expect(dayToIsoDate(day)).toBe('2026-10-19');
    expect(isoDateToDay('2026-02-30')).toBeNull();
    expect(isoDateToDay('19/10/2026')).toBeNull();
  });

  it('labels the system-local calendar day', () => {
    const noon = new Date(2026, 9, 19, 12).getTime();
    expect(dayToIsoDate(systemDay(noon))).toBe('2026-10-19');
  });

  it('detects single-day spans', () => {
    expect(isSingleDaySpan({ startDay: 5, endDay: 5 })).toBe(true);
    expect(isSingleDaySpan({ startDay: 5, endDay: 6 })).toBe(false);
  });

  it('formats counts, spans and clock times per locale', () => {
    expect(formatCount(1234.4, 'en')).toBe('1,234');
    expect(formatCount(1234, 'es')).toBe('1234');
    const day = isoDateToDay('2026-10-19')!;
    expect(formatSpan({ startDay: day, endDay: day }, 'en')).toBe('Oct 19, 2026');
    expect(formatSpan({ startDay: day - 6, endDay: day }, 'en')).toBe('Oct 13, 2026 – Oct 19, 2026');
    expect(formatClockTime(0, 'en')).toBe('—');
  });
});

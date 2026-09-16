export type AnalyticsRangeKey = 'today' | '7d' | '30d' | '90d' | 'custom';

export const ANALYTICS_RANGE_KEYS: AnalyticsRangeKey[] = ['today', '7d', '30d', '90d', 'custom'];

export type AnalyticsDaySpan = {
  startDay: number;
  endDay: number;
};

/** UTC day bucket matching the Rust host (`unix_secs / 86400`). */
export function utcDay(unixMs: number): number {
  return Math.floor(unixMs / 86_400_000);
}

/** Resolve a range chip to an inclusive UTC day span. */
export function resolveAnalyticsRange(
  range: AnalyticsRangeKey,
  todayDay: number,
  custom?: AnalyticsDaySpan,
): AnalyticsDaySpan {
  switch (range) {
    case 'today':
      return { startDay: todayDay, endDay: todayDay };
    case '7d':
      return { startDay: todayDay - 6, endDay: todayDay };
    case '30d':
      return { startDay: todayDay - 29, endDay: todayDay };
    case '90d':
      return { startDay: todayDay - 89, endDay: todayDay };
    case 'custom': {
      const start = custom?.startDay ?? todayDay;
      const end = custom?.endDay ?? todayDay;
      return start <= end ? { startDay: start, endDay: end } : { startDay: end, endDay: start };
    }
  }
}

/** Day bucket to an ISO date (`YYYY-MM-DD`) for date inputs. */
export function dayToIsoDate(day: number): string {
  return new Date(day * 86_400_000).toISOString().slice(0, 10);
}

/** Strict ISO date to day bucket; `null` for malformed input. */
export function isoDateToDay(iso: string): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso.trim());
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const date = Number(match[3]);
  if (month < 1 || month > 12 || date < 1 || date > 31) return null;
  const ms = Date.UTC(year, month - 1, date);
  const check = new Date(ms);
  if (check.getUTCFullYear() !== year || check.getUTCMonth() !== month - 1 || check.getUTCDate() !== date) {
    return null;
  }
  return Math.floor(ms / 86_400_000);
}

/** Short localized day label for chart axes. */
export function formatDayLabel(day: number, locale: string): string {
  return new Date(day * 86_400_000).toLocaleDateString(locale === 'es' ? 'es-ES' : 'en-US', {
    month: 'short',
    day: 'numeric',
    timeZone: 'UTC',
  });
}

/** Grouped integer display (`1,234`). */
export function formatCount(value: number, locale: string): string {
  return Math.round(value).toLocaleString(locale === 'es' ? 'es-ES' : 'en-US');
}

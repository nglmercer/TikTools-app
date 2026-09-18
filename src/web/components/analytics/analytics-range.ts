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

/**
 * System IANA zone name (`Asia/Karachi`, `America/New_York`, …), `UTC` when
 * the runtime does not expose one. All intraday rendering uses this zone so
 * Today matches the system clock in the tray.
 */
export function systemTimeZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  } catch {
    return 'UTC';
  }
}

/**
 * System-local day index for an epoch-ms instant. The index is the UTC
 * midnight sharing the local calendar label, so `formatDayDate` keeps
 * rendering it with `timeZone: 'UTC'` and the label stays exact in any zone.
 * Matches the Rust frame `floor((unix_secs + offset) / 86400)` at local noon.
 */
export function systemDay(unixMs: number): number {
  const date = new Date(unixMs);
  return Math.floor(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) / 86_400_000);
}

/**
 * Zone offset (local = UTC + offset, seconds) of the system zone at a local
 * day index. DST transitions inside the probed day can skew this by the
 * transition step; spans use the end day's offset.
 */
export function tzOffsetSecsForDay(day: number): number {
  return -new Date(day * 86_400_000 + 43_200_000).getTimezoneOffset() * 60;
}

/** Short zone label (`EDT`, `GMT+5`) for subtitles, in the system zone. */
export function formatZoneShort(locale: string): string {
  try {
    const parts = new Intl.DateTimeFormat(locale === 'es' ? 'es-ES' : 'en-US', {
      timeZone: systemTimeZone(),
      timeZoneName: 'short',
    }).formatToParts(new Date());
    return parts.find((part) => part.type === 'timeZoneName')?.value ?? '';
  } catch {
    return '';
  }
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

/**
 * Day index to an ISO date (`YYYY-MM-DD`) for date inputs. Frame-agnostic:
 * indices are label-based, so this maps UTC and system-local days alike.
 */
export function dayToIsoDate(day: number): string {
  return new Date(day * 86_400_000).toISOString().slice(0, 10);
}

/** Strict ISO date to label-based day index; `null` for malformed input. */
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

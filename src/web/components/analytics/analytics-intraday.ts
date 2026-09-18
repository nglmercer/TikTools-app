import type { AnalyticsTopViewer } from '../../../shared/messages.ts';
import type { AnalyticsMetric } from './analytics-chart.ts';

export const HOURS_PER_DAY = 24;

/**
 * Intraday (Today) helpers built from values the summary already carries.
 *
 * The backend stores daily buckets only, so there is no per-hour event table.
 * What we DO have per viewer is `lastSeen` (unix seconds, a real datetime)
 * plus that viewer's metric totals. Grouping the selected metric by the
 * `lastSeen` hour gives Today a genuine time-based graphic instead of a
 * meaningless one-point line. It is an approximation (a viewer's totals are
 * attributed to their last-active hour) and is labelled as such in the UI.
 */

/** Hour 0..23 of a unix-seconds timestamp inside `day`, or -1 when outside. */
export function hourBucketOfTimestamp(unixSecs: number, day: number): number {
  if (!Number.isFinite(unixSecs) || unixSecs < 0) return -1;
  const dayOfTs = Math.floor(unixSecs / 86_400);
  if (dayOfTs !== day) return -1;
  const hour = Math.floor((unixSecs % 86_400) / 3_600);
  return hour >= 0 && hour < HOURS_PER_DAY ? hour : -1;
}

/** This viewer's contribution to the hourly series of the active metric. */
export function viewerHourWeight(viewer: AnalyticsTopViewer, metric: AnalyticsMetric): number {
  switch (metric) {
    case 'chats':
      return viewer.chats;
    case 'gifts':
      return viewer.gifts;
    case 'likes':
      return viewer.likes;
    case 'diamonds':
      return viewer.diamonds;
    case 'peakViewers':
      return 1;
  }
}

/** 24 hourly values for a single day, attributing each viewer to their last-active hour. */
export function buildHourlySeries(
  topViewers: AnalyticsTopViewer[],
  day: number,
  metric: AnalyticsMetric,
): number[] {
  const values = new Array<number>(HOURS_PER_DAY).fill(0);
  for (const viewer of topViewers) {
    const hour = hourBucketOfTimestamp(viewer.lastSeen, day);
    if (hour < 0) continue;
    const weight = viewerHourWeight(viewer, metric);
    if (weight > 0) values[hour] = (values[hour] ?? 0) + weight;
  }
  return values;
}

/** Localized hour tick (`00:00`, `13:00`) on the day's UTC scale. */
export function formatHourLabel(hour: number, locale: string): string {
  return new Date(hour * 3_600_000).toLocaleTimeString(locale === 'es' ? 'es-ES' : 'en-US', {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone: 'UTC',
  });
}

/** Localized clock time for a unix-seconds datetime (contributors "last active"). */
export function formatTime(unixSecs: number, locale: string): string {
  if (!Number.isFinite(unixSecs) || unixSecs <= 0) return '—';
  return new Date(unixSecs * 1_000).toLocaleTimeString(locale === 'es' ? 'es-ES' : 'en-US', {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone: 'UTC',
  });
}

/** Full localized day date (`Sep 18, 2026`) for Today headers. */
export function formatDayDate(day: number, locale: string): string {
  return new Date(day * 86_400_000).toLocaleDateString(locale === 'es' ? 'es-ES' : 'en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

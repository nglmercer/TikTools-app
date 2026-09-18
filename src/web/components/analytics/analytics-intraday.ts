import type { AnalyticsHourRow, AnalyticsTopViewer } from '../../../shared/messages.ts';
import { STACKABLE_METRICS, type AnalyticsMetric } from './analytics-chart.ts';
import { formatZoneShort, systemTimeZone } from './analytics-range.ts';

export const HOURS_PER_DAY = 24;

/**
 * Intraday (Today) helpers in the system-zone frame.
 *
 * The backend returns true per-hour counters (`summary.hours`) for single-day
 * spans recorded after the hourly table landed. For older data the frontend
 * falls back to grouping each viewer's metric total by their `lastSeen` hour
 * (`buildHourlySeries`): an approximation, labelled as such in the UI.
 *
 * Frame convention: `day` is the label-based index from `systemDay()`, and
 * `offsetSecs` is the zone offset (local = UTC + offset) the summary was
 * requested with. Day labels keep rendering with `timeZone: 'UTC'` because
 * the index *is* the UTC midnight sharing the local label; clock times
 * render in the system zone.
 */

/** Local hour 0..23 of a unix-seconds timestamp inside `day`, else -1. */
export function hourBucketOfTimestamp(unixSecs: number, day: number, offsetSecs: number): number {
  if (!Number.isFinite(unixSecs) || unixSecs < 0) return -1;
  const shifted = unixSecs + offsetSecs;
  if (Math.floor(shifted / 86_400) !== day) return -1;
  const hour = Math.floor((shifted % 86_400 + 86_400) % 86_400 / 3_600);
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

/**
 * 24 hourly values for a single local day, attributing each viewer to their
 * last-active hour. Fallback for pre-hourly data; prefer `summary.hours`.
 */
export function buildHourlySeries(
  topViewers: AnalyticsTopViewer[],
  day: number,
  metric: AnalyticsMetric,
  offsetSecs: number,
): number[] {
  const values = new Array<number>(HOURS_PER_DAY).fill(0);
  for (const viewer of topViewers) {
    const hour = hourBucketOfTimestamp(viewer.lastSeen, day, offsetSecs);
    if (hour < 0) continue;
    const weight = viewerHourWeight(viewer, metric);
    if (weight > 0) values[hour] = (values[hour] ?? 0) + weight;
  }
  return values;
}

/**
 * True backend hour rows mapped to one 24-value series per metric, ready for
 * the stacked Today chart. Missing hours read as zero.
 */
export function hourRowsToSeries(hours: AnalyticsHourRow[]): Record<AnalyticsMetric, number[]> {
  const series = {
    chats: new Array<number>(HOURS_PER_DAY).fill(0),
    gifts: new Array<number>(HOURS_PER_DAY).fill(0),
    likes: new Array<number>(HOURS_PER_DAY).fill(0),
    diamonds: new Array<number>(HOURS_PER_DAY).fill(0),
    peakViewers: new Array<number>(HOURS_PER_DAY).fill(0),
  };
  for (const row of hours) {
    if (row.hour < 0 || row.hour >= HOURS_PER_DAY) continue;
    for (const metric of STACKABLE_METRICS) {
      series[metric][row.hour] = Math.max(0, row[metric] ?? 0);
    }
    series.peakViewers[row.hour] = Math.max(0, row.peakViewers ?? 0);
  }
  return series;
}

/** Deterministic 24h axis tick (`00:00`, `13:00`) for a wall-clock hour. */
export function formatHourLabel(hour: number): string {
  const clamped = Math.min(HOURS_PER_DAY - 1, Math.max(0, Math.floor(hour)));
  return `${String(clamped).padStart(2, '0')}:00`;
}

/** Localized clock time for a unix-seconds datetime, in the system zone. */
export function formatTime(unixSecs: number, locale: string): string {
  if (!Number.isFinite(unixSecs) || unixSecs <= 0) return '—';
  return new Date(unixSecs * 1_000).toLocaleTimeString(locale === 'es' ? 'es-ES' : 'en-US', {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone: systemTimeZone(),
  });
}

/** Full localized day date (`Sep 18, 2026`) for a label-based day index. */
export function formatDayDate(day: number, locale: string): string {
  return new Date(day * 86_400_000).toLocaleDateString(locale === 'es' ? 'es-ES' : 'en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

/** Single-day card subtitle: local date plus the wall-clock span and zone. */
export function formatDayTimeSubtitle(day: number, locale: string): string {
  const zone = formatZoneShort(locale);
  const span = '00:00–23:59';
  return zone ? `${formatDayDate(day, locale)} · ${span} ${zone}` : `${formatDayDate(day, locale)} · ${span}`;
}

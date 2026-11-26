import type { AnalyticsDayRow, AnalyticsHourRow, AnalyticsTopViewer, AnalyticsTotals } from '../../../../shared/messages.ts';

export type AnalyticsMetric = 'chats' | 'gifts' | 'likes' | 'diamonds' | 'peakViewers';

export const analyticsMetrics: readonly AnalyticsMetric[] = ['chats', 'gifts', 'likes', 'diamonds', 'peakViewers'];

export const stackableMetrics: readonly AnalyticsMetric[] = ['chats', 'gifts', 'likes', 'diamonds'];

export const metricColor: Record<AnalyticsMetric, string> = {
  chats: 'var(--color-chart-1)',
  gifts: 'var(--color-chart-2)',
  likes: 'var(--color-chart-3)',
  diamonds: 'var(--color-chart-4)',
  peakViewers: 'var(--color-chart-5)',
};

export const HOURS_PER_DAY = 24;

export type MetricSeries = Record<AnalyticsMetric, number[]>;

export function isAnalyticsMetric(value: unknown): value is AnalyticsMetric {
  return analyticsMetrics.includes(value as AnalyticsMetric);
}

/** Chart selection: one metric or every metric overlaid in a single chart. */
export type AnalyticsMetricSelection = AnalyticsMetric | 'all';

export function isAnalyticsMetricSelection(value: unknown): value is AnalyticsMetricSelection {
  return value === 'all' || isAnalyticsMetric(value);
}

export function totalFor(totals: AnalyticsTotals, metric: AnalyticsMetric): number {
  return totals[metric] ?? 0;
}

export function fillDaySeries(days: readonly AnalyticsDayRow[], startDay: number, endDay: number, metric: AnalyticsMetric): number[] {
  const byDay = new Map(days.map((row) => [row.day, row[metric] ?? 0]));
  return Array.from({ length: Math.max(0, endDay - startDay + 1) }, (_, index) => byDay.get(startDay + index) ?? 0);
}

function emptySeries(): MetricSeries {
  return Object.fromEntries(analyticsMetrics.map((metric) => [metric, new Array<number>(HOURS_PER_DAY).fill(0)])) as MetricSeries;
}

export function hourRowsToSeries(hours: readonly AnalyticsHourRow[]): MetricSeries {
  const series = emptySeries();
  for (const row of hours) {
    if (row.hour < 0 || row.hour >= HOURS_PER_DAY) continue;
    for (const metric of analyticsMetrics) series[metric][row.hour] = Math.max(0, row[metric] ?? 0);
  }
  return series;
}

export function hourBucketOfTimestamp(unixSecs: number, day: number, offsetSecs: number): number {
  if (!Number.isFinite(unixSecs) || unixSecs < 0) return -1;
  const shifted = unixSecs + offsetSecs;
  if (Math.floor(shifted / 86_400) !== day) return -1;
  return Math.floor((((shifted % 86_400) + 86_400) % 86_400) / 3_600);
}

function viewerWeight(viewer: AnalyticsTopViewer, metric: AnalyticsMetric): number {
  return metric === 'peakViewers' ? 1 : viewer[metric];
}

export function viewersToHourlySeries(viewers: readonly AnalyticsTopViewer[], day: number, offsetSecs: number): MetricSeries {
  const series = emptySeries();
  for (const viewer of viewers) {
    const hour = hourBucketOfTimestamp(viewer.lastSeen, day, offsetSecs);
    if (hour < 0) continue;
    for (const metric of analyticsMetrics) series[metric][hour]! += viewerWeight(viewer, metric);
  }
  return series;
}

export const WEEKDAYS_PER_WEEK = 7;

/**
 * Average activity per weekday (0 = Sunday, JS convention) across the span.
 * Day numbers are UTC-pinned epoch days; epoch day 0 was a Thursday.
 * Averages instead of totals because a span rarely contains every weekday
 * the same number of times — totals would inflate the overrepresented days.
 */
export function dayRowsToWeekdaySeries(days: readonly AnalyticsDayRow[], startDay: number, endDay: number): MetricSeries {
  const sums = Object.fromEntries(analyticsMetrics.map((metric) => [metric, new Array<number>(WEEKDAYS_PER_WEEK).fill(0)])) as MetricSeries;
  const counts = new Array<number>(WEEKDAYS_PER_WEEK).fill(0);
  const weekdayOf = (day: number): number => (((day + 4) % WEEKDAYS_PER_WEEK) + WEEKDAYS_PER_WEEK) % WEEKDAYS_PER_WEEK;
  for (let day = startDay; day <= endDay; day += 1) counts[weekdayOf(day)]! += 1;
  const byDay = new Map(days.map((row) => [row.day, row]));
  for (let day = startDay; day <= endDay; day += 1) {
    const row = byDay.get(day);
    if (!row) continue;
    const weekday = weekdayOf(day);
    for (const metric of analyticsMetrics) sums[metric][weekday]! += Math.max(0, row[metric] ?? 0);
  }
  return Object.fromEntries(
    analyticsMetrics.map((metric) => [
      metric,
      sums[metric].map((sum, weekday) => {
        const count = counts[weekday] ?? 0;
        return count > 0 ? Math.round(sum / count) : 0;
      }),
    ]),
  ) as MetricSeries;
}

export function formatHourLabel(hour: number): string {
  return `${String(Math.min(HOURS_PER_DAY - 1, Math.max(0, Math.floor(hour)))).padStart(2, '0')}:00`;
}

import type { AnalyticsDayRow } from '../../../shared/messages.ts';

export type AnalyticsMetric = 'chats' | 'gifts' | 'likes' | 'diamonds' | 'peakViewers';

export const ANALYTICS_METRICS: AnalyticsMetric[] = ['chats', 'gifts', 'likes', 'diamonds', 'peakViewers'];

export function metricValue(row: AnalyticsDayRow, metric: AnalyticsMetric): number {
  return row[metric] ?? 0;
}

/** A span with a single day cannot render a trend line — callers should show a breakdown instead. */
export function isSingleDaySpan(startDay: number, endDay: number): boolean {
  return endDay <= startDay;
}

export type MetricBreakdownEntry = {
  metric: AnalyticsMetric;
  value: number;
  /** 0..1 fraction of the largest metric in the same day. */
  fraction: number;
};

/** Normalized per-metric breakdown for a single day (Today / single-day custom). */
export function buildMetricBreakdown(values: Record<AnalyticsMetric, number>): MetricBreakdownEntry[] {
  const entries = ANALYTICS_METRICS.map((metric) => ({ metric, value: Math.max(0, values[metric] ?? 0) }));
  const max = entries.reduce((top, entry) => Math.max(top, entry.value), 0);
  return entries.map((entry) => ({
    ...entry,
    fraction: max > 0 ? entry.value / max : 0,
  }));
}

/** Fill every day in the span with its metric value (zero when missing). */
export function fillDaySeries(
  days: AnalyticsDayRow[],
  startDay: number,
  endDay: number,
  metric: AnalyticsMetric,
): number[] {
  const byDay = new Map(days.map((row) => [row.day, metricValue(row, metric)]));
  const values: number[] = [];
  for (let day = startDay; day <= endDay; day += 1) {
    values.push(byDay.get(day) ?? 0);
  }
  return values;
}

export type ChartPoint = {
  x: number;
  y: number;
  value: number;
};

export type ChartGeometry = {
  line: string;
  area: string;
  points: ChartPoint[];
  max: number;
};

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

/**
 * Line/area geometry for the activity chart. Values scale against their own
 * max; a flat zero series renders along the baseline.
 */
export function buildLineGeometry(
  values: number[],
  width: number,
  height: number,
  pad = 10,
): ChartGeometry {
  const max = values.reduce((top, value) => Math.max(top, value), 0);
  const innerWidth = Math.max(0, width - pad * 2);
  const innerHeight = Math.max(0, height - pad * 2);
  const points = values.map((value, index) => {
    const x = values.length <= 1
      ? pad + innerWidth / 2
      : pad + (innerWidth * index) / (values.length - 1);
    const y = max > 0 ? pad + innerHeight - (innerHeight * value) / max : pad + innerHeight;
    return { x: round(x), y: round(y), value };
  });
  if (points.length === 0) {
    return { line: '', area: '', points, max };
  }
  const line = points
    .map((point, index) => `${index === 0 ? 'M' : 'L'}${point.x},${point.y}`)
    .join(' ');
  const first = points[0];
  const last = points[points.length - 1];
  const base = round(pad + innerHeight);
  const area = first && last ? `${line} L${last.x},${base} L${first.x},${base} Z` : '';
  return { line, area, points, max };
}

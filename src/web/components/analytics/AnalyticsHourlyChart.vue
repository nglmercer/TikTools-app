<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import { METRIC_COLOR_VAR, STACKABLE_METRICS, type AnalyticsMetric } from './analytics-chart.ts';
import { formatHourLabel, HOURS_PER_DAY } from './analytics-intraday.ts';
import { formatCount } from './analytics-range.ts';

type AnalyticsHourlyChartProps = {
  locale: Locale;
  /** One 24-value series per metric (true backend hours or the fallback). */
  series: Record<AnalyticsMetric, number[]>;
  /** Selected metric: stacked counts highlight it, Viewers shows peaks alone. */
  active: AnalyticsMetric;
  metricLabel: (metric: AnalyticsMetric) => string;
};

const CHART_WIDTH = 720;
const CHART_HEIGHT = 220;
const CHART_PAD = 12;

/**
 * Today graphic: one stacked bar per hour over the four count metrics, each
 * in its own color with a legend, so a single active hour still reads rich.
 * The Viewers chip switches to peak viewers alone (different unit, own
 * scale). No chart library, same SVG language as AnalyticsChart.
 */
export const AnalyticsHourlyChart = defineVueFunctional<AnalyticsHourlyChartProps>((props) => {
  const { locale, series, active, metricLabel } = props;
  const viewersOnly = active === 'peakViewers';
  const drawn: AnalyticsMetric[] = viewersOnly ? ['peakViewers'] : [...STACKABLE_METRICS];

  const at = (metric: AnalyticsMetric, hour: number): number => series[metric]?.[hour] ?? 0;
  const stackTotal = (hour: number): number =>
    STACKABLE_METRICS.reduce((sum, metric) => sum + at(metric, hour), 0);
  const max = viewersOnly
    ? series.peakViewers.reduce((top, value) => Math.max(top, value), 0)
    : Array.from({ length: HOURS_PER_DAY }, (_, hour) => stackTotal(hour)).reduce(
        (top, value) => Math.max(top, value),
        0,
      );

  const base = CHART_HEIGHT - CHART_PAD;
  const innerHeight = Math.max(0, CHART_HEIGHT - CHART_PAD * 2);
  const slot = (CHART_WIDTH - CHART_PAD * 2) / HOURS_PER_DAY;
  const barWidth = Math.max(2, Math.min(18, slot * 0.62));
  const gridSteps = [0, 0.5, 1];
  const tickHours = [0, 6, 12, 18, 23];
  const activeLabel = metricLabel(active);

  return (
    <div class="analytics-hourly">
      <div class="analytics-legend" role="list" aria-label={activeLabel}>
        {drawn.map((metric) => (
          <span
            key={metric}
            role="listitem"
            class={`analytics-legend__item${metric === active ? ' is-active' : ''}`}
          >
            <span class="analytics-legend__dot" style={{ background: METRIC_COLOR_VAR[metric] }} />
            {metricLabel(metric)}
          </span>
        ))}
      </div>
      <svg
        class="analytics-chart"
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        role="img"
        aria-label={`${activeLabel}: ${formatCount(max, locale)}`}
      >
        {gridSteps.map((step) => {
          const y = CHART_PAD + innerHeight * (1 - step);
          return (
            <g key={step}>
              <line x1={CHART_PAD} y1={y} x2={CHART_WIDTH - CHART_PAD} y2={y} class="analytics-chart__grid" />
              <text x={CHART_WIDTH - CHART_PAD} y={y - 4} text-anchor="end" class="analytics-chart__tick">
                {formatCount(max * step, locale)}
              </text>
            </g>
          );
        })}
        {Array.from({ length: HOURS_PER_DAY }, (_, hour) => {
          const x = CHART_PAD + slot * hour + (slot - barWidth) / 2;
          if (viewersOnly) {
            const value = at('peakViewers', hour);
            const height = max > 0 ? (innerHeight * value) / max : 0;
            return (
              <rect
                key={hour}
                x={Math.round(x * 100) / 100}
                y={Math.round((base - height) * 100) / 100}
                width={Math.round(barWidth * 100) / 100}
                height={Math.round(Math.max(height, value > 0 ? 2 : 0) * 100) / 100}
                rx={3}
                fill={METRIC_COLOR_VAR.peakViewers}
                opacity={value > 0 ? 0.9 : 0}
              >
                <title>{`${formatHourLabel(hour)} — ${activeLabel}: ${formatCount(value, locale)}`}</title>
              </rect>
            );
          }
          // Stacked bottom-up; the selected metric stays vivid, others dim.
          let offset = 0;
          return (
            <g key={hour}>
              {STACKABLE_METRICS.map((metric) => {
                const value = at(metric, hour);
                const height = max > 0 ? (innerHeight * value) / max : 0;
                const y = base - offset - height;
                offset += height;
                if (value <= 0) return null;
                return (
                  <rect
                    key={metric}
                    x={Math.round(x * 100) / 100}
                    y={Math.round(y * 100) / 100}
                    width={Math.round(barWidth * 100) / 100}
                    height={Math.round(Math.max(height, 2) * 100) / 100}
                    fill={METRIC_COLOR_VAR[metric]}
                    opacity={metric === active ? 0.95 : 0.28}
                  >
                    <title>{`${formatHourLabel(hour)} — ${metricLabel(metric)}: ${formatCount(value, locale)}`}</title>
                  </rect>
                );
              })}
            </g>
          );
        })}
        {tickHours.map((hour) => {
          const x = CHART_PAD + slot * hour + slot / 2;
          const anchor = hour === 0 ? 'start' : hour === 23 ? 'end' : 'middle';
          return (
            <text key={hour} x={Math.round(x * 100) / 100} y={CHART_HEIGHT - 2} text-anchor={anchor} class="analytics-chart__tick">
              {formatHourLabel(hour)}
            </text>
          );
        })}
      </svg>
      {max === 0 ? <p class="analytics-hourly__empty">{t(locale, 'analyticsNoHourlyData')}</p> : null}
    </div>
  );
});

export default AnalyticsHourlyChart;
</script>

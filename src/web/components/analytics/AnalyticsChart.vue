<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import type { AnalyticsDayRow } from '../../../shared/messages.ts';
import { formatCount, formatDayLabel } from './analytics-range.ts';
import { buildLineGeometry, fillDaySeries, type AnalyticsMetric } from './analytics-chart.ts';

type AnalyticsChartProps = {
  locale: string;
  days: AnalyticsDayRow[];
  startDay: number;
  endDay: number;
  metric: AnalyticsMetric;
  label: string;
};

const CHART_WIDTH = 720;
const CHART_HEIGHT = 220;
const CHART_PAD = 12;

let chartSequence = 0;

/**
 * Small SVG line/area chart for one daily metric series. No chart library.
 * Single-day spans render one centered bar (a one-point line would be an
 * invisible dot), multi-day spans render the usual line + area.
 */
export const AnalyticsChart = defineVueFunctional<AnalyticsChartProps>((props) => {
  const { locale, days, startDay, endDay, metric, label } = props;
  const values = fillDaySeries(days, startDay, endDay, metric);
  const geometry = buildLineGeometry(values, CHART_WIDTH, CHART_HEIGHT, CHART_PAD);
  const gradientId = `analytics-area-${(chartSequence += 1)}`;
  const gridSteps = [0, 0.5, 1];
  const showDots = values.length > 1 && values.length <= 62;
  const isSingle = values.length <= 1;

  const labelDays = isSingle
    ? [startDay]
    : [startDay, Math.round((startDay + endDay) / 2), endDay].filter(
        (day, index, all) => all.indexOf(day) === index,
      );

  const singleValue = isSingle ? (values[0] ?? 0) : 0;
  const singleBar = isSingle && singleValue > 0
    ? (() => {
        const base = CHART_HEIGHT - CHART_PAD;
        const top = CHART_PAD;
        const cx = CHART_WIDTH / 2;
        const barWidth = Math.min(120, CHART_WIDTH / 4);
        return { x: cx - barWidth / 2, y: top, width: barWidth, height: Math.max(0, base - top) };
      })()
    : null;

  return (
    <svg
      class="analytics-chart"
      viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
      role="img"
      aria-label={`${label}: ${formatCount(geometry.max, locale)} max`}
    >
      <defs>
        <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--tt-cyan)" stop-opacity="0.35" />
          <stop offset="100%" stop-color="var(--tt-cyan)" stop-opacity="0.02" />
        </linearGradient>
      </defs>
      {gridSteps.map((step) => {
        const y = CHART_PAD + (CHART_HEIGHT - CHART_PAD * 2) * (1 - step);
        return (
          <g key={step}>
            <line x1={CHART_PAD} y1={y} x2={CHART_WIDTH - CHART_PAD} y2={y} class="analytics-chart__grid" />
            <text x={CHART_WIDTH - CHART_PAD} y={y - 4} text-anchor="end" class="analytics-chart__tick">
              {formatCount(geometry.max * step, locale)}
            </text>
          </g>
        );
      })}
      {isSingle ? (
        <>
          {singleBar ? (
            <rect
              x={singleBar.x}
              y={singleBar.y}
              width={singleBar.width}
              height={singleBar.height}
              rx={8}
              class="analytics-chart__bar"
            />
          ) : null}
          <text x={CHART_WIDTH / 2} y={singleBar ? singleBar.y - 8 : CHART_HEIGHT / 2} text-anchor="middle" class="analytics-chart__value">
            {formatCount(singleValue, locale)}
          </text>
          {singleBar ? null : (
            <text x={CHART_WIDTH / 2} y={CHART_HEIGHT / 2 + 18} text-anchor="middle" class="analytics-chart__tick">
              {label}
            </text>
          )}
        </>
      ) : (
        <>
          {geometry.area ? <path d={geometry.area} fill={`url(#${gradientId})`} /> : null}
          {geometry.line ? <path d={geometry.line} class="analytics-chart__line" fill="none" /> : null}
          {showDots
            ? geometry.points.map((point, index) => (
                <circle key={index} cx={point.x} cy={point.y} r={values.length > 31 ? 1.6 : 2.6} class="analytics-chart__dot" />
              ))
            : null}
        </>
      )}
      {labelDays.map((day) => {
        const fraction = endDay === startDay ? 0.5 : (day - startDay) / (endDay - startDay);
        const x = CHART_PAD + (CHART_WIDTH - CHART_PAD * 2) * fraction;
        const anchor = day === startDay ? 'start' : day === endDay ? 'end' : 'middle';
        return (
          <text key={day} x={x} y={CHART_HEIGHT - 2} text-anchor={anchor} class="analytics-chart__tick">
            {formatDayLabel(day, locale)}
          </text>
        );
      })}
    </svg>
  );
});

export default AnalyticsChart;
</script>

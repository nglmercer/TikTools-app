<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import { formatCount } from './analytics-range.ts';
import { formatHourLabel, HOURS_PER_DAY } from './analytics-intraday.ts';

type AnalyticsHourlyChartProps = {
  locale: Locale;
  values: number[];
  label: string;
};

const CHART_WIDTH = 720;
const CHART_HEIGHT = 220;
const CHART_PAD = 12;

/** 24-bar intraday chart for the Today view. No chart library, same SVG language as AnalyticsChart. */
export const AnalyticsHourlyChart = defineVueFunctional<AnalyticsHourlyChartProps>((props) => {
  const { locale, values, label } = props;
  const hours = values.slice(0, HOURS_PER_DAY);
  while (hours.length < HOURS_PER_DAY) hours.push(0);
  const max = hours.reduce((top, value) => Math.max(top, value), 0);
  const base = CHART_HEIGHT - CHART_PAD;
  const innerHeight = Math.max(0, CHART_HEIGHT - CHART_PAD * 2);
  const slot = (CHART_WIDTH - CHART_PAD * 2) / HOURS_PER_DAY;
  const barWidth = Math.max(2, Math.min(18, slot * 0.62));
  const gridSteps = [0, 0.5, 1];
  const tickHours = [0, 6, 12, 18, 23];

  return (
    <div class="analytics-hourly">
      <svg
        class="analytics-chart"
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        role="img"
        aria-label={`${label}: ${formatCount(max, locale)}`}
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
        {hours.map((value, hour) => {
          const height = max > 0 ? (innerHeight * value) / max : 0;
          const x = CHART_PAD + slot * hour + (slot - barWidth) / 2;
          return (
            <rect
              key={hour}
              x={Math.round(x * 100) / 100}
              y={Math.round((base - height) * 100) / 100}
              width={Math.round(barWidth * 100) / 100}
              height={Math.round(Math.max(height, value > 0 ? 2 : 0) * 100) / 100}
              rx={3}
              class={`analytics-chart__bar${value > 0 && value === max ? ' is-peak' : ''}`}
            >
              <title>{`${formatHourLabel(hour, locale)} — ${formatCount(value, locale)} ${label}`}</title>
            </rect>
          );
        })}
        {tickHours.map((hour) => {
          const x = CHART_PAD + slot * hour + slot / 2;
          const anchor = hour === 0 ? 'start' : hour === 23 ? 'end' : 'middle';
          return (
            <text key={hour} x={Math.round(x * 100) / 100} y={CHART_HEIGHT - 2} text-anchor={anchor} class="analytics-chart__tick">
              {formatHourLabel(hour, locale)}
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

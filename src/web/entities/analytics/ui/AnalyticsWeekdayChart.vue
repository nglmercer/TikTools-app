<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatCount } from '@/shared/lib/intl.ts';
import { buildLineGeometry, seriesMax } from '../lib/chart-geometry.ts';
import { useChartHover } from '../lib/use-chart-hover.ts';
import ChartHoverTooltip from './ChartHoverTooltip.vue';
import {
  analyticsMetrics,
  metricColor,
  WEEKDAYS_PER_WEEK,
  type AnalyticsMetricSelection,
  type MetricSeries,
} from '../lib/analytics-metrics.ts';

/** Average-activity-per-weekday bars (single metric) or overlaid lines (all). */
const props = defineProps<{
  series: MetricSeries;
  metric: AnalyticsMetricSelection;
}>();

const box = { width: 720, height: 180, pad: 20 };
const gridSteps = [0, 0.5, 1];

const { t, locale } = useI18n();

// Localized short weekday labels. The week starts on the locale's first day
// when the runtime knows it (most of the app's locales start Monday).
const firstDay = computed(() => {
  try {
    const info = (new Intl.Locale(locale.value) as { weekInfo?: { firstDay?: number } }).weekInfo;
    if (info?.firstDay) return info.firstDay === 7 ? 0 : info.firstDay;
  } catch {
    // weekInfo unsupported — Monday start is right for most of the app's locales
  }
  return 1;
});
// Weekday indices (0 = Sunday) in display order — every series and label
// below follows this order, so bars and labels never drift apart.
const order = computed(() => Array.from({ length: WEEKDAYS_PER_WEEK }, (_, index) => (firstDay.value + index) % WEEKDAYS_PER_WEEK));
const weekdayLabels = computed(() => {
  const formatter = new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' });
  // 2026-11-23 is a Monday: weekday w sits (w + 6) % 7 days after it.
  const mondayUtc = Date.UTC(2026, 10, 23);
  return order.value.map((weekday) => formatter.format(new Date(mondayUtc + ((weekday + 6) % WEEKDAYS_PER_WEEK) * 86_400_000)));
});
const reorder = (input: readonly number[]): number[] => order.value.map((weekday) => Math.max(0, input[weekday] ?? 0));

const isAll = computed(() => props.metric === 'all');
const values = computed(() =>
  isAll.value ? [] : reorder(props.series[props.metric as Exclude<AnalyticsMetricSelection, 'all'>]),
);
const overlaid = computed(() =>
  isAll.value
    ? analyticsMetrics.map((metric) => ({
        metric,
        values: reorder(props.series[metric]),
      }))
    : [],
);
const hoverTitle = computed(() => (hoverIndex.value === null ? '' : weekdayLabels.value[hoverIndex.value] ?? ''));
const hoverRows = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return [];
  const metrics = isAll.value ? analyticsMetrics : [props.metric as Exclude<AnalyticsMetricSelection, 'all'>];
  return metrics.map((metric) => ({
    key: metric,
    color: metricColor[metric],
    label: t(`analytics.metrics.${metric}`),
    value: formatCount(reorder(props.series[metric])[index] ?? 0, locale.value),
    before: null,
  }));
});
const tooltipStyle = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return {};
  const ratio = slotX(index) / box.width;
  const translate = ratio < 0.15 ? '0' : ratio > 0.85 ? '-100%' : '-50%';
  return { left: `${ratio * 100}%`, transform: `translateX(${translate})` };
});
const hoverX = computed(() => (hoverIndex.value === null ? null : slotX(hoverIndex.value)));

const max = computed(() =>
  isAll.value
    ? overlaid.value.reduce((top, series) => Math.max(top, seriesMax(series.values)), 0)
    : values.value.reduce((top, value) => Math.max(top, value), 0),
);
const overlaidGeometry = computed(() =>
  overlaid.value.map((series) => ({ metric: series.metric, geometry: buildLineGeometry(series.values, box, max.value) })),
);
const label = computed(() => t(`analytics.metrics.${props.metric}`));

// ── Hover inspection — indices are display-order positions, matching the
// reordered series and the label ticks exactly. ───────────────────────────
const slotCount = computed(() => WEEKDAYS_PER_WEEK);
const { hoverIndex, onPointermove, onPointerleave, slotX } = useChartHover(box, slotCount);
const slot = (box.width - box.pad * 2) / WEEKDAYS_PER_WEEK;
const width = Math.max(2, Math.min(40, slot * 0.5));
const innerHeight = box.height - box.pad * 2;
const bars = computed(() =>
  values.value.map((value, index) => ({
    index,
    x: box.pad + slot * index + (slot - width) / 2,
    height: value > 0 && max.value > 0 ? Math.max(2, (innerHeight * value) / max.value) : 0,
    value,
  })),
);

</script>

<template>
  <div class="weekday-chart">
    <svg
      class="weekday-chart__svg"
      :viewBox="`0 0 ${box.width} ${box.height}`"
      role="img"
      :aria-label="`${label}: ${formatCount(max, locale)}`"
      @pointermove="onPointermove"
      @pointerleave="onPointerleave"
    >
    <g
      v-for="step in gridSteps"
      :key="step"
    >
      <line
        class="weekday-chart__grid"
        :x1="box.pad"
        :x2="box.width - box.pad"
        :y1="box.pad + innerHeight * (1 - step)"
        :y2="box.pad + innerHeight * (1 - step)"
      />
      <text
        class="weekday-chart__tick"
        :x="box.width - box.pad"
        :y="box.pad + innerHeight * (1 - step) - 5"
        text-anchor="end"
      >{{ formatCount(max * step, locale) }}</text>
    </g>
    <template v-if="isAll">
      <path
        v-for="series in overlaidGeometry"
        :key="series.metric"
        class="weekday-chart__line"
        :style="{ '--metric-color': metricColor[series.metric] }"
        :d="series.geometry.line"
      />
    </template>
    <template v-else>
      <rect
        v-for="bar in bars"
        :key="bar.index"
        :x="bar.x"
        :y="box.height - box.pad - bar.height"
        :width="width"
        :height="bar.height"
        rx="2"
        :fill="metricColor[metric as Exclude<AnalyticsMetricSelection, 'all'>]"
        :opacity="hoverIndex === null || hoverIndex === bar.index ? 0.9 : 0.35"
      />
    </template>
    <line
      v-if="hoverX !== null"
      class="weekday-chart__guide"
      :x1="hoverX"
      :x2="hoverX"
      :y1="box.pad"
      :y2="box.height - box.pad"
    />
    <text
      v-for="(tick, index) in weekdayLabels"
      :key="index"
      class="weekday-chart__tick"
      :x="box.pad + slot * index + slot / 2"
      :y="box.height - 4"
      text-anchor="middle"
    >{{ tick }}</text>
    </svg>
    <ChartHoverTooltip
      v-if="hoverIndex !== null"
      :style="tooltipStyle"
      :title="hoverTitle"
      :rows="hoverRows"
    />
  </div>
</template>

<style scoped>
.weekday-chart {
  position: relative;
}

.weekday-chart__svg {
  display: block;
  width: 100%;
  height: auto;
  cursor: crosshair;
}

.weekday-chart__guide {
  stroke: var(--color-text-subtle);
  stroke-width: 1;
  stroke-dasharray: 3 3;
  opacity: .6;
  pointer-events: none;
}

.weekday-chart__grid {
  stroke: var(--color-chart-grid);
  stroke-width: 1;
}

.weekday-chart__tick {
  fill: var(--color-text-subtle);
  font-family: var(--font-family);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.weekday-chart__line {
  fill: none;
  stroke: var(--metric-color);
  stroke-width: 1.8;
  stroke-linejoin: round;
  stroke-linecap: round;
}
</style>

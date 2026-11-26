<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsDayRow } from '../../../../shared/messages.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import { formatDayDate, formatDayLabel, type AnalyticsDaySpan } from '../lib/analytics-range.ts';
import { analyticsMetrics, fillDaySeries, metricColor, type AnalyticsMetricSelection } from '../lib/analytics-metrics.ts';
import { buildLineGeometry, seriesMax } from '../lib/chart-geometry.ts';
import { useChartHover } from '../lib/use-chart-hover.ts';
import ChartHoverTooltip from './ChartHoverTooltip.vue';

const props = defineProps<{
  days: readonly AnalyticsDayRow[];
  span: AnalyticsDaySpan;
  metric: AnalyticsMetricSelection;
  /** Same-length period immediately before, drawn as a dashed reference line. */
  previousDays?: readonly AnalyticsDayRow[];
}>();

const box = { width: 720, height: 220, pad: 20 };
const gridSteps = [0, 0.5, 1];

const { t, locale } = useI18n();
const isAll = computed(() => props.metric === 'all');
const values = computed(() =>
  isAll.value ? [] : fillDaySeries(props.days, props.span.startDay, props.span.endDay, props.metric as Exclude<AnalyticsMetricSelection, 'all'>),
);
// 'all' overlays every metric on the shared scale; the previous-period
// reference is dropped there — five dashed echoes would drown the signal.
const overlaid = computed(() =>
  isAll.value
    ? analyticsMetrics.map((metric) => ({
        metric,
        values: fillDaySeries(props.days, props.span.startDay, props.span.endDay, metric),
      }))
    : [],
);
const previousValues = computed(() => {
  if (!props.previousDays || isAll.value) return null;
  const length = props.span.endDay - props.span.startDay + 1;
  return fillDaySeries(props.previousDays, props.span.startDay - length, props.span.endDay - length, props.metric as Exclude<AnalyticsMetricSelection, 'all'>);
});
// The scale must cover both series or the comparison line lies about proportion.
const scaleMax = computed(() =>
  isAll.value
    ? overlaid.value.reduce((top, series) => Math.max(top, seriesMax(series.values)), 0)
    : Math.max(seriesMax(values.value), previousValues.value ? seriesMax(previousValues.value) : 0),
);
const geometry = computed(() => buildLineGeometry(values.value, box, scaleMax.value));
const overlaidGeometry = computed(() =>
  overlaid.value.map((series) => ({ metric: series.metric, values: series.values, geometry: buildLineGeometry(series.values, box, scaleMax.value) })),
);
const previousGeometry = computed(() => (previousValues.value ? buildLineGeometry(previousValues.value, box, scaleMax.value) : null));
const label = computed(() => t(`analytics.metrics.${props.metric}`));
const showDots = computed(() => !isAll.value && values.value.length <= 62);
const peakIndex = computed(() => {
  let best = -1;
  let bestValue = -1;
  values.value.forEach((value, index) => {
    if (value > bestValue) {
      bestValue = value;
      best = index;
    }
  });
  return best;
});
// ── Hover inspection ─────────────────────────────────────────────────────
const slotCount = computed(() => (isAll.value ? overlaid.value[0]?.values.length ?? 0 : values.value.length));
const { hoverIndex, onPointermove, onPointerleave, slotX } = useChartHover(box, slotCount);

const hoverDay = computed(() => (hoverIndex.value === null ? null : props.span.startDay + hoverIndex.value));
const hoverDateLabel = computed(() => (hoverDay.value === null ? '' : formatDayDate(hoverDay.value, locale.value)));
interface HoverRow {
  key: string;
  color: string;
  label: string;
  value: string;
  before: string | null;
}
const hoverRows = computed<HoverRow[]>(() => {
  const index = hoverIndex.value;
  if (index === null) return [];
  const rows: HoverRow[] = [];
  if (isAll.value) {
    for (const series of overlaidGeometry.value) {
      rows.push({
        key: series.metric,
        color: metricColor[series.metric],
        label: t(`analytics.metrics.${series.metric}`),
        value: formatCount(series.values[index] ?? 0, locale.value),
        before: null,
      });
    }
  } else {
    rows.push({
      key: 'current',
      color: metricColor[props.metric as Exclude<AnalyticsMetricSelection, 'all'>],
      label: label.value,
      value: formatCount(values.value[index] ?? 0, locale.value),
      before: previousValues.value ? formatCount(previousValues.value[index] ?? 0, locale.value) : null,
    });
  }
  return rows;
});
const tooltipStyle = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return {};
  const ratio = slotX(index) / box.width;
  const translate = ratio < 0.15 ? '0' : ratio > 0.85 ? '-100%' : '-50%';
  return { left: `${ratio * 100}%`, transform: `translateX(${translate})` };
});
const hoverX = computed(() => (hoverIndex.value === null ? null : slotX(hoverIndex.value)));
const hoverPoints = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return [];
  if (isAll.value) {
    return overlaidGeometry.value.map((series) => ({
      key: series.metric,
      color: metricColor[series.metric],
      x: series.geometry.points[index]?.x ?? 0,
      y: series.geometry.points[index]?.y ?? 0,
    }));
  }
  const point = geometry.value.points[index];
  return point ? [{ key: 'current', color: undefined as string | undefined, x: point.x, y: point.y }] : [];
});

const ticks = computed(() => {
  const { startDay, endDay } = props.span;
  return [...new Set([startDay, Math.round((startDay + endDay) / 2), endDay])].map((day) => ({
    day,
    x: box.pad + (box.width - box.pad * 2) * (endDay === startDay ? 0.5 : (day - startDay) / (endDay - startDay)),
    anchor: day === startDay ? 'start' : day === endDay ? 'end' : 'middle',
  }));
});
const gridLines = computed(() =>
  gridSteps.map((step) => ({
    step,
    y: box.pad + (box.height - box.pad * 2) * (1 - step),
    label: formatCount(scaleMax.value * step, locale.value),
  })),
);
</script>

<template>
  <div class="daily-chart">
    <svg
      class="daily-chart__svg"
      :viewBox="`0 0 ${box.width} ${box.height}`"
      role="img"
      :aria-label="`${label}: ${formatCount(scaleMax, locale)}`"
      :style="isAll ? undefined : { '--metric-color': metricColor[metric as Exclude<AnalyticsMetricSelection, 'all'>] }"
      @pointermove="onPointermove"
      @pointerleave="onPointerleave"
    >
    <g
      v-for="line in gridLines"
      :key="line.step"
    >
      <line
        class="daily-chart__grid"
        :x1="box.pad"
        :x2="box.width - box.pad"
        :y1="line.y"
        :y2="line.y"
      />
      <text
        class="daily-chart__tick"
        :x="box.width - box.pad"
        :y="line.y - 5"
        text-anchor="end"
      >{{ line.label }}</text>
    </g>
    <path
      v-if="previousGeometry?.line"
      class="daily-chart__previous"
      :d="previousGeometry.line"
    >
      <title>{{ t('analytics.comparePrevious') }}</title>
    </path>
    <template v-if="isAll">
      <path
        v-for="series in overlaidGeometry"
        :key="series.metric"
        class="daily-chart__line"
        :style="{ '--metric-color': metricColor[series.metric] }"
        :d="series.geometry.line"
      />
    </template>
    <path
      v-else-if="geometry.line"
      class="daily-chart__line"
      :d="geometry.line"
    />
    <template v-if="showDots">
      <circle
        v-for="(point, index) in geometry.points"
        :key="index"
        class="daily-chart__dot"
        :class="{ 'daily-chart__dot--peak': index === peakIndex && point.value > 0 }"
        :cx="point.x"
        :cy="point.y"
        :r="index === peakIndex && point.value > 0 ? 4 : values.length > 31 ? 1.6 : 2.6"
      />
    </template>
    <template v-if="hoverX !== null">
      <line
        class="daily-chart__guide"
        :x1="hoverX"
        :x2="hoverX"
        :y1="box.pad"
        :y2="box.height - box.pad"
      />
      <circle
        v-for="point in hoverPoints"
        :key="point.key"
        class="daily-chart__hover-dot"
        :style="point.color ? { '--metric-color': point.color } : undefined"
        :cx="point.x"
        :cy="point.y"
        r="3.5"
      />
    </template>
    <text
      v-for="tick in ticks"
      :key="tick.day"
      class="daily-chart__tick"
      :x="tick.x"
      :y="box.height - 4"
      :text-anchor="tick.anchor"
    >{{ formatDayLabel(tick.day, locale) }}</text>
    </svg>
    <ChartHoverTooltip
      v-if="hoverIndex !== null"
      :style="tooltipStyle"
      :title="hoverDateLabel"
      :rows="hoverRows"
      :compare-label="t('analytics.comparePrevious')"
    />
  </div>
</template>

<style scoped>
.daily-chart {
  position: relative;
}

.daily-chart__svg {
  display: block;
  width: 100%;
  height: auto;
  cursor: crosshair;
}

.daily-chart__grid {
  stroke: var(--color-chart-grid);
  stroke-width: 1;
}

.daily-chart__tick {
  fill: var(--color-text-subtle);
  font-family: var(--font-family);
  font-size: 11px;
}

.daily-chart__previous {
  fill: none;
  stroke: var(--color-text-subtle);
  stroke-width: 1.5;
  stroke-dasharray: 4 4;
  opacity: .55;
}

.daily-chart__line {
  fill: none;
  stroke: var(--metric-color);
  stroke-width: 2;
  stroke-linejoin: round;
  stroke-linecap: round;
}

.daily-chart__dot {
  fill: var(--color-bg-panel);
  stroke: var(--metric-color);
  stroke-width: 1.5;
}

.daily-chart__dot--peak {
  fill: var(--metric-color);
  stroke: var(--color-bg-panel);
}

.daily-chart__guide {
  stroke: var(--color-text-subtle);
  stroke-width: 1;
  stroke-dasharray: 3 3;
  opacity: .6;
  pointer-events: none;
}

.daily-chart__hover-dot {
  fill: var(--color-bg-panel);
  stroke: var(--metric-color, var(--color-accent));
  stroke-width: 2;
  pointer-events: none;
}

</style>

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
  type AnalyticsMetricSelection,
  type MetricSeries,
} from '../lib/analytics-metrics.ts';

/**
 * Activity per slot (hour of the day, weekday): bars for one metric, or one line per metric for "all", with a hover
 * guide and tooltip. The hourly and weekday charts only choose the slots, their labels and the proportions.
 */
const props = defineProps<{
  /** One value per slot and metric, already in display order. */
  series: MetricSeries;
  metric: AnalyticsMetricSelection;
  slotCount: number;
  box: { width: number; height: number; pad: number };
  /** Widest a bar may be, and the share of its slot it fills. */
  barMaxWidth: number;
  barRatio: number;
  /** Labels under the axis; a chart may label only some slots. */
  ticks: readonly { slot: number; label: string; anchor: 'start' | 'middle' | 'end' }[];
  slotTitle: (slot: number) => string;
}>();

const gridSteps = [0, 0.5, 1];
type SingleMetric = Exclude<AnalyticsMetricSelection, 'all'>;

const { t, locale } = useI18n();
const isAll = computed(() => props.metric === 'all');
const clean = (input: readonly (number | undefined)[]): number[] => input.map((value) => Math.max(0, value ?? 0));
const values = computed(() => (isAll.value ? [] : clean(props.series[props.metric as SingleMetric])));
// 'all' overlays every metric as lines: stacking bars would pretend the units are additive when they are not
// (diamonds are a value, peak a ceiling).
const overlaid = computed(() => (isAll.value ? analyticsMetrics.map((metric) => ({ metric, values: clean(props.series[metric]) })) : []));
const max = computed(() =>
  isAll.value
    ? overlaid.value.reduce((top, series) => Math.max(top, seriesMax(series.values)), 0)
    : values.value.reduce((top, value) => Math.max(top, value), 0),
);
const overlaidGeometry = computed(() =>
  overlaid.value.map((series) => ({ metric: series.metric, geometry: buildLineGeometry(series.values, props.box, max.value) })),
);
const label = computed(() => t(`analytics.metrics.${props.metric}`));

// Hover inspection: indices are display-order positions, matching the series and the tick labels exactly.
const slotCount = computed(() => props.slotCount);
const { hoverIndex, onPointermove, onPointerleave, slotX } = useChartHover(props.box, slotCount);
const hoverTitle = computed(() => (hoverIndex.value === null ? '' : props.slotTitle(hoverIndex.value)));
const hoverRows = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return [];
  const metrics = isAll.value ? analyticsMetrics : [props.metric as SingleMetric];
  return metrics.map((metric) => ({
    key: metric,
    color: metricColor[metric],
    label: t(`analytics.metrics.${metric}`),
    value: formatCount(props.series[metric][index] ?? 0, locale.value),
    before: null,
  }));
});
const tooltipStyle = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return {};
  const ratio = slotX(index) / props.box.width;
  const translate = ratio < 0.15 ? '0' : ratio > 0.85 ? '-100%' : '-50%';
  return { left: `${ratio * 100}%`, transform: `translateX(${translate})` };
});
const hoverX = computed(() => (hoverIndex.value === null ? null : slotX(hoverIndex.value)));
const slot = computed(() => (props.box.width - props.box.pad * 2) / props.slotCount);
const barWidth = computed(() => Math.max(2, Math.min(props.barMaxWidth, slot.value * props.barRatio)));
const innerHeight = computed(() => props.box.height - props.box.pad * 2);
const bars = computed(() =>
  values.value.map((value, index) => ({
    index,
    x: props.box.pad + slot.value * index + (slot.value - barWidth.value) / 2,
    height: value > 0 && max.value > 0 ? Math.max(2, (innerHeight.value * value) / max.value) : 0,
  })),
);
</script>

<template>
  <div class="slot-chart">
    <div class="slot-chart__plot">
      <svg
        class="slot-chart__svg"
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
            class="slot-chart__grid"
            :x1="box.pad"
            :x2="box.width - box.pad"
            :y1="box.pad + innerHeight * (1 - step)"
            :y2="box.pad + innerHeight * (1 - step)"
          />
          <text
            class="slot-chart__tick"
            :x="box.width - box.pad"
            :y="box.pad + innerHeight * (1 - step) - 5"
            text-anchor="end"
          >{{ formatCount(max * step, locale) }}</text>
        </g>
        <template v-if="isAll">
          <path
            v-for="series in overlaidGeometry"
            :key="series.metric"
            class="slot-chart__line"
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
            :width="barWidth"
            :height="bar.height"
            rx="2"
            :fill="metricColor[metric as SingleMetric]"
            :opacity="hoverIndex === null || hoverIndex === bar.index ? 0.9 : 0.35"
          />
        </template>
        <line
          v-if="hoverX !== null"
          class="slot-chart__guide"
          :x1="hoverX"
          :x2="hoverX"
          :y1="box.pad"
          :y2="box.height - box.pad"
        />
        <text
          v-for="tick in ticks"
          :key="tick.slot"
          class="slot-chart__tick"
          :x="box.pad + slot * tick.slot + slot / 2"
          :y="box.height - 4"
          :text-anchor="tick.anchor"
        >{{ tick.label }}</text>
      </svg>
      <ChartHoverTooltip
        v-if="hoverIndex !== null"
        :style="tooltipStyle"
        :title="hoverTitle"
        :rows="hoverRows"
      />
    </div>
    <slot
      name="footer"
      :max="max"
    />
  </div>
</template>

<style scoped>
.slot-chart {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.slot-chart__plot {
  position: relative;
}

.slot-chart__svg {
  display: block;
  width: 100%;
  height: auto;
  cursor: crosshair;
}

.slot-chart__guide {
  stroke: var(--color-text-subtle);
  stroke-width: 1;
  stroke-dasharray: 3 3;
  opacity: .6;
  pointer-events: none;
}

.slot-chart__grid {
  stroke: var(--color-chart-grid);
  stroke-width: 1;
}

.slot-chart__tick {
  fill: var(--color-text-subtle);
  font-family: var(--font-family);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.slot-chart__line {
  fill: none;
  stroke: var(--metric-color);
  stroke-width: 1.8;
  stroke-linejoin: round;
  stroke-linecap: round;
}
</style>

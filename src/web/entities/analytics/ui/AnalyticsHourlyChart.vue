<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatCount } from '@/shared/lib/intl.ts';
import { buildLineGeometry, seriesMax } from '../lib/chart-geometry.ts';
import { useChartHover } from '../lib/use-chart-hover.ts';
import ChartHoverTooltip from './ChartHoverTooltip.vue';
import {
  analyticsMetrics,
  formatHourLabel,
  HOURS_PER_DAY,
  metricColor,
  type AnalyticsMetricSelection,
  type MetricSeries,
} from '../lib/analytics-metrics.ts';

const props = defineProps<{
  series: MetricSeries;
  metric: AnalyticsMetricSelection;
}>();

const box = { width: 720, height: 220, pad: 20 };
const gridSteps = [0, 0.5, 1];
const tickHours = [0, 6, 12, 18, 23];

const { t, locale } = useI18n();
const isAll = computed(() => props.metric === 'all');
const values = computed(() =>
  isAll.value ? [] : props.series[props.metric as Exclude<AnalyticsMetricSelection, 'all'>].map((value) => Math.max(0, value ?? 0)),
);
// 'all' overlays every metric as lines — stacking bars would pretend the
// units are additive when they are not (diamonds are a value, peak a ceiling).
const overlaid = computed(() =>
  isAll.value
    ? analyticsMetrics.map((metric) => ({
        metric,
        values: props.series[metric].map((value) => Math.max(0, value ?? 0)),
      }))
    : [],
);
const max = computed(() =>
  isAll.value
    ? overlaid.value.reduce((top, series) => Math.max(top, seriesMax(series.values)), 0)
    : values.value.reduce((top, value) => Math.max(top, value), 0),
);
const overlaidGeometry = computed(() =>
  overlaid.value.map((series) => ({ metric: series.metric, geometry: buildLineGeometry(series.values, box, max.value) })),
);
const label = computed(() => t(`analytics.metrics.${props.metric}`));

// ── Hover inspection ─────────────────────────────────────────────────────
const slotCount = computed(() => HOURS_PER_DAY);
const { hoverIndex, onPointermove, onPointerleave, slotX } = useChartHover(box, slotCount);
const hoverTitle = computed(() => (hoverIndex.value === null ? '' : formatHourLabel(hoverIndex.value)));
const hoverRows = computed(() => {
  const index = hoverIndex.value;
  if (index === null) return [];
  const metrics = isAll.value ? analyticsMetrics : [props.metric as Exclude<AnalyticsMetricSelection, 'all'>];
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
  const ratio = slotX(index) / box.width;
  const translate = ratio < 0.15 ? '0' : ratio > 0.85 ? '-100%' : '-50%';
  return { left: `${ratio * 100}%`, transform: `translateX(${translate})` };
});
const hoverX = computed(() => (hoverIndex.value === null ? null : slotX(hoverIndex.value)));
const slot = (box.width - box.pad * 2) / HOURS_PER_DAY;
const width = Math.max(2, Math.min(18, slot * 0.62));
const innerHeight = box.height - box.pad * 2;
const bars = computed(() =>
  values.value.map((value, hour) => ({
    hour,
    x: box.pad + slot * hour + (slot - width) / 2,
    height: value > 0 && max.value > 0 ? Math.max(2, (innerHeight * value) / max.value) : 0,
    value,
  })),
);
</script>

<template>
  <div class="hourly-chart">
    <div class="hourly-chart__plot">
      <svg
        class="hourly-chart__svg"
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
          class="hourly-chart__grid"
          :x1="box.pad"
          :x2="box.width - box.pad"
          :y1="box.pad + innerHeight * (1 - step)"
          :y2="box.pad + innerHeight * (1 - step)"
        />
        <text
          class="hourly-chart__tick"
          :x="box.width - box.pad"
          :y="box.pad + innerHeight * (1 - step) - 5"
          text-anchor="end"
        >{{ formatCount(max * step, locale) }}</text>
      </g>
      <template v-if="isAll">
        <path
          v-for="series in overlaidGeometry"
          :key="series.metric"
          class="hourly-chart__line"
          :style="{ '--metric-color': metricColor[series.metric] }"
          :d="series.geometry.line"
        />
      </template>
      <template v-else>
        <rect
          v-for="bar in bars"
          :key="bar.hour"
          :x="bar.x"
          :y="box.height - box.pad - bar.height"
          :width="width"
          :height="bar.height"
          rx="2"
          :fill="metricColor[metric as Exclude<AnalyticsMetricSelection, 'all'>]"
          :opacity="hoverIndex === null || hoverIndex === bar.hour ? 0.9 : 0.35"
        />
      </template>
      <line
        v-if="hoverX !== null"
        class="hourly-chart__guide"
        :x1="hoverX"
        :x2="hoverX"
        :y1="box.pad"
        :y2="box.height - box.pad"
      />
      <text
        v-for="hour in tickHours"
        :key="hour"
        class="hourly-chart__tick"
        :x="box.pad + slot * hour + slot / 2"
        :y="box.height - 4"
        :text-anchor="hour === 0 ? 'start' : hour === 23 ? 'end' : 'middle'"
      >{{ formatHourLabel(hour) }}</text>
      </svg>
      <ChartHoverTooltip
        v-if="hoverIndex !== null"
        :style="tooltipStyle"
        :title="hoverTitle"
        :rows="hoverRows"
      />
    </div>
    <p
      v-if="max === 0"
      class="hourly-chart__empty"
    >
      {{ t('analytics.noHourlyData') }}
    </p>
  </div>
</template>

<style scoped>
.hourly-chart {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.hourly-chart__plot {
  position: relative;
}

.hourly-chart__svg {
  display: block;
  width: 100%;
  height: auto;
  cursor: crosshair;
}

.hourly-chart__guide {
  stroke: var(--color-text-subtle);
  stroke-width: 1;
  stroke-dasharray: 3 3;
  opacity: .6;
  pointer-events: none;
}

.hourly-chart__grid {
  stroke: var(--color-chart-grid);
  stroke-width: 1;
}

.hourly-chart__tick {
  fill: var(--color-text-subtle);
  font-family: var(--font-family);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.hourly-chart__line {
  fill: none;
  stroke: var(--metric-color);
  stroke-width: 1.8;
  stroke-linejoin: round;
  stroke-linecap: round;
}

.hourly-chart__empty {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  text-align: center;
}
</style>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsDayRow } from '../../../../shared/messages.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import { formatDayDate, formatDayLabel, type AnalyticsDaySpan } from '../lib/analytics-range.ts';
import { fillDaySeries, metricColor, type AnalyticsMetric } from '../lib/analytics-metrics.ts';
import { buildLineGeometry, seriesMax } from '../lib/chart-geometry.ts';

const props = defineProps<{
  days: readonly AnalyticsDayRow[];
  span: AnalyticsDaySpan;
  metric: AnalyticsMetric;
  /** Same-length period immediately before, drawn as a dashed reference line. */
  previousDays?: readonly AnalyticsDayRow[];
}>();

const box = { width: 720, height: 220, pad: 20 };
const gridSteps = [0, 0.5, 1];

const { t, locale } = useI18n();
const values = computed(() => fillDaySeries(props.days, props.span.startDay, props.span.endDay, props.metric));
const previousValues = computed(() => {
  if (!props.previousDays) return null;
  const length = props.span.endDay - props.span.startDay + 1;
  return fillDaySeries(props.previousDays, props.span.startDay - length, props.span.endDay - length, props.metric);
});
// The scale must cover both series or the comparison line lies about proportion.
const scaleMax = computed(() => Math.max(seriesMax(values.value), previousValues.value ? seriesMax(previousValues.value) : 0));
const geometry = computed(() => buildLineGeometry(values.value, box, scaleMax.value));
const previousGeometry = computed(() => (previousValues.value ? buildLineGeometry(previousValues.value, box, scaleMax.value) : null));
const label = computed(() => t(`analytics.metrics.${props.metric}`));
const showDots = computed(() => values.value.length <= 62);
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
  <svg
    class="daily-chart"
    :viewBox="`0 0 ${box.width} ${box.height}`"
    role="img"
    :aria-label="`${label}: ${formatCount(geometry.max, locale)}`"
    :style="{ '--metric-color': metricColor[metric] }"
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
    <path
      v-if="geometry.line"
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
      >
        <title>{{ formatDayDate(span.startDay + index, locale) }} — {{ label }}: {{ formatCount(point.value, locale) }}</title>
      </circle>
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
</template>

<style scoped>
.daily-chart {
  display: block;
  width: 100%;
  height: auto;
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
</style>

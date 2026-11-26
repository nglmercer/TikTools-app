<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatCount } from '@/shared/lib/intl.ts';
import {
  formatHourLabel,
  HOURS_PER_DAY,
  metricColor,
  type AnalyticsMetric,
  type MetricSeries,
} from '../lib/analytics-metrics.ts';

const props = defineProps<{
  series: MetricSeries;
  metric: AnalyticsMetric;
}>();

const box = { width: 720, height: 220, pad: 20 };
const gridSteps = [0, 0.5, 1];
const tickHours = [0, 6, 12, 18, 23];

const { t, locale } = useI18n();
const values = computed(() => props.series[props.metric].map((value) => Math.max(0, value ?? 0)));
const max = computed(() => values.value.reduce((top, value) => Math.max(top, value), 0));
const label = computed(() => t(`analytics.metrics.${props.metric}`));
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
    <svg
      class="hourly-chart__svg"
      :viewBox="`0 0 ${box.width} ${box.height}`"
      role="img"
      :aria-label="`${label}: ${formatCount(max, locale)}`"
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
      <rect
        v-for="bar in bars"
        :key="bar.hour"
        :x="bar.x"
        :y="box.height - box.pad - bar.height"
        :width="width"
        :height="bar.height"
        rx="2"
        :fill="metricColor[metric]"
        opacity="0.9"
      >
        <title>{{ formatHourLabel(bar.hour) }} — {{ label }}: {{ formatCount(bar.value, locale) }}</title>
      </rect>
      <text
        v-for="hour in tickHours"
        :key="hour"
        class="hourly-chart__tick"
        :x="box.pad + slot * hour + slot / 2"
        :y="box.height - 4"
        :text-anchor="hour === 0 ? 'start' : hour === 23 ? 'end' : 'middle'"
      >{{ formatHourLabel(hour) }}</text>
    </svg>
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

.hourly-chart__svg {
  display: block;
  width: 100%;
  height: auto;
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

.hourly-chart__empty {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  text-align: center;
}
</style>

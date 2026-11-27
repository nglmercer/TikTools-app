<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import {
  formatHourLabel,
  HOURS_PER_DAY,
  type AnalyticsMetricSelection,
  type MetricSeries,
} from '../lib/analytics-metrics.ts';
import AnalyticsSlotChart from './AnalyticsSlotChart.vue';

defineProps<{
  series: MetricSeries;
  metric: AnalyticsMetricSelection;
}>();

const box = { width: 720, height: 220, pad: 20 };
const anchorFor = (hour: number) => (hour === 0 ? 'start' : hour === HOURS_PER_DAY - 1 ? 'end' : 'middle');
const ticks = [0, 6, 12, 18, HOURS_PER_DAY - 1].map((hour) => ({ slot: hour, label: formatHourLabel(hour), anchor: anchorFor(hour) }) as const);
const { t } = useI18n();
</script>

<template>
  <AnalyticsSlotChart
    :series="series"
    :metric="metric"
    :slot-count="HOURS_PER_DAY"
    :box="box"
    :bar-max-width="18"
    :bar-ratio="0.62"
    :ticks="ticks"
    :slot-title="formatHourLabel"
  >
    <template #footer="{ max }">
      <p
        v-if="max === 0"
        class="hourly-chart__empty"
      >
        {{ t('analytics.noHourlyData') }}
      </p>
    </template>
  </AnalyticsSlotChart>
</template>

<style scoped>
.hourly-chart__empty {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  text-align: center;
}
</style>

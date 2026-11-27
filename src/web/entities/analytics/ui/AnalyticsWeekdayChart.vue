<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  WEEKDAYS_PER_WEEK,
  analyticsMetrics,
  type AnalyticsMetricSelection,
  type MetricSeries,
} from '../lib/analytics-metrics.ts';
import AnalyticsSlotChart from './AnalyticsSlotChart.vue';

/** Average-activity-per-weekday bars (single metric) or overlaid lines (all). */
const props = defineProps<{
  series: MetricSeries;
  metric: AnalyticsMetricSelection;
}>();

const box = { width: 720, height: 180, pad: 20 };
const { locale } = useI18n();

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
const ticks = computed(() => weekdayLabels.value.map((label, slot) => ({ slot, label, anchor: 'middle' }) as const));
const slotTitle = (slot: number): string => weekdayLabels.value[slot] ?? '';

// Every metric reordered once, so the shared chart can treat the weekdays like any other run of slots.
const ordered = computed<MetricSeries>(() => {
  const reorder = (input: readonly number[]): number[] => order.value.map((weekday) => Math.max(0, input[weekday] ?? 0));
  return Object.fromEntries(analyticsMetrics.map((metric) => [metric, reorder(props.series[metric])])) as MetricSeries;
});
</script>

<template>
  <AnalyticsSlotChart
    :series="ordered"
    :metric="metric"
    :slot-count="WEEKDAYS_PER_WEEK"
    :box="box"
    :bar-max-width="40"
    :bar-ratio="0.5"
    :ticks="ticks"
    :slot-title="slotTitle"
  />
</template>

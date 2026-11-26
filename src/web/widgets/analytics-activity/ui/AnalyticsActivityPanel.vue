<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsSummaryData } from '../../../../shared/messages.ts';
import { analyticsMetrics, hourRowsToSeries, type AnalyticsMetric } from '@/entities/analytics/lib/analytics-metrics.ts';
import {
  formatDayDate,
  formatSpan,
  formatZoneShort,
  isSingleDaySpan,
} from '@/entities/analytics/lib/analytics-range.ts';
import AnalyticsDailyChart from '@/entities/analytics/ui/AnalyticsDailyChart.vue';
import AnalyticsHourlyChart from '@/entities/analytics/ui/AnalyticsHourlyChart.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';

const props = defineProps<{
  summary: AnalyticsSummaryData;
  previous: AnalyticsSummaryData | null;
}>();

/** The charted metric is picked here, in the panel it affects — not by the KPI cards. */
const metric = defineModel<AnalyticsMetric>('metric', { required: true });
const { t, locale } = useI18n();
const span = computed(() => ({ startDay: props.summary.startDay, endDay: props.summary.endDay }));
const singleDay = computed(() => isSingleDaySpan(span.value));
const hourly = computed(() => hourRowsToSeries(props.summary.hours));
const hasHourly = computed(() =>
  props.summary.hours.some((row) =>
    Object.entries(row).some(([key, value]) => key !== 'hour' && typeof value === 'number' && value > 0),
  ),
);
const subtitle = computed(() => {
  if (!singleDay.value) return formatSpan(span.value, locale.value);
  const zone = formatZoneShort(locale.value);
  return [formatDayDate(span.value.startDay, locale.value), zone].filter(Boolean).join(' · ');
});
const metricOptions = computed(() => analyticsMetrics.map((value) => ({ value, label: t(`analytics.metrics.${value}`) })));
</script>

<template>
  <BasePanel
    :title="singleDay ? t('analytics.activity.today') : t('analytics.activity.title')"
    :subtitle="subtitle"
  >
    <template #actions>
      <BaseSegmented
        v-model="metric"
        :options="metricOptions"
        size="sm"
        :aria-label="t('analytics.metricPicker')"
      />
    </template>
    <AnalyticsHourlyChart
      v-if="singleDay"
      :series="hourly"
      :metric="metric"
    />
    <AnalyticsDailyChart
      v-else
      :days="summary.days"
      :span="span"
      :metric="metric"
      :previous-days="previous?.days"
    />
    <section
      v-if="!singleDay && hasHourly"
      class="activity-panel__pattern"
    >
      <h3 class="activity-panel__pattern-title">
        {{ t('analytics.activity.pattern') }}
      </h3>
      <AnalyticsHourlyChart
        :series="hourly"
        :metric="metric"
      />
      <p class="activity-panel__note">
        {{ t('analytics.activity.patternHint') }}
      </p>
    </section>
    <p
      v-if="singleDay && !summary.hours.length"
      class="activity-panel__note"
    >
      {{ t('analytics.activity.noHourly') }}
    </p>
  </BasePanel>
</template>

<style scoped>
.activity-panel__pattern {
  margin-top: var(--space-4);
  padding-top: var(--space-3);
  border-top: 1px solid var(--color-border);
}

.activity-panel__pattern-title {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-subtle);
}

.activity-panel__note {
  margin: var(--space-3) 0 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}
</style>

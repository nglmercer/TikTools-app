<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsSummaryData } from '../../../../shared/messages.ts';
import { analyticsMetrics, hourRowsToSeries, metricColor, type AnalyticsMetric, type AnalyticsMetricSelection } from '@/entities/analytics/lib/analytics-metrics.ts';
import { formatZoneShort, isSingleDaySpan } from '@/entities/analytics/lib/analytics-range.ts';
import AnalyticsDailyChart from '@/entities/analytics/ui/AnalyticsDailyChart.vue';
import AnalyticsHourlyChart from '@/entities/analytics/ui/AnalyticsHourlyChart.vue';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import BaseDateText from '@/shared/ui/date-text/BaseDateText.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconHeart from '@/shared/ui/icon/icons/IconHeart.vue';
import IconDiamond from '@/shared/ui/icon/icons/IconDiamond.vue';
import IconEye from '@/shared/ui/icon/icons/IconEye.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import BaseIconSelect from '@/shared/ui/menu/BaseIconSelect.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';

const props = defineProps<{
  summary: AnalyticsSummaryData;
  previous: AnalyticsSummaryData | null;
}>();

/** The charted metric is picked here, in the panel it affects — not by the KPI cards. */
const metric = defineModel<AnalyticsMetricSelection>('metric', { required: true });
const { t, locale } = useI18n();
// Narrow viewports trade the segmented row for a native select — five
// Spanish labels never fit a phone screen, and the select collapses them
// into the one control mobile users already know.
const narrowPicker = useMediaQuery('(width < 720px)');
const span = computed(() => ({ startDay: props.summary.startDay, endDay: props.summary.endDay }));
const singleDay = computed(() => isSingleDaySpan(span.value));
const hourly = computed(() => hourRowsToSeries(props.summary.hours));
const hasHourly = computed(() =>
  props.summary.hours.some((row) =>
    Object.entries(row).some(([key, value]) => key !== 'hour' && typeof value === 'number' && value > 0),
  ),
);
const zoneSuffix = computed(() => formatZoneShort(locale.value));
// Every option carries its metric icon tinted in the chart color, so the
// picker itself is the legend — no separate swatch row repeating the labels.
const metricIcons: Record<AnalyticsMetric, typeof IconChat> = {
  chats: IconChat,
  gifts: IconCoins,
  likes: IconHeart,
  diamonds: IconDiamond,
  peakViewers: IconEye,
};
const metricOptions = computed(() => [
  { value: 'all' as const, label: t('analytics.metrics.all'), icon: IconLayers },
  ...analyticsMetrics.map((value) => ({
    value,
    label: t(`analytics.metrics.${value}`),
    icon: metricIcons[value],
    color: metricColor[value],
  })),
]);
</script>

<template>
  <BasePanel
    :title="singleDay ? t('analytics.activity.today') : t('analytics.activity.title')"
  >
    <template #subtitle>
      <BaseDateText
        :value="summary.startDay * 86400"
        :end-value="singleDay ? undefined : summary.endDay * 86400"
        utc
      />
      <template v-if="singleDay && zoneSuffix"> · {{ zoneSuffix }}</template>
    </template>
    <template #actions>
      <BaseIconSelect
        v-if="narrowPicker"
        :model-value="metric"
        :options="metricOptions"
        :label="t('analytics.metricPicker')"
        placement="bottom-start"
        @update:model-value="metric = $event as AnalyticsMetricSelection"
      />
      <BaseSegmented
        v-else
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

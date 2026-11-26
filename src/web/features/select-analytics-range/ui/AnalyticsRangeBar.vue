<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { analyticsRangeKeys, type AnalyticsRangeKey } from '@/entities/analytics/lib/analytics-range.ts';
import BaseDateRange from '@/shared/ui/date-range/BaseDateRange.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';

const range = defineModel<AnalyticsRangeKey>('range', { required: true });
const start = defineModel<string>('start', { required: true });
const end = defineModel<string>('end', { required: true });

const { t } = useI18n();
const options = computed(() => analyticsRangeKeys.map((key) => ({ value: key, label: t(`analytics.ranges.${key}`) })));
</script>

<template>
  <div class="range-bar">
    <BaseSegmented
      v-model="range"
      :options="options"
      :aria-label="t('analytics.rangeLabel')"
    />
    <BaseDateRange
      v-if="range === 'custom'"
      v-model:start="start"
      v-model:end="end"
      class="range-bar__custom"
      :from-label="t('analytics.ranges.from')"
      :to-label="t('analytics.ranges.to')"
    />
  </div>
</template>

<style scoped>
.range-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-5);
}

</style>

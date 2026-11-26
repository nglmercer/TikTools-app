<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { isAnalyticsMetric, type AnalyticsMetric } from '@/entities/analytics/lib/analytics-metrics.ts';
import {
  analyticsRangeKeys,
  dayToIsoDate,
  formatDayDate,
  formatSpan,
  isSingleDaySpan,
  isoDateToDay,
  resolveAnalyticsRange,
  systemDay,
  type AnalyticsRangeKey,
} from '@/entities/analytics/lib/analytics-range.ts';
import { useAnalyticsStore } from '@/entities/analytics/model/analytics-store.ts';
import AnalyticsContributorsTable from '@/entities/analytics/ui/AnalyticsContributorsTable.vue';
import AnalyticsInsights from '@/entities/analytics/ui/AnalyticsInsights.vue';
import AnalyticsMetricStrip from '@/entities/analytics/ui/AnalyticsMetricStrip.vue';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import AnalyticsRangeBar from '@/features/select-analytics-range/ui/AnalyticsRangeBar.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import type { TableSort } from '@/shared/ui/table/table-types.ts';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import AnalyticsActivityPanel from '@/widgets/analytics-activity/ui/AnalyticsActivityPanel.vue';

const { t, locale } = useI18n();
const connection = useConnectionStore();
const store = useAnalyticsStore();
const { summary, previous, loading, error } = storeToRefs(store);

const saved = loadViewState().analytics;
const today = systemDay(Date.now());
const range = ref<AnalyticsRangeKey>(analyticsRangeKeys.includes(saved?.range as AnalyticsRangeKey) ? saved!.range! : '7d');
const metric = ref<AnalyticsMetric>(isAnalyticsMetric(saved?.metric) ? saved.metric : 'chats');
const query = ref(saved?.contributorQuery ?? '');
const customStart = ref(saved?.customStart ?? dayToIsoDate(today - 6));
const customEnd = ref(saved?.customEnd ?? dayToIsoDate(today));
const sort = ref<TableSort | null>(null);

const creator = computed(() => connection.activeCreator);
const span = computed(() => {
  const now = systemDay(Date.now());
  return resolveAnalyticsRange(range.value, now, {
    startDay: isoDateToDay(customStart.value) ?? now,
    endDay: isoDateToDay(customEnd.value) ?? now,
  });
});

const hasData = computed(() => {
  const data = summary.value;
  if (!data) return false;
  return (
    data.days.length > 0 ||
    data.topViewers.length > 0 ||
    data.sessions > 0 ||
    Object.values(data.totals).some((value) => value > 0) ||
    data.hours.some((row) => Object.entries(row).some(([key, value]) => key !== 'hour' && typeof value === 'number' && value > 0))
  );
});

const emptyDescription = computed(() => {
  const data = summary.value;
  if (data && isSingleDaySpan(data)) return t('analytics.empty.today', { date: formatDayDate(data.startDay, locale.value) });
  return t('analytics.empty.hint');
});

const spanLabel = computed(() =>
  summary.value ? formatSpan({ startDay: summary.value.startDay, endDay: summary.value.endDay }, locale.value) : '',
);

function refresh(): void {
  void store.load(creator.value, span.value);
}

watch([span, creator], refresh, { immediate: true });

const persist = debounce(() => {
  saveViewState('analytics', {
    range: range.value,
    metric: metric.value,
    contributorQuery: query.value,
    customStart: customStart.value,
    customEnd: customEnd.value,
  });
});
watch([range, metric, query, customStart, customEnd], persist);
</script>

<template>
  <div class="analytics-page" data-guide-scope="analytics">
    <header class="analytics-page__header">
      <div class="analytics-page__heading">
        <h1 class="analytics-page__title">
          {{ t('analytics.title') }}
        </h1>
        <p
          v-if="summary"
          class="analytics-page__subtitle"
        >
          {{ spanLabel }}
          <span aria-hidden="true">·</span>
          {{ t('analytics.facts.sessionsValue', { count: summary.sessions }) }}
        </p>
      </div>
      <div class="analytics-page__controls">
        <AnalyticsRangeBar
          data-guide="analytics-range"
          v-model:range="range"
          v-model:start="customStart"
          v-model:end="customEnd"
        />
        <span
          v-if="creator"
          class="analytics-page__creator"
        >@{{ creator.replace(/^@/, '') }}</span>
        <BaseIconButton
          :label="t('analytics.refresh')"
          :disabled="loading"
          @click="refresh"
        >
          <IconRefresh />
        </BaseIconButton>
      </div>
    </header>

    <BaseAlert
      v-if="error"
      tone="error"
      compact
      :title="error"
    />

    <BaseEmptyState
      v-if="!summary || !hasData"
      class="analytics-page__empty"
      :title="loading && !summary ? t('analytics.loading') : t('analytics.empty.title')"
      :description="loading && !summary ? undefined : emptyDescription"
    >
      <template #icon>
        <IconChart :size="32" />
      </template>
    </BaseEmptyState>

    <template v-else>
      <AnalyticsMetricStrip
        data-guide="analytics-metrics"
        :totals="summary.totals"
        :previous-totals="previous?.totals ?? null"
        :days="summary.days"
        :start-day="summary.startDay"
        :end-day="summary.endDay"
      />
      <div class="analytics-page__main">
        <AnalyticsActivityPanel
          data-guide="analytics-activity"
          class="analytics-page__activity"
          v-model:metric="metric"
          :summary="summary"
          :previous="previous"
        />
        <BasePanel data-guide="analytics-facts" :title="t('analytics.insights.title')">
          <AnalyticsInsights :summary="summary" />
        </BasePanel>
      </div>
      <BasePanel
        data-guide="analytics-contributors"
        class="analytics-page__contributors"
        :title="t('analytics.contributors.title')"
        flush
      >
        <template #actions>
          <BaseTextField
            v-model="query"
            class="analytics-page__search"
            type="search"
            size="sm"
            :placeholder="t('analytics.contributors.search')"
            :aria-label="t('analytics.contributors.search')"
          >
            <template #prefix>
              <IconSearch />
            </template>
          </BaseTextField>
        </template>
        <AnalyticsContributorsTable
          v-model:sort="sort"
          :viewers="summary.topViewers"
          :query="query"
        />
      </BasePanel>
    </template>
  </div>
</template>

<style scoped>
.analytics-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-5);
  min-width: 0;
  min-height: 0;
  padding: var(--space-6);
  overflow-y: auto;
}

.analytics-page__header {
  display: flex;
  flex: none;
  flex-wrap: wrap;
  align-items: flex-end;
  justify-content: space-between;
  gap: var(--space-4);
}

.analytics-page__heading {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.analytics-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: var(--font-weight-semibold);
  letter-spacing: -0.01em;
}

.analytics-page__subtitle {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
}

.analytics-page__controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
}

.analytics-page__creator {
  padding: 2px var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: 999px;
  background: var(--color-bg-panel);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  white-space: nowrap;
}

.analytics-page__empty {
  flex: 1;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

.analytics-page__main {
  display: grid;
  grid-template-columns: minmax(0, 2.1fr) minmax(280px, 1fr);
  gap: var(--space-5);
  align-items: start;
}

.analytics-page__activity {
  min-width: 0;
}

.analytics-page__contributors {
  flex: none;
  min-height: 240px;
}

.analytics-page__search {
  width: 220px;
  max-width: 100%;
}

@media (width < 1100px) {
  .analytics-page {
    padding: var(--space-4);
  }

  .analytics-page__main {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (width < 720px) {
  .analytics-page {
    gap: var(--space-4);
    padding: var(--space-4);
  }

  .analytics-page__controls {
    width: 100%;
  }

  .analytics-page__controls > :first-child {
    width: 100%;
    overflow-x: auto;
  }

  .analytics-page__search {
    width: 100%;
  }
}
</style>

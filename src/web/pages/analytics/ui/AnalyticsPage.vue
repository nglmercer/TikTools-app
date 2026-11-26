<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { isAnalyticsMetricSelection, type AnalyticsMetricSelection } from '@/entities/analytics/lib/analytics-metrics.ts';
import {
  analyticsRangeKeys,
  dayToIsoDate,
  formatDayDate,
  isSingleDaySpan,
  isoDateToDay,
  resolveAnalyticsRange,
  systemDay,
  type AnalyticsRangeKey,
} from '@/entities/analytics/lib/analytics-range.ts';
import { useAnalyticsStore } from '@/entities/analytics/model/analytics-store.ts';
import AnalyticsContributorsTable from '@/entities/analytics/ui/AnalyticsContributorsTable.vue';
import AnalyticsSessionsTable from '@/entities/analytics/ui/AnalyticsSessionsTable.vue';
import AnalyticsInsights from '@/entities/analytics/ui/AnalyticsInsights.vue';
import AnalyticsMetricStrip from '@/entities/analytics/ui/AnalyticsMetricStrip.vue';
import { formatCount } from '@/shared/lib/intl.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import AnalyticsRangeBar from '@/features/select-analytics-range/ui/AnalyticsRangeBar.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
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
const metric = ref<AnalyticsMetricSelection>(isAnalyticsMetricSelection(saved?.metric) ? saved.metric : 'chats');
const query = ref(saved?.contributorQuery ?? '');
const customStart = ref(saved?.customStart ?? dayToIsoDate(today - 6));
const customEnd = ref(saved?.customEnd ?? dayToIsoDate(today));
const sort = ref<TableSort | null>(null);
type AnalyticsTab = 'overview' | 'contributors' | 'sessions';
const savedTab = saved?.tab;
const tab = ref<AnalyticsTab>(savedTab === 'contributors' || savedTab === 'sessions' ? savedTab : 'overview');

const tabOptions = computed(() => [
  { value: 'overview' as const, label: t('analytics.tabs.overview') },
  { value: 'contributors' as const, label: t('analytics.tabs.contributors') },
  { value: 'sessions' as const, label: t('analytics.tabs.sessions') },
]);

const audienceLabel = computed(() => {
  const audience = summary.value?.audience;
  if (!audience || audience.activeViewers <= 0) return '';
  return t('analytics.contributors.audience', {
    active: formatCount(audience.activeViewers, locale.value),
    new: formatCount(audience.newViewers, locale.value),
    returning: formatCount(Math.max(0, audience.activeViewers - audience.newViewers), locale.value),
  });
});

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

function refresh(): void {
  void store.load(creator.value, span.value);
}

watch([span, creator], refresh, { immediate: true });

// The desktop projection deliberately drops analytics.updated events (they
// fire every few seconds during a stream), so the page resyncs itself on a
// slow cadence — only while visible, cheap enough for the local host and
// race-safe because the store keeps the newest request only.
const POLL_MS = 30_000;
let poll: ReturnType<typeof setInterval> | null = null;
onMounted(() => {
  poll = setInterval(() => {
    if (document.visibilityState === 'visible' && creator.value && !loading.value) refresh();
  }, POLL_MS);
});
onBeforeUnmount(() => {
  if (poll) clearInterval(poll);
});

const persist = debounce(() => {
  saveViewState('analytics', {
    range: range.value,
    metric: metric.value,
    contributorQuery: query.value,
    customStart: customStart.value,
    customEnd: customEnd.value,
    tab: tab.value,
  });
});
watch([range, metric, query, customStart, customEnd, tab], persist);
</script>

<template>
  <div class="analytics-page" data-guide-scope="analytics">
    <BaseAlert
      v-if="error"
      tone="error"
      compact
      :title="error"
    />

    <BaseTabs
      v-model="tab"
      class="analytics-page__tabs"
      :tabs="tabOptions"
      :label="t('analytics.title')"
      list-end
    >
      <template #bar>
        <div class="analytics-page__heading">
          <h1 class="analytics-page__title">
            {{ t('analytics.title') }}
          </h1>
          <BaseIconButton
            class="analytics-page__refresh"
            :label="t('analytics.refresh')"
            :disabled="loading"
            @click="refresh"
          >
            <IconRefresh />
          </BaseIconButton>
        </div>
        <div class="analytics-page__controls">
          <AnalyticsRangeBar
            data-guide="analytics-range"
            v-model:range="range"
            v-model:start="customStart"
            v-model:end="customEnd"
          />
        </div>
      </template>
      <template #default="{ value }">
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
        <div
          v-else-if="value === 'overview'"
          class="analytics-page__tab"
        >
          <div class="analytics-page__main">
            <AnalyticsActivityPanel
              data-guide="analytics-activity"
              class="analytics-page__activity"
              v-model:metric="metric"
              :summary="summary"
              :previous="previous"
            />
            <aside class="analytics-page__side">
              <BasePanel data-guide="analytics-facts" :title="t('analytics.insights.title')">
                <AnalyticsInsights :summary="summary" />
              </BasePanel>
              <AnalyticsMetricStrip
                data-guide="analytics-metrics"
                :totals="summary.totals"
                :previous-totals="previous?.totals ?? null"
                :days="summary.days"
                :start-day="summary.startDay"
                :end-day="summary.endDay"
              />
            </aside>
          </div>
        </div>
        <BasePanel
          v-else-if="value === 'contributors'"
          data-guide="analytics-contributors"
          class="analytics-page__contributors"
          :title="t('analytics.contributors.title')"
          flush
        >
          <template
            v-if="audienceLabel"
            #subtitle
          >
            {{ audienceLabel }}
          </template>
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
        <BasePanel
          v-else
          data-guide="analytics-sessions"
          :title="t('analytics.sessions.title')"
          flush
        >
          <AnalyticsSessionsTable
            v-if="summary.sessionList.length"
            :sessions="summary.sessionList"
          />
          <p
            v-else
            class="analytics-page__sessions-empty"
          >
            {{ t('analytics.sessions.empty') }}
          </p>
        </BasePanel>
      </template>
    </BaseTabs>
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

.analytics-page__tabs {
  flex: 1;
  min-height: 0;
}

.analytics-page__tab {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  padding-top: var(--space-5);
}

.analytics-page__sessions-empty {
  margin: 0;
  padding: var(--space-5);
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.analytics-page__heading {
  display: flex;
  align-items: center;
  align-self: center;
  gap: var(--space-2);
  min-width: 0;
}

.analytics-page__title {
  margin: 0;
  font-size: 18px;
  font-weight: var(--font-weight-semibold);
  letter-spacing: -0.01em;
}

.analytics-page__controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  align-self: center;
  gap: var(--space-4);
  margin-left: auto;
}

.analytics-page__empty {
  min-height: 320px;
}

.analytics-page__main {
  display: grid;
  grid-template-columns: minmax(0, 2.1fr) minmax(280px, 1fr);
  gap: var(--space-5);
  align-items: start;
  /* The tab panel stretches to the viewport height; without this the default
     align-content distributes the surplus into the auto rows themselves,
     opening dead gaps between the panels. */
  align-content: start;
}

.analytics-page__side {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  min-width: 0;
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

  .analytics-page__controls {
    flex-wrap: nowrap;
  }

  .analytics-page__controls > :first-child {
    flex: 1;
    min-width: 0;
    overflow-x: auto;
  }

  .analytics-page__search {
    width: 100%;
  }
}
</style>

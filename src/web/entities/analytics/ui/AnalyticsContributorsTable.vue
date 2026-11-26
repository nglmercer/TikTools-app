<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsTopViewer } from '../../../../shared/messages.ts';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import type { TableSort } from '@/shared/ui/table/table-types.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import { formatClockTime, formatDayDate, isoDateToDay, systemDay } from '../lib/analytics-range.ts';
import {
  filterContributors,
  rankContributors,
  sortContributors,
  type ContributorSortKey,
  type RankedContributor,
} from '../lib/contributors.ts';

const props = defineProps<{
  viewers: readonly AnalyticsTopViewer[];
  query: string;
}>();

const sort = defineModel<TableSort | null>('sort', { default: null });
const { t, locale } = useI18n();

const columns = computed(() => [
  { key: 'rank', label: t('analytics.contributors.rank'), width: '56px', sortable: true },
  { key: 'uniqueId', label: t('analytics.contributors.viewer'), sortable: true },
  { key: 'chats', label: t('analytics.metrics.chats'), width: '84px', align: 'end' as const, sortable: true },
  { key: 'gifts', label: t('analytics.metrics.gifts'), width: '84px', align: 'end' as const, sortable: true },
  { key: 'diamonds', label: t('analytics.metrics.diamonds'), width: '96px', align: 'end' as const, sortable: true },
  {
    key: 'interactions',
    label: t('analytics.contributors.interactions'),
    width: '110px',
    align: 'end' as const,
    sortable: true,
  },
  { key: 'share', label: t('analytics.contributorsShare'), width: '90px', align: 'end' as const, sortable: false },
  { key: 'lastSeen', label: t('analytics.contributors.lastActive'), width: '96px', align: 'end' as const, sortable: true },
]);

const rows = computed(() => {
  const filtered = filterContributors(rankContributors(props.viewers), props.query);
  const sorted = sort.value ? sortContributors(filtered, sort.value.key as ContributorSortKey, sort.value.direction) : filtered;
  const totalInteractions = props.viewers.reduce((sum, viewer) => sum + Math.max(0, viewer.interactions ?? 0), 0);
  return sorted.map((row) => ({
    ...row,
    share: totalInteractions > 0 ? Math.max(0, row.interactions) / totalInteractions : 0,
  }));
});

/** Clock time for same-day activity, date + time when the range spans days. */
function formatLastSeen(unixSecs: number): string {
  const day = isoDateToDay(new Date(unixSecs * 1000).toISOString().slice(0, 10));
  const today = systemDay(Date.now());
  const time = formatClockTime(unixSecs, locale.value);
  if (day !== null && day !== today) return `${formatDayDate(day, locale.value)} · ${time}`;
  return time;
}
</script>

<template>
  <BaseTable
    v-model:sort="sort"
    class="contributors-table"
    :columns="columns"
    :rows="rows"
    :row-key="(row: RankedContributor) => row.uniqueId"
  >
    <template #cell-rank="{ value }">
      <span class="contributors-table__rank">#{{ value }}</span>
    </template>
    <template #cell-uniqueId="{ row, value }">
      <span class="contributors-table__viewer">
        <img
          v-if="row.avatarUrl"
          class="contributors-table__avatar"
          :src="row.avatarUrl"
          :alt="''"
          loading="lazy"
        >
        <span
          v-else
          class="contributors-table__avatar contributors-table__avatar--fallback"
          aria-hidden="true"
        >{{ String(value).slice(0, 1).toUpperCase() }}</span>
        @{{ value }}
      </span>
    </template>
    <template #cell-chats="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-gifts="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-diamonds="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-interactions="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-share="{ row }">
      <span class="contributors-table__share">
        <span
          class="contributors-table__share-bar"
          :style="{ width: `${Math.round(row.share * 100)}%` }"
        />
        {{ Math.round(row.share * 100) }}%
      </span>
    </template>
    <template #cell-lastSeen="{ value }">
      {{ formatLastSeen(Number(value)) }}
    </template>
    <template #empty>
      {{ query ? t('analytics.contributors.noMatches') : t('analytics.contributors.empty') }}
    </template>
  </BaseTable>
</template>

<style scoped>
.contributors-table {
  overflow-x: auto;
}

.contributors-table__rank {
  color: var(--color-text-subtle);
  font-variant-numeric: tabular-nums;
}

.contributors-table__viewer {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.contributors-table__avatar {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  object-fit: cover;
  flex: none;
}

.contributors-table__avatar--fallback {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg-control);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
}

.contributors-table__share {
  position: relative;
  display: inline-block;
  min-width: 44px;
  font-variant-numeric: tabular-nums;
}

.contributors-table__share-bar {
  position: absolute;
  left: 0;
  bottom: -2px;
  height: 2px;
  border-radius: 1px;
  background: var(--color-chart-1);
  opacity: .8;
}

@media (width < 720px) {
  .contributors-table :deep(td:nth-child(3)),
  .contributors-table :deep(th:nth-child(3)),
  .contributors-table :deep(td:nth-child(4)),
  .contributors-table :deep(th:nth-child(4)),
  .contributors-table :deep(td:nth-child(5)),
  .contributors-table :deep(th:nth-child(5)) {
    display: none;
  }
}
</style>

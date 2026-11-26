<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsSessionRow } from '../../../../shared/messages.ts';
import BaseDateText from '@/shared/ui/date-text/BaseDateText.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import { formatCount } from '@/shared/lib/intl.ts';

const props = defineProps<{
  sessions: readonly AnalyticsSessionRow[];
}>();

const { t, locale } = useI18n();

const columns = computed(() => [
  { key: 'startedAt', label: t('analytics.sessions.started'), sortable: false },
  { key: 'duration', label: t('analytics.sessions.duration'), width: '110px', align: 'end' as const, sortable: false },
  { key: 'peakViewers', label: t('analytics.facts.peakViewers'), width: '110px', align: 'end' as const, sortable: false },
  { key: 'chats', label: t('analytics.metrics.chats'), width: '84px', align: 'end' as const, sortable: false },
  { key: 'likes', label: t('analytics.metrics.likes'), width: '84px', align: 'end' as const, sortable: false },
  { key: 'gifts', label: t('analytics.metrics.gifts'), width: '84px', align: 'end' as const, sortable: false },
]);

function formatDuration(startedAt: number, endedAt: number | null): string {
  const end = endedAt ?? Math.floor(Date.now() / 1_000);
  const secs = Math.max(0, end - startedAt);
  const hours = Math.floor(secs / 3_600);
  const minutes = Math.floor((secs % 3_600) / 60);
  if (hours > 0) return t('analytics.sessions.durationHm', { h: hours, m: minutes });
  return t('analytics.sessions.durationM', { m: Math.max(1, minutes) });
}

const rows = computed(() =>
  props.sessions.map((session, index) => ({
    ...session,
    rowId: `${session.startedAt}-${index}`,
    duration: formatDuration(session.startedAt, session.endedAt),
    live: session.endedAt === null,
  })),
);
</script>

<template>
  <BaseTable
    class="sessions-table"
    :columns="columns"
    :rows="rows"
    :row-key="(row: { rowId: string }) => row.rowId"
  >
    <template #cell-startedAt="{ row }">
      <span class="sessions-table__start">
        <BaseDateText
          :value="Number(row.startedAt)"
          mode="datetime"
        />
        <span
          v-if="row.live"
          class="sessions-table__live"
        >{{ t('analytics.sessions.live') }}</span>
      </span>
    </template>
    <template #cell-peakViewers="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-chats="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-likes="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
    <template #cell-gifts="{ value }">
      {{ formatCount(Number(value), locale) }}
    </template>
  </BaseTable>
</template>

<style scoped>
.sessions-table__start {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.sessions-table__live {
  padding: 0 var(--space-2);
  border-radius: var(--radius-full, 999px);
  background: var(--color-bg-success, rgb(34 197 94 / 14%));
  color: var(--color-text-success);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium, 500);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
</style>

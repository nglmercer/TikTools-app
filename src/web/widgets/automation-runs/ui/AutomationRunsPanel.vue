<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BehaviorRun } from '../../../../automation/behavior/types.ts';
import { formatClockTime } from '@/entities/analytics/lib/analytics-range.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';

const RUN_LIMIT = 50;

const props = defineProps<{
  runs: readonly BehaviorRun[];
}>();

const { t, locale } = useI18n();
// The log is always at hand but can be folded away to give the workspace the full height.
const open = ref(true);

// Phones keep the columns that identify a run; the result text needs room.
const isPhone = useMediaQuery('(width < 600px)');
const columns = computed<TableColumn[]>(() => [
  { key: 'status', label: t('automations.runs.columns.status'), width: '90px' },
  { key: 'time', label: t('automations.runs.columns.time'), width: '60px' },
  { key: 'event', label: t('automations.runs.columns.event') },
  { key: 'action', label: t('automations.runs.columns.action') },
  ...(isPhone.value ? [] : [{ key: 'result', label: t('automations.runs.columns.result') }]),
]);

const rows = computed(() =>
  props.runs.slice(0, RUN_LIMIT).map((run) => ({
    id: run.id,
    status: run.status,
    test: run.test,
    time: formatClockTime(run.at / 1000, locale.value),
    event: run.eventName ?? '—',
    action: run.actionName,
    result: run.error ?? run.summary,
  })),
);
</script>

<template>
  <BasePanel
    class="automation-runs"
    :title="t('automations.runs.title')"
    flush
  >
    <template #leading>
      <BaseIconButton
        :label="open ? t('automations.runs.collapse') : t('automations.runs.expand')"
        size="sm"
        :aria-expanded="open"
        @click="open = !open"
      >
        <IconChevronDown v-if="open" />
        <IconChevronRight v-else />
      </BaseIconButton>
    </template>
    <BaseTable
      v-if="open"
      class="automation-runs__table"
      :columns="columns"
      :rows="rows"
      :row-key="(row) => row.id"
    >
      <template #cell-status="{ row }">
        <span
          class="automation-runs__status"
          :class="`automation-runs__status--${row.status}`"
        >
          <span
            class="automation-runs__dot"
            aria-hidden="true"
          />
          {{ row.status === 'ok' ? t('automations.runs.ok') : t('automations.runs.error') }}
        </span>
      </template>
      <template #cell-action="{ row }">
        {{ row.action }}
        <span
          v-if="row.test"
          class="automation-runs__badge"
        >{{ t('automations.runs.testBadge') }}</span>
      </template>
      <template #cell-result="{ row }">
        <span class="automation-runs__result">{{ row.result }}</span>
      </template>
      <template #empty>
        <p class="automation-runs__empty">
          {{ t('automations.runs.empty') }}
        </p>
      </template>
    </BaseTable>
  </BasePanel>
</template>

<style scoped>
.automation-runs__table {
  height: 180px;
}

.automation-runs__status {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.automation-runs__dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-success);
}

.automation-runs__status--error {
  color: var(--color-danger-hover);
}

.automation-runs__status--error .automation-runs__dot {
  background: var(--color-danger);
}

.automation-runs__badge {
  margin-left: var(--space-2);
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}

.automation-runs__result {
  display: block;
  overflow: hidden;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.automation-runs__empty {
  margin: 0;
  color: var(--color-text-subtle);
}
</style>

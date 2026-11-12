<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';
import type { AutomationEventRow } from '../model/use-automation-library-rows.ts';
defineProps<{
  columns: readonly TableColumn[];
  rows: readonly AutomationEventRow[];
  query: string;
  checkboxKey: (id: string) => number;
}>();
const emit = defineEmits<{ setEnabled: [id: string, enabled: boolean] }>();
const selected = defineModel<string | null>('selected', { default: null });
const { t } = useAutomationText();
</script>

<template>
  <BaseTable
    v-model:selected="selected"
    class="automation-library__table"
    :columns="columns"
    :rows="rows"
    :row-key="(row) => row.id"
    selectable
  >
    <template #cell-enabled="{ row }">
      <span @pointerdown.stop>
        <BaseCheckbox
          :key="checkboxKey(row.id)"
          :model-value="row.enabled"
          :aria-label="t('automations.library.enable', { name: row.name })"
          @update:model-value="emit('setEnabled', row.id, $event)"
        />
      </span>
    </template>
    <template #cell-name="{ row }">
      <span class="automation-library__name">{{ row.name }}</span>
      <span class="automation-library__sub">{{ row.conditions }}</span>
    </template>
    <template #cell-actions="{ row }">
      <span class="automation-library__ellipsis">{{ row.actions.join(', ') || '—' }}</span>
    </template>
    <template #empty>
      <BaseEmptyState
        v-if="query.trim()"
        :title="t('automations.library.noMatches', { query: query.trim() })"
      />
      <BaseEmptyState
        v-else
        :title="t('automations.library.emptyEvents')"
        :description="t('automations.library.emptyEventsHint')"
      >
        <template #icon>
          <IconFlow :size="28" />
        </template>
      </BaseEmptyState>
    </template>
  </BaseTable>
</template>

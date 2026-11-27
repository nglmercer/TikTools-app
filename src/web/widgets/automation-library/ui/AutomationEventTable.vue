<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';
import type { AutomationEventRow } from '../model/use-automation-library-rows.ts';
import AutomationLibraryTable from './AutomationLibraryTable.vue';

defineProps<{
  columns: readonly TableColumn[];
  rows: readonly AutomationEventRow[];
  query: string;
  checkboxKey: (id: string) => number;
}>();
const emit = defineEmits<{ setEnabled: [id: string, enabled: boolean]; rowMenu: [id: string, event: MouseEvent] }>();
const selected = defineModel<string | null>('selected', { default: null });
const { t } = useAutomationText();
</script>

<template>
  <AutomationLibraryTable
    v-model:selected="selected"
    :columns="columns"
    :rows="rows"
    :query="query"
    :checkbox-key="checkboxKey"
    :detail="(row) => row.conditions"
    :empty-title="t('automations.library.emptyEvents')"
    :empty-description="t('automations.library.emptyEventsHint')"
    @set-enabled="(id, enabled) => emit('setEnabled', id, enabled)"
    @row-menu="(id, event) => emit('rowMenu', id, event)"
  >
    <template #cell-actions="{ row }">
      <span class="automation-library__ellipsis">{{ row.actions.join(', ') || '—' }}</span>
    </template>
  </AutomationLibraryTable>
</template>

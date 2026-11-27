<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';
import { actionTypeIcon } from '../lib/action-type-icon.ts';
import type { AutomationActionRow } from '../model/use-automation-library-rows.ts';
import AutomationLibraryTable from './AutomationLibraryTable.vue';

defineProps<{
  columns: readonly TableColumn[];
  rows: readonly AutomationActionRow[];
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
    :detail="(row) => row.detail"
    :empty-title="t('automations.library.emptyActions')"
    :empty-description="t('automations.library.emptyActionsHint')"
    @set-enabled="(id, enabled) => emit('setEnabled', id, enabled)"
    @row-menu="(id, event) => emit('rowMenu', id, event)"
  >
    <template #cell-type="{ row }">
      <span class="automation-library__type">
        <component
          :is="actionTypeIcon(row.typeId)"
          class="automation-library__type-icon"
        />
        <span class="automation-library__ellipsis">{{ row.type }}</span>
      </span>
      <span
        v-if="row.missing"
        class="automation-library__warning"
      >{{ t('automations.library.pluginMissing') }}</span>
    </template>
    <template #cell-lastRun="{ row }">
      <span :class="{ 'automation-library__failed': row.failed }">{{ row.lastRun }}</span>
    </template>
  </AutomationLibraryTable>
</template>

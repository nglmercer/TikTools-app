<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';
import { actionTypeIcon } from '../lib/action-type-icon.ts';
import type { AutomationActionRow } from '../model/use-automation-library-rows.ts';
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
        <BaseSwitch
          :key="checkboxKey(row.id)"
          :model-value="row.enabled"
          :aria-label="t('automations.library.enable', { name: row.name })"
          @update:model-value="emit('setEnabled', row.id, $event)"
        />
      </span>
    </template>
    <template #cell-name="{ row }">
      <span class="automation-library__name">{{ row.name }}</span>
      <span class="automation-library__sub">{{ row.detail }}</span>
    </template>
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
    <template #cell-menu="{ row }">
      <span @pointerdown.stop>
        <BaseIconButton
          :label="t('automations.library.rowActions', { name: row.name })"
          size="sm"
          aria-haspopup="menu"
          @click="emit('rowMenu', row.id, $event)"
        >
          <IconMoreVertical />
        </BaseIconButton>
      </span>
    </template>
    <template #empty>
      <BaseEmptyState
        v-if="query.trim()"
        :title="t('automations.library.noMatches', { query: query.trim() })"
      />
      <BaseEmptyState
        v-else
        :title="t('automations.library.emptyActions')"
        :description="t('automations.library.emptyActionsHint')"
      >
        <template #icon>
          <IconFlow :size="28" />
        </template>
      </BaseEmptyState>
    </template>
  </BaseTable>
</template>

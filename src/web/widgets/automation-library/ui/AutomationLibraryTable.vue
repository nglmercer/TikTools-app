<script setup lang="ts" generic="Row extends { id: string; name: string; enabled: boolean }">
import { computed, useSlots } from 'vue';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';

/**
 * The part of the events and actions tables that is the same: a selectable table whose first column switches the
 * row on or off, whose name cell carries a one-line detail, and whose last column opens the row menu. A table adds
 * its own columns through `cell-<key>` slots.
 */
defineProps<{
  columns: readonly TableColumn[];
  rows: readonly Row[];
  query: string;
  checkboxKey: (id: string) => number;
  /** The grey second line under the name. */
  detail: (row: Row) => string;
  emptyTitle: string;
  emptyDescription: string;
}>();
const emit = defineEmits<{ setEnabled: [id: string, enabled: boolean]; rowMenu: [id: string, event: MouseEvent] }>();
const selected = defineModel<string | null>('selected', { default: null });
const { t } = useAutomationText();
// A table's own columns arrive as `cell-<key>` slots and are handed to BaseTable unchanged.
const slots = useSlots();
const ownCells = computed(() => Object.keys(slots).filter((name) => name.startsWith('cell-')));
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
      <span class="automation-library__sub">{{ detail(row) }}</span>
    </template>
    <template
      v-for="name in ownCells"
      :key="name"
      #[name]="scope"
    >
      <slot
        :name="name"
        v-bind="scope"
      />
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
        :title="emptyTitle"
        :description="emptyDescription"
      >
        <template #icon>
          <IconFlow :size="28" />
        </template>
      </BaseEmptyState>
    </template>
  </BaseTable>
</template>

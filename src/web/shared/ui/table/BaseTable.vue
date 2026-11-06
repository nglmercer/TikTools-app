<script setup lang="ts" generic="T extends object, K extends string | number">
import { computed } from 'vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import { nextSort, type TableColumn, type TableSort } from './table-types.ts';

const props = withDefaults(
  defineProps<{
    columns: readonly TableColumn[];
    rows: readonly T[];
    rowKey: (row: T) => K;
    selectable?: boolean;
  }>(),
  { selectable: false },
);

const emit = defineEmits<{ activate: [row: T] }>();
const selected = defineModel<K | null>('selected', { default: null });
const sort = defineModel<TableSort | null>('sort', { default: null });

const keys = computed(() => props.rows.map(props.rowKey));
const selectedIndex = computed(() => (selected.value === null ? -1 : keys.value.indexOf(selected.value)));

function cellValue(row: T, key: string): unknown {
  return (row as Record<string, unknown>)[key];
}

function ariaSort(column: TableColumn): 'ascending' | 'descending' | 'none' | undefined {
  if (!column.sortable) return undefined;
  if (sort.value?.key !== column.key) return 'none';
  return sort.value.direction === 'asc' ? 'ascending' : 'descending';
}

function select(index: number): void {
  const key = keys.value[index];
  if (props.selectable && key !== undefined) selected.value = key;
}

function onKeydown(event: KeyboardEvent): void {
  if (!props.selectable || !props.rows.length) return;
  const last = props.rows.length - 1;
  const from = selectedIndex.value;
  const targets: Record<string, () => number> = {
    ArrowDown: () => Math.min(from + 1, last),
    ArrowUp: () => (from < 0 ? last : Math.max(from - 1, 0)),
    Home: () => 0,
    End: () => last,
  };
  if (event.key === 'Enter' && from >= 0) {
    event.preventDefault();
    emit('activate', props.rows[from]!);
    return;
  }
  const target = targets[event.key];
  if (!target) return;
  event.preventDefault();
  select(target());
}
</script>

<template>
  <div
    class="base-table"
    :tabindex="selectable ? 0 : undefined"
    @keydown="onKeydown"
  >
    <table
      class="base-table__table"
      :role="selectable ? 'grid' : undefined"
    >
      <colgroup>
        <col
          v-for="column in columns"
          :key="column.key"
          :style="{ width: column.width }"
        >
      </colgroup>
      <thead>
        <tr>
          <th
            v-for="column in columns"
            :key="column.key"
            class="base-table__header"
            :class="`base-table__cell--${column.align ?? 'start'}`"
            scope="col"
            :aria-sort="ariaSort(column)"
          >
            <button
              v-if="column.sortable"
              class="base-table__sort"
              type="button"
              @click="sort = nextSort(sort, column.key)"
            >
              {{ column.label }}
              <IconChevronUp
                v-if="sort?.key === column.key && sort.direction === 'asc'"
                :size="12"
              />
              <IconChevronDown
                v-else-if="sort?.key === column.key"
                :size="12"
              />
            </button>
            <template v-else>
              {{ column.label }}
            </template>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(row, index) in rows"
          :key="keys[index]"
          class="base-table__row"
          :aria-selected="selectable ? index === selectedIndex : undefined"
          @pointerdown="select(index)"
          @dblclick="emit('activate', row)"
        >
          <td
            v-for="column in columns"
            :key="column.key"
            class="base-table__cell"
            :class="`base-table__cell--${column.align ?? 'start'}`"
            :role="selectable ? 'gridcell' : undefined"
          >
            <slot
              :name="`cell-${column.key}`"
              :row="row"
              :value="cellValue(row, column.key)"
            >
              {{ cellValue(row, column.key) }}
            </slot>
          </td>
        </tr>
      </tbody>
    </table>
    <div
      v-if="!rows.length && $slots.empty"
      class="base-table__empty"
    >
      <slot name="empty" />
    </div>
  </div>
</template>

<style scoped>
.base-table {
  min-height: 0;
  overflow: auto;
  background: var(--color-bg-panel);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  outline: none;
}

.base-table__table {
  width: 100%;
  border-collapse: collapse;
  table-layout: auto;
}

.base-table__header {
  position: sticky;
  top: 0;
  z-index: 1;
  height: var(--control-height-md);
  padding: 0 var(--space-4);
  border-right: var(--border-width) solid var(--color-border-subtle);
  border-bottom: var(--border-width) solid var(--color-border);
  background: var(--color-bg-panel-header);
  color: var(--color-text-muted);
  font-weight: var(--font-weight-semibold);
  white-space: nowrap;
}

.base-table__sort {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  max-width: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
  outline: none;
  transform: none;
}

.base-table__sort:hover,
.base-table__sort:focus-visible {
  color: var(--color-text);
}

.base-table__row:hover {
  background: var(--color-bg-hover);
}

.base-table__row[aria-selected='true'] {
  background: var(--color-bg-selected);
  color: var(--color-text-on-accent);
}

.base-table:focus-visible .base-table__row[aria-selected='true'] {
  outline: var(--border-width) solid var(--color-border-focus);
  outline-offset: calc(var(--border-width) * -1);
}

.base-table__cell {
  height: var(--control-height-md);
  padding: 0 var(--space-4);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.base-table__cell--start {
  text-align: left;
}

.base-table__cell--center {
  text-align: center;
}

.base-table__cell--end {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.base-table__empty {
  padding: var(--space-6);
  color: var(--color-text-subtle);
  text-align: center;
}
</style>

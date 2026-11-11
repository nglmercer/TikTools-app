import { computed, ref, watch, type Ref } from 'vue';
import { isGlobalKey, sortedGlobalEntries } from '@/entities/runtime-global/lib/global-key.ts';

export interface GlobalRow {
  id: number;
  key: string;
  value: string;
}

export type GlobalRowError = 'emptyKey' | 'invalidKey' | 'duplicateKey';

let nextRowId = 0;

function createRow(key = '', value = ''): GlobalRow {
  nextRowId += 1;
  return { id: nextRowId, key, value };
}

export function rowsFromGlobals(globals: Record<string, string>): GlobalRow[] {
  return sortedGlobalEntries(globals).map(([key, value]) => createRow(key, value));
}

export function validateGlobalRows(rows: readonly GlobalRow[]): Map<number, GlobalRowError> {
  const counts = new Map<string, number>();
  for (const row of rows) {
    if (row.key) counts.set(row.key, (counts.get(row.key) ?? 0) + 1);
  }
  const errors = new Map<number, GlobalRowError>();
  for (const row of rows) {
    if (!row.key) {
      if (row.value) errors.set(row.id, 'emptyKey');
    } else if (!isGlobalKey(row.key)) {
      errors.set(row.id, 'invalidKey');
    } else if ((counts.get(row.key) ?? 0) > 1) {
      errors.set(row.id, 'duplicateKey');
    }
  }
  return errors;
}

export function globalsFromRows(rows: readonly GlobalRow[]): Record<string, string> {
  return Object.fromEntries(rows.filter((row) => row.key).map((row) => [row.key, row.value]));
}

export function useGlobalsDraft(source: Readonly<Ref<Record<string, string>>>) {
  const rows = ref<GlobalRow[]>(rowsFromGlobals(source.value));
  const dirty = ref(false);
  const errors = computed(() => validateGlobalRows(rows.value));
  const blocked = computed(() => errors.value.size > 0);

  watch(source, (globals) => {
    if (!dirty.value) rows.value = rowsFromGlobals(globals);
  });

  function add(): void {
    rows.value = [...rows.value, createRow()];
  }

  function update(id: number, patch: Partial<Pick<GlobalRow, 'key' | 'value'>>): void {
    rows.value = rows.value.map((row) => (row.id === id ? { ...row, ...patch } : row));
    dirty.value = true;
  }

  function remove(id: number): void {
    rows.value = rows.value.filter((row) => row.id !== id);
    dirty.value = true;
  }

  function markSaved(): void {
    dirty.value = false;
  }

  /** Drops every unsaved edit and shows the stored values again. */
  function discard(): void {
    rows.value = rowsFromGlobals(source.value);
    dirty.value = false;
  }

  return { rows, dirty, errors, blocked, add, update, remove, markSaved, discard };
}

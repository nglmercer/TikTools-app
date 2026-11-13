import { computed, ref, watch, type Ref } from 'vue';
import { isGlobalKey, sortedGlobalEntries } from '@/entities/runtime-global/lib/global-key.ts';
import { variableText, type RuntimeVariable, type RuntimeVariableKind } from '@/entities/runtime-global/lib/runtime-variable.ts';

export interface GlobalRow {
  id: number;
  key: string;
  value: string;
  kind?: RuntimeVariableKind;
}

export type GlobalRowError = 'emptyKey' | 'invalidKey' | 'duplicateKey' | 'invalidNumber' | 'invalidBoolean' | 'invalidList' | 'valueTooLong';

let nextRowId = 0;

function createRow(key = '', value = '', kind: RuntimeVariableKind = 'text'): GlobalRow {
  nextRowId += 1;
  return { id: nextRowId, key, value, kind };
}

export function variableFromRow(row: GlobalRow): RuntimeVariable {
  switch (row.kind ?? 'text') {
    case 'text': case 'secret':
      if (new TextEncoder().encode(row.value).length > 4096) throw new Error('valueTooLong');
      return { type: row.kind === 'secret' ? 'secret' : 'text', value: row.value };
    case 'number': {
      let value: unknown;
      try { value = JSON.parse(row.value); } catch { throw new Error('invalidNumber'); }
      if (typeof value !== 'number' || !Number.isFinite(value)) throw new Error('invalidNumber');
      return Number.isInteger(value) && !Number.isSafeInteger(value)
        ? { type: 'number', value, numberText: row.value.trim() }
        : { type: 'number', value };
    }
    case 'boolean':
      if (row.value !== 'true' && row.value !== 'false') throw new Error('invalidBoolean');
      return { type: 'boolean', value: row.value === 'true' };
    case 'list': {
      let value: unknown;
      try { value = JSON.parse(row.value); } catch { throw new Error('invalidList'); }
      if (!Array.isArray(value) || value.length > 100 || !value.every((item) => typeof item === 'string' && [...item].length <= 256)) throw new Error('invalidList');
      return { type: 'list', value: value as string[] };
    }
  }
}

export function variablesFromRows(rows: readonly GlobalRow[]): Record<string, RuntimeVariable> {
  return Object.fromEntries(rows.filter((row) => row.key).map((row) => [row.key, variableFromRow(row)]));
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
    } else {
      try { variableFromRow(row); } catch (cause) { errors.set(row.id, (cause as Error).message as GlobalRowError); }
    }
  }
  return errors;
}

export function globalsFromRows(rows: readonly GlobalRow[]): Record<string, string> {
  return Object.fromEntries(rows.filter((row) => row.key).map((row) => [row.key, row.value]));
}

export function useGlobalsDraft(source: Readonly<Ref<Record<string, string>>>, typed?: Readonly<Ref<{ revision: string | null; variables: Record<string, RuntimeVariable> }>>) {
  const readRows = () => typed
    ? Object.entries(typed.value.variables).sort(([a], [b]) => a.localeCompare(b)).map(([key, variable]) => createRow(key, variableText(variable), variable.type))
    : rowsFromGlobals(source.value);
  const rows = ref<GlobalRow[]>(readRows());
  const baseRevision = ref<string | null>(typed?.value.revision ?? null);
  const dirty = ref(false);
  const errors = computed(() => validateGlobalRows(rows.value));
  const blocked = computed(() => errors.value.size > 0);

  watch(() => [source.value, typed?.value], () => {
    if (!dirty.value) { rows.value = readRows(); baseRevision.value = typed?.value.revision ?? null; }
  });

  function add(): void {
    rows.value = [...rows.value, createRow()];
  }

  function update(id: number, patch: Partial<Pick<GlobalRow, 'key' | 'value' | 'kind'>>): void {
    rows.value = rows.value.map((row) => (row.id === id ? { ...row, ...patch } : row));
    dirty.value = true;
  }

  function remove(id: number): void {
    rows.value = rows.value.filter((row) => row.id !== id);
    dirty.value = true;
  }

  function markSaved(): void {
    dirty.value = false;
    rows.value = readRows();
    baseRevision.value = typed?.value.revision ?? null;
  }

  /** Drops every unsaved edit and shows the stored values again. */
  function discard(): void {
    rows.value = readRows();
    baseRevision.value = typed?.value.revision ?? null;
    dirty.value = false;
  }

  return { rows, dirty, baseRevision, errors, blocked, add, update, remove, markSaved, discard };
}

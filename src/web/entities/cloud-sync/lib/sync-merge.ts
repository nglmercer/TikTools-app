import { changedTopLevelKeys, jsonEqual } from '@/shared/ui/json/json-diff.ts';
import type { JsonValue, SyncChange } from '../api/sync-adapter.ts';

export type FieldSide = 'base' | 'local' | 'cloud';

/** One top-level field where Local and Cloud disagree. `$` stands for a value that is not an object. */
export interface FieldChoice {
  key: string;
  base: JsonValue | undefined;
  local: JsonValue | undefined;
  cloud: JsonValue | undefined;
  /** The side picked by default: whichever side changed, Local when both did. */
  choice: FieldSide;
}

export const ROOT_FIELD = '$';

function asObject(value: unknown): Record<string, JsonValue> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, JsonValue>) : null;
}

/** Fields that need a decision, built from base to local and base to Cloud. */
export function buildFieldChoices(base: JsonValue | undefined, local: JsonValue | undefined, cloud: JsonValue | undefined): FieldChoice[] {
  const b = asObject(base);
  const l = asObject(local);
  const c = asObject(cloud);
  if (!l || !c || (base !== undefined && !b)) {
    if (jsonEqual(local, cloud)) return [];
    return [{ key: ROOT_FIELD, base, local, cloud, choice: 'local' }];
  }
  const mine = changedTopLevelKeys(b ?? {}, l);
  const theirs = changedTopLevelKeys(b ?? {}, c);
  const touchedLocal = new Set([...mine.added, ...mine.removed, ...mine.changed]);
  const touchedCloud = new Set([...theirs.added, ...theirs.removed, ...theirs.changed]);
  const order = [...Object.keys(l), ...Object.keys(c).filter((key) => !(key in l)), ...Object.keys(b ?? {}).filter((key) => !(key in l) && !(key in c))];
  const fields: FieldChoice[] = [];
  for (const key of new Set(order)) {
    if (jsonEqual(l[key], c[key])) continue;
    const localChanged = touchedLocal.has(key);
    const cloudChanged = touchedCloud.has(key);
    fields.push({ key, base: b?.[key], local: l[key], cloud: c[key], choice: cloudChanged && !localChanged ? 'cloud' : 'local' });
  }
  return fields;
}

/** The merged value for the picked sides. Fields that do not differ keep their (shared) value. */
export function mergeChoices(
  local: JsonValue | undefined,
  cloud: JsonValue | undefined,
  fields: readonly Pick<FieldChoice, 'key' | 'base' | 'local' | 'cloud'>[],
  picks: Readonly<Record<string, FieldSide>>,
): JsonValue | undefined {
  const pick = (field: Pick<FieldChoice, 'base' | 'local' | 'cloud'>, key: string): JsonValue | undefined => field[picks[key] ?? 'local'];
  const root = fields.find((field) => field.key === ROOT_FIELD);
  if (root) return pick(root, ROOT_FIELD);
  const l = asObject(local) ?? {};
  const c = asObject(cloud) ?? {};
  const out: Record<string, JsonValue> = {};
  for (const key of new Set([...Object.keys(l), ...Object.keys(c)])) {
    const field = fields.find((entry) => entry.key === key);
    const value = field ? pick(field, key) : l[key];
    if (value !== undefined) out[key] = value;
  }
  return out;
}

export interface ChangeGroup {
  side: SyncChange['side'];
  changes: SyncChange[];
}

/** Changes grouped To send, To fetch, Conflicts, in that order, skipping empty groups. */
export function groupChanges(changes: readonly SyncChange[]): ChangeGroup[] {
  return (['local', 'remote', 'conflict'] as const)
    .map((side) => ({ side, changes: changes.filter((change) => change.side === side) }))
    .filter((group) => group.changes.length > 0);
}

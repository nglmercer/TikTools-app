/** Shared model for the change review workspace. One item is one logical
 * record (event, action, template, version diff); the semantic field diff is
 * derived generically by flattening both sides, so every review flow renders
 * the same element-first view without per-domain mapping code. */

export type ChangeReviewStatus = 'added' | 'modified' | 'deleted' | 'unchanged';

export interface ChangeReviewItem {
  /** Stable key used for expansion state. */
  id: string;
  title: string;
  /** Context line under the title: type id, profile, revision… */
  subtitle?: string;
  /** Derived from before/after when omitted. */
  status?: ChangeReviewStatus;
  /** Leaves the active set — hidden by the profile membership filter. */
  leaving?: boolean;
  /** Collapses repeated identical changes into one representative card. */
  count?: number;
  /** New records can be renamed before applying the change. */
  editable?: boolean;
  /** Show every field as context even when unchanged — for reviews whose
   * subject is not the record content itself, like a profile assignment. */
  alwaysShowFields?: boolean;
  before?: unknown;
  after?: unknown;
}

export interface ChangeReviewField {
  key: string;
  before: unknown;
  after: unknown;
  status: ChangeReviewStatus;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Flattens plain objects into dot-path leaves. Arrays stay leaf values:
 * their element shape is rarely meaningful as independent review rows. */
export function flattenLeaves(value: unknown, prefix = '', out: Record<string, unknown> = {}): Record<string, unknown> {
  if (!isPlainObject(value)) {
    if (prefix) out[prefix] = value;
    return out;
  }
  for (const [key, entry] of Object.entries(value)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (isPlainObject(entry)) flattenLeaves(entry, path, out);
    else out[path] = entry;
  }
  return out;
}

function leafEqual(a: unknown, b: unknown): boolean {
  if (Array.isArray(a) || Array.isArray(b) || isPlainObject(a) || isPlainObject(b)) {
    try {
      return JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
    } catch {
      return false;
    }
  }
  return a === b;
}

/** Semantic diff of both sides as flattened leaf fields, changed first. */
export function diffFields(before: unknown, after: unknown): ChangeReviewField[] {
  const left = flattenLeaves(before);
  const right = flattenLeaves(after);
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
  const fields: ChangeReviewField[] = keys.map((key) => {
    const hasBefore = key in left;
    const hasAfter = key in right;
    const a = left[key];
    const b = right[key];
    let status: ChangeReviewStatus;
    if (!hasBefore) status = 'added';
    else if (!hasAfter) status = 'deleted';
    else status = leafEqual(a, b) ? 'unchanged' : 'modified';
    return { key, before: hasBefore ? a : undefined, after: hasAfter ? b : undefined, status };
  });
  const order: Record<ChangeReviewStatus, number> = { modified: 0, added: 1, deleted: 2, unchanged: 3 };
  return fields.sort((a, b) => order[a.status] - order[b.status]);
}

export function deriveStatus(before: unknown, after: unknown): ChangeReviewStatus {
  const hasBefore = before !== null && before !== undefined;
  const hasAfter = after !== null && after !== undefined;
  if (!hasBefore && hasAfter) return 'added';
  if (hasBefore && !hasAfter) return 'deleted';
  return diffFields(before, after).some((field) => field.status !== 'unchanged') ? 'modified' : 'unchanged';
}

export function itemStatus(item: ChangeReviewItem): ChangeReviewStatus {
  return item.status ?? deriveStatus(item.before, item.after);
}

/** Short readable rendering of one leaf value. Arrays render compact JSON. */
export function displayLeaf(value: unknown): string {
  if (value === null || value === undefined || value === '') return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

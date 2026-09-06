import type { JsonObject, JsonValue } from '../../../automation/types.ts';

// Compatibility entrypoint. New controls import from ./index.ts and the
// focused modules beside it; existing extensions may keep this path.

/**
 * Generic autocomplete item. Any input can push these — from a live event
 * object, a JSON schema, a static list, or a mix of all three.
 *
 * - `value` is what gets inserted (a dotted path like `event.user.uniqueId`,
 *   a header name, a full `{{ }}` snippet source…).
 * - `label` is the human line in the list.
 * - `kind` / `detail` describe the type (`string`, `number`, `object`…).
 * - `preview` is a live sample value (`"luna_dev"`, `42`).
 * - `documentation` is the long hint shown on hover.
 */
export type AutocompleteKind =
  | 'string'
  | 'number'
  | 'boolean'
  | 'object'
  | 'array'
  | 'null'
  | 'path'
  | 'snippet'
  | 'unknown';

export interface AutocompleteItem {
  value: string;
  label: string;
  kind?: AutocompleteKind;
  /** Short type line shown next to the label, e.g. `string · event`. */
  detail?: string;
  /** Long description shown in the hover/detail pane. */
  documentation?: string;
  /** Live sample value shown on hover, e.g. `"luna_dev"`. */
  preview?: string;
}

export type AutocompleteSource =
  | { kind: 'list'; items: AutocompleteItem[] }
  | { kind: 'object'; value: JsonValue; prefix?: string; maxItems?: number }
  | { kind: 'schema'; schema: JsonObject; prefix?: string };

/** Backwards-compatible alias: template suggestions are autocomplete items. */
export type GenericSuggestion = AutocompleteItem;

export function inferKind(value: JsonValue | undefined): AutocompleteKind {
  if (value === undefined) return 'unknown';
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  switch (typeof value) {
    case 'string': return 'string';
    case 'number': return 'number';
    case 'boolean': return 'boolean';
    case 'object': return 'object';
    default: return 'unknown';
  }
}

export function formatPreview(value: JsonValue | undefined, maxLength = 96): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value === 'string') {
    const quoted = JSON.stringify(value) ?? `"${value}"`;
    return truncate(quoted, maxLength);
  }
  if (value === null) return 'null';
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    const serialized = JSON.stringify(value) ?? String(value);
    return truncate(serialized, maxLength);
  } catch {
    return String(value);
  }
}

function truncate(value: string, maxLength: number): string {
  return value.length > maxLength ? `${value.slice(0, maxLength - 3)}...` : value;
}

function humanizePath(path: string): string {
  const last = path.split('.').pop() ?? path;
  return last.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/^./, (c) => c.toUpperCase());
}

/**
 * Push any object as autocomplete: `{ event: {...}, data: {...} }` becomes
 * `event.user.uniqueId`, `data.count`… with inferred type + live preview.
 * Arrays expose their first element as `prefix.0.*` so shapes stay visible.
 */
export function suggestionsFromObject(
  root: JsonValue,
  prefix = 'event',
  options: { maxItems?: number; maxDepth?: number; labelPrefix?: string } = {},
): AutocompleteItem[] {
  const maxItems = options.maxItems ?? 120;
  const maxDepth = options.maxDepth ?? 4;
  const out: AutocompleteItem[] = [];
  const seen = new Set<string>();

  const visit = (value: JsonValue, path: string, depth: number): void => {
    if (out.length >= maxItems || depth > maxDepth) return;
    if (!seen.has(path)) {
      seen.add(path);
      const kind = inferKind(value);
      out.push({
        value: path,
        label: humanizePath(path),
        kind,
        detail: kind,
        preview: formatPreview(value),
      });
    }
    if (value !== null && typeof value === 'object' && depth < maxDepth) {
      if (Array.isArray(value)) {
        const first = value[0];
        if (first !== undefined && first !== null && typeof first === 'object' && !Array.isArray(first)) {
          for (const [key, entry] of Object.entries(first as JsonObject)) {
            if (entry === undefined) continue;
            visit(entry ?? null, `${path}.0.${key}`, depth + 1);
            if (out.length >= maxItems) return;
          }
        } else if (value.length > 0) {
          // Scalar arrays still deserve one indexed entry for the preview.
          const preview = formatPreview(value);
          const existing = out[out.length - 1];
          if (existing && preview) existing.preview = preview;
        }
        return;
      }
      for (const [key, entry] of Object.entries(value)) {
        if (entry === undefined) continue;
        if (!/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key)) continue;
        visit(entry ?? null, path ? `${path}.${key}` : key, depth + 1);
        if (out.length >= maxItems) return;
      }
    }
  };

  if (root !== null && typeof root === 'object' && !Array.isArray(root) && prefix === '') {
    for (const [key, entry] of Object.entries(root as JsonObject)) {
      if (entry === undefined) continue;
      visit(entry ?? null, key, 0);
      if (out.length >= maxItems) break;
    }
    return out;
  }
  visit(root, prefix, 0);
  return out;
}

/**
 * Push any JSON Schema as autocomplete: each `properties` entry becomes an
 * item carrying its `type` + `description` so hover can show value or type.
 */
export function suggestionsFromSchema(schema: JsonObject, prefix = ''): AutocompleteItem[] {
  const properties = schema.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return [];
  const out: AutocompleteItem[] = [];
  for (const [key, raw] of Object.entries(properties as JsonObject)) {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) continue;
    const field = raw as JsonObject;
    const type = typeof field.type === 'string' ? field.type : Array.isArray(field.type) ? String(field.type[0] ?? 'unknown') : 'unknown';
    const description = typeof field.description === 'string'
      ? field.description
      : typeof field.title === 'string' ? field.title : undefined;
    const example = field.default ?? field.example ?? (field.enum as JsonValue[] | undefined)?.[0];
    const path = prefix ? `${prefix}.${key}` : key;
    out.push({
      value: path,
      label: typeof field.title === 'string' ? field.title : humanizePath(path),
      kind: normalizeSchemaKind(type),
      detail: type,
      documentation: description,
      preview: example !== undefined ? formatPreview(example as JsonValue) : undefined,
    });
  }
  return out;
}

function normalizeSchemaKind(type: string): AutocompleteKind {
  switch (type) {
    case 'string': return 'string';
    case 'number':
    case 'integer': return 'number';
    case 'boolean': return 'boolean';
    case 'object': return 'object';
    case 'array': return 'array';
    case 'null': return 'null';
    default: return 'unknown';
  }
}

/** Merge several sources, keeping first-seen order and deduping by value. */
export function mergeSuggestions(...lists: Array<AutocompleteItem[] | undefined>): AutocompleteItem[] {
  const seen = new Set<string>();
  const out: AutocompleteItem[] = [];
  for (const list of lists) {
    if (!list) continue;
    for (const item of list) {
      if (!item?.value || seen.has(item.value)) continue;
      seen.add(item.value);
      out.push(item);
    }
  }
  return out;
}

export function sourcesToSuggestions(sources: AutocompleteSource[]): AutocompleteItem[] {
  return mergeSuggestions(...sources.map((source) => {
    if (source.kind === 'list') return source.items;
    if (source.kind === 'object') return suggestionsFromObject(source.value, source.prefix ?? 'event', { maxItems: source.maxItems });
    return suggestionsFromSchema(source.schema, source.prefix ?? '');
  }));
}

export type ScoredSuggestion<T extends AutocompleteItem = AutocompleteItem> = {
  item: T;
  score: number;
  matchRanges: Array<{ start: number; end: number }>;
};

/**
 * Fuzzy filter used by every template input. Scores prefix matches on the
 * value highest so `event.user` surfaces before a label-only hit. Field-name
 * matches (`unique` → `event.user.uniqueId`) outrank mid-path hits, and the
 * subsequence fallback only runs on 3+ chars to keep 2-letter words quiet.
 * Returns the ranges to `<mark>` in the dropdown (highlight).
 */
export function filterSuggestions<T extends AutocompleteItem = AutocompleteItem>(
  items: T[],
  query: string,
  limit = 10,
): Array<ScoredSuggestion<T>> {
  const needle = query.trim().toLowerCase();
  if (!needle) return items.slice(0, limit).map((item) => ({ item, score: 0, matchRanges: [] }));
  const scored: Array<ScoredSuggestion<T>> = [];
  for (const item of items) {
    const value = item.value.toLowerCase();
    const label = item.label.toLowerCase();
    const lastSegment = value.split('.').pop() ?? value;
    let score = -1;
    let ranges: Array<{ start: number; end: number }> = [];
    if (value.startsWith(needle)) {
      score = 100;
      ranges = [{ start: 0, end: needle.length }];
    } else if (lastSegment.startsWith(needle)) {
      const at = value.length - lastSegment.length;
      score = 80 - Math.min(at, 20);
      ranges = [{ start: at, end: at + needle.length }];
    } else {
      const at = value.indexOf(needle);
      if (at >= 0) {
        score = 50 - Math.min(at, 40);
        ranges = [{ start: at, end: at + needle.length }];
      } else {
        const lastAt = lastSegment.indexOf(needle);
        if (lastAt >= 0) {
          const atValue = value.length - lastSegment.length + lastAt;
          score = 45 - Math.min(atValue, 40);
          ranges = [{ start: atValue, end: atValue + needle.length }];
        } else {
          const labelAt = label.indexOf(needle);
          if (labelAt >= 0) {
            score = 20 - Math.min(labelAt, 15);
            ranges = [];
          } else if (needle.length >= 3) {
            // Subsequence fallback (e.g. `euu` → `event.user.uniqueId`).
            const sub = subsequenceRanges(value, needle);
            if (sub) {
              score = 5;
              ranges = sub;
            }
          }
        }
      }
    }
    if (score >= 0) scored.push({ item, score, matchRanges: ranges });
  }
  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, limit);
}

function subsequenceRanges(haystack: string, needle: string): Array<{ start: number; end: number }> | null {
  const ranges: Array<{ start: number; end: number }> = [];
  let cursor = 0;
  for (const char of needle) {
    const at = haystack.indexOf(char, cursor);
    if (at < 0) return null;
    ranges.push({ start: at, end: at + 1 });
    cursor = at + 1;
  }
  // Too scattered = noise; cap the span.
  const span = (ranges[ranges.length - 1]?.end ?? 0) - (ranges[0]?.start ?? 0);
  if (span > 24) return null;
  return ranges;
}

/** Split a string into highlighted / plain segments for `<mark>` rendering. */
export function highlightSegments(
  text: string,
  ranges: Array<{ start: number; end: number }>,
): Array<{ text: string; highlight: boolean }> {
  if (ranges.length === 0) return [{ text, highlight: false }];
  const sorted = [...ranges].sort((a, b) => a.start - b.start);
  const out: Array<{ text: string; highlight: boolean }> = [];
  let cursor = 0;
  for (const range of sorted) {
    const start = Math.max(0, Math.min(range.start, text.length));
    const end = Math.max(start, Math.min(range.end, text.length));
    if (start > cursor) out.push({ text: text.slice(cursor, start), highlight: false });
    if (end > start) out.push({ text: text.slice(start, end), highlight: true });
    cursor = Math.max(cursor, end);
  }
  if (cursor < text.length) out.push({ text: text.slice(cursor), highlight: false });
  return out;
}

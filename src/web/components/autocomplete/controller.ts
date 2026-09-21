import type { AutocompleteItem } from './types.ts';
import { moveActiveIndex } from './autocomplete/keyboard.ts';

/** Converts legacy/foreign list entries into the one autocomplete shape. */
export function normalizeAutocompleteItem(value: unknown): AutocompleteItem {
  const item = (value && typeof value === 'object' ? value : {}) as Partial<AutocompleteItem> & { value?: unknown; label?: unknown };
  const rawValue = String(item.value ?? '');
  return {
    value: rawValue,
    label: String(item.label ?? rawValue),
    kind: item.kind,
    detail: item.detail ?? (item.kind ? String(item.kind) : undefined),
    documentation: item.documentation,
    preview: item.preview,
  };
}

/** Normalizes and deduplicates suggestions by inserted value. */
export function normalizeAutocompleteItems(values: unknown[]): AutocompleteItem[] {
  const seen = new Set<string>();
  const result: AutocompleteItem[] = [];
  for (const value of values) {
    const item = normalizeAutocompleteItem(value);
    if (!item.value || seen.has(item.value)) continue;
    seen.add(item.value);
    result.push(item);
  }
  return result;
}

/** Shared selection state transition for Arrow navigation.
 * @deprecated Use `moveActiveIndex` from `./autocomplete/keyboard.ts` instead (identical behavior).
 */
export const moveAutocompleteSelection = moveActiveIndex;

/** Merges registry, live, schema, and caller-provided sources in priority order. */
export function resolveAutocompleteSources(...sources: Array<AutocompleteItem[] | undefined>): AutocompleteItem[] {
  const seen = new Set<string>();
  const result: AutocompleteItem[] = [];
  for (const source of sources) {
    for (const item of source ?? []) {
      if (!item?.value || seen.has(item.value)) continue;
      seen.add(item.value);
      result.push(item);
    }
  }
  return result;
}

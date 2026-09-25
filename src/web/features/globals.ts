/**
 * Runtime globals store: operator key/value pairs rendered at event time
 * (`{{ globals.commandPort }}`), unlike template params which bake at
 * import. Host-owned via `globals.*` RPC; this module owns the DOM-side
 * validation, the settings editor state, and the autocomplete rows.
 */

import { ref } from 'vue';

import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import type { AutocompleteItem } from '../components/autocomplete/index.ts';
import type { Locale } from '../i18n.ts';

/** Mirrors the host key rule: letter/`_` first, 1..=64 chars. */
const GLOBAL_KEY_PATTERN = /^[A-Za-z_][A-Za-z0-9_.-]{0,63}$/;

export function isGlobalKey(key: string): boolean {
  return GLOBAL_KEY_PATTERN.test(key);
}

function inferKind(value: string): AutocompleteItem['kind'] {
  const trimmed = value.trim();
  if (trimmed === 'true' || trimmed === 'false') return 'boolean';
  if (trimmed !== '' && Number.isFinite(Number(trimmed))) return 'number';
  return 'string';
}

function previewValue(value: string): string {
  return value.length > 64 ? `${value.slice(0, 61)}...` : value;
}

/** One autocomplete row per global, sorted by key. Untagged = universal. */
export function globalSuggestionItems(
  globals: Record<string, string>,
  locale: Locale,
): AutocompleteItem[] {
  return Object.entries(globals)
    .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
    .map(([key, value]) => ({
      value: `globals.${key}`,
      label: key,
      kind: inferKind(value),
      detail: 'Global',
      documentation: locale === 'es'
        ? `Global en tiempo de ejecución · ${key} = ${previewValue(value)}`
        : `Runtime global · ${key} = ${previewValue(value)}`,
      preview: previewValue(value),
    }));
}

function asGlobalsMap(result: unknown): Record<string, string> {
  if (!result || typeof result !== 'object' || Array.isArray(result)) return {};
  const raw = (result as { globals?: unknown }).globals;
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {};
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
    if (typeof value === 'string') out[key] = value;
  }
  return out;
}

export function useGlobals(control: ControlClient) {
  const globals = ref<Record<string, string>>({});
  const loading = ref(false);
  const error = ref<string | null>(null);

  const loadGlobals = async (): Promise<void> => {
    loading.value = true;
    error.value = null;
    try {
      const result = await control.call('globals.list', {});
      globals.value = asGlobalsMap(result);
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loading.value = false;
    }
  };

  /**
   * Persists a draft: deletes removed keys, writes added/changed ones, in
   * key order. Local state only advances after every write succeeds; a
   * failure reloads from the host so the editor resyncs.
   */
  const saveGlobals = async (next: Record<string, string>): Promise<void> => {
    error.value = null;
    const current = globals.value;
    const deletions = Object.keys(current)
      .filter((key) => !(key in next))
      .sort();
    const writes = Object.entries(next)
      .filter(([key, value]) => current[key] !== value)
      .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
    try {
      for (const key of deletions) {
        await control.call('globals.delete', { key });
      }
      for (const [key, value] of writes) {
        await control.call('globals.set', { key, value });
      }
      globals.value = { ...next };
    } catch (failure) {
      const message = errorMessage(failure);
      await loadGlobals();
      error.value = message;
      throw failure;
    }
  };

  return { globals, loading, error, loadGlobals, saveGlobals };
}

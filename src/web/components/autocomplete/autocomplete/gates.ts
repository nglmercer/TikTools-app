import type { SuggestionItem, SuggestionScope, TemplateQuery } from '../types.ts';

/* ------------------------------------------------------------------ */
/* Scopes (S24): explicit per-consumer filtering.                      */
/* ------------------------------------------------------------------ */

/**
 * Keep rows for one consumer scope. Rows without a `scopes` tag are
 * universal (shown everywhere); `generic` matches every row. Consumers
 * pass their TemplateField `scope` straight through.
 */
export function filterByScope<T extends Pick<SuggestionItem, 'scopes'>>(
  items: readonly T[],
  scope: SuggestionScope,
): T[] {
  if (scope === 'generic') return [...items];
  return items.filter((item) => !item.scopes || item.scopes.length === 0 || item.scopes.includes(scope));
}

/* ------------------------------------------------------------------ */
/* Modes (S7/S8/S10): template opens only in braces, presets carry     */
/* zero event variables, options match dynamic lists.                  */
/* ------------------------------------------------------------------ */

/** Template gate: field supports templates + focused + caret in `{{ }}`. */
export function shouldOpenTemplate(input: {
  supportsTemplates: boolean;
  focused: boolean;
  query: TemplateQuery | null;
  explicitInvoke?: boolean;
}): boolean {
  if (!input.supportsTemplates || !input.focused) return false;
  if (input.query) return true;
  // Explicit invoke (Ctrl+Space) is the only path outside braces; plain
  // focus must NOT show event variables (S8).
  return input.explicitInvoke === true;
}

/**
 * Preset gate: focus-empty shows all, typing filters, explicit invoke
 * forces. Preset rows are URL destinations only — zero event vars (S10).
 */
export function shouldOpenPreset(input: {
  focused: boolean;
  explicitInvoke?: boolean;
  query: string;
  matchCount: number;
}): boolean {
  if (!input.focused) return false;
  if (input.explicitInvoke === true) return true;
  if (input.query.trim().length === 0) return true;
  return input.matchCount > 0;
}

/** Options gate: same shape as presets, over a dynamic list. */
export function shouldOpenOptions(input: {
  focused: boolean;
  explicitInvoke?: boolean;
  query: string;
  matchCount: number;
  openOnFocus?: boolean;
}): boolean {
  if (!input.focused) return false;
  if (input.explicitInvoke === true) return true;
  if (input.query.trim().length === 0) return input.openOnFocus !== false;
  return input.matchCount > 0;
}

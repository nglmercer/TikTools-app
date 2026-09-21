import { findTemplateQuery } from '../token.ts';

/* ------------------------------------------------------------------ */
/* Insertion (S14): caret-preserving, range-replacing.                 */
/* ------------------------------------------------------------------ */

export type InsertResult = { value: string; caret: number };

/**
 * Insert a template variable: replaces the `{{ …` span under the caret
 * with `{{ value }}` (consuming a directly-following `}}` so no stray
 * braces remain). Outside braces (explicit invoke) inserts `{{ value }}`
 * at the caret, replacing the current word when one is under it.
 * Surrounding text and the trailing caret position are preserved.
 */
export function applyTemplateInsert(value: string, caret: number, suggestionValue: string): InsertResult {
  const safeCaret = Math.max(0, Math.min(caret, value.length));
  const query = findTemplateQuery(value, safeCaret);
  const inserted = `{{ ${suggestionValue} }}`;
  if (query) {
    const nextValue = `${value.slice(0, query.start)}${inserted}${value.slice(query.replaceEnd)}`;
    const nextCaret = query.start + inserted.length;
    return { value: nextValue, caret: nextCaret };
  }
  const word = /[A-Za-z0-9_$.]*$/.exec(value.slice(0, safeCaret))?.[0] ?? '';
  const start = safeCaret - word.length;
  const nextValue = `${value.slice(0, start)}${inserted}${value.slice(safeCaret)}`;
  return { value: nextValue, caret: start + inserted.length };
}

/**
 * Insert a URL preset: swaps only the `scheme://host` origin, keeping the
 * typed path/query. With no origin yet (empty field, bare `https://`),
 * the preset URL is used as-is. Caret lands at the end.
 */
export function applyPresetInsert(current: string, presetUrl: string): InsertResult {
  const origin = /^https?:\/\/[^/?#\s]*/i.exec(presetUrl.trim())?.[0]?.replace(/\/+$/, '') ?? presetUrl.trim();
  const match = /^https?:\/\/[^/?#\s]*/i.exec(current);
  if (!match) {
    const nextValue = presetUrl;
    return { value: nextValue, caret: nextValue.length };
  }
  const rest = current.slice(match[0].length);
  const nextValue = !rest ? `${origin}/` : /^[/?#]/.test(rest) ? `${origin}${rest}` : `${origin}/${rest}`;
  return { value: nextValue, caret: nextValue.length };
}

/** Insert an option: replaces the word under the caret with the value. */
export function applyOptionInsert(value: string, caret: number, optionValue: string): InsertResult {
  const safeCaret = Math.max(0, Math.min(caret, value.length));
  const word = /[A-Za-z0-9_$.]*$/.exec(value.slice(0, safeCaret))?.[0] ?? '';
  const start = safeCaret - word.length;
  const nextValue = `${value.slice(0, start)}${optionValue}${value.slice(safeCaret)}`;
  return { value: nextValue, caret: start + optionValue.length };
}

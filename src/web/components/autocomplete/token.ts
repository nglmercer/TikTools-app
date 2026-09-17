import type { AutocompleteToken, TemplateQuery } from './types.ts';

/** Finds the single replacement range shared by all autocomplete controls. */
export function getAutocompleteToken(value: string, cursor: number, mode: 'template' | 'path' = 'template'): AutocompleteToken {
  const before = value.slice(0, cursor);
  if (mode === 'template') {
    const start = before.lastIndexOf('{{');
    if (start >= 0 && !before.slice(start).includes('}}')) {
      return { start, query: before.slice(start + 2).trim(), inside: true };
    }
  }
  const match = before.match(/[A-Za-z0-9_$.]*$/);
  const word = match?.[0] ?? '';
  return {
    start: cursor - word.length,
    query: word,
    inside: mode === 'path' || before.lastIndexOf('{{') >= 0 && !before.slice(before.lastIndexOf('{{')).includes('}}'),
  };
}

/**
 * Pure double-brace detector (S8/S9). Returns the template query when —
 * and only when — the caret sits inside an unclosed `{{ …` expression:
 *
 * - no `{{` before the caret, or a `}}` closes it before the caret → null
 * - the caret already passed the closing `}}` → null
 * - the expression text looks like a URL (`://` anywhere) → null, so URL
 *   text never suggests event variables
 *
 * `replaceEnd` extends through a closing `}}` that directly follows the
 * caret (whitespace allowed), so insertion replaces the whole span instead
 * of leaving a stray `}}` behind.
 */
export function findTemplateQuery(value: string, caret: number): TemplateQuery | null {
  const safeCaret = Math.max(0, Math.min(caret, value.length));
  const before = value.slice(0, safeCaret);
  const start = before.lastIndexOf('{{');
  if (start < 0) return null;
  if (before.slice(start).includes('}}')) return null;
  const after = value.slice(safeCaret);
  const closeMatch = /^\s*\}\}/.exec(after);
  // A `}}` later on the same expression still closes it: `{{ a }} |` with
  // the caret after the close is outside, but `lastIndexOf` already found
  // the opener — the `before` check above covers it. What remains: a close
  // ahead on this same line region ends the replace range there.
  const query = before.slice(start + 2).trim();
  if (query.includes('://')) return null;
  if (/^\s*https?:$/i.test(query)) return null;
  return {
    start,
    replaceEnd: closeMatch ? safeCaret + closeMatch[0].length : safeCaret,
    query,
  };
}

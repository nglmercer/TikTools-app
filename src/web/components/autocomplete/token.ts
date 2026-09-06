import type { AutocompleteToken } from './types.ts';

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

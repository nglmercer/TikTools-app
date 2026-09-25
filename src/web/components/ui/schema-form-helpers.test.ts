import { expect, test } from 'bun:test';

import { resolveAutocompleteSources } from './schema-form-helpers.ts';

test('field suggestions merge globals.* rows beside event variables', () => {
  const suggestionsFor = resolveAutocompleteSources({
    locale: 'en',
    globals: { commandPort: '46665' },
  });
  const values = suggestionsFor('url', true).map((item) => item.value);
  expect(values).toContain('globals.commandPort');
});

test('fields without globals resolve event variables only', () => {
  const suggestionsFor = resolveAutocompleteSources({ locale: 'en' });
  const values = suggestionsFor('url', true).map((item) => item.value);
  expect(values.some((value) => value.startsWith('globals.'))).toBe(false);
});

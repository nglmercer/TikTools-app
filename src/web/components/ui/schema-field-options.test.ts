import { expect, test } from 'bun:test';

import { resolveFieldOptions } from './schema-field-options.ts';

test('schema enums win over dynamic and hinted lists', () => {
  const resolved = resolveFieldOptions(
    { type: 'string', enum: ['en', 'es', 42] },
    { options: [{ value: 'fr', label: 'French' }] },
    [{ value: 'dyn', label: 'Dynamic' }],
    'en',
  );
  expect(resolved.source).toBe('schema');
  expect(resolved.options).toEqual([
    { value: 'en', label: 'en' },
    { value: 'es', label: 'es' },
  ]);
});

test('dynamic options win over hinted lists', () => {
  const resolved = resolveFieldOptions(
    { type: 'string' },
    { options: [{ value: 'fr', label: 'French' }] },
    [{ value: 'dyn', label: 'Dynamic' }],
    'en',
  );
  expect(resolved.source).toBe('dynamic');
  expect(resolved.options).toEqual([{ value: 'dyn', label: 'Dynamic' }]);
});

test('hinted entries localize labels and read icons', () => {
  const resolved = resolveFieldOptions(
    { type: 'string' },
    { options: [{ value: 'a', label: { default: 'Alpha' }, hint: 'pick me', icon: 'bolt' }, null, 'junk'] },
    undefined,
    'en',
  );
  expect(resolved.source).toBe('hinted');
  expect(resolved.options).toEqual([{ value: 'a', label: 'Alpha', hint: 'pick me', icon: 'bolt' }]);
});

test('no option source resolves to none', () => {
  expect(resolveFieldOptions({ type: 'string' }, undefined, undefined, 'en')).toEqual({
    options: [],
    source: 'none',
  });
  expect(resolveFieldOptions({ type: 'string' }, {}, [], 'en').source).toBe('none');
});

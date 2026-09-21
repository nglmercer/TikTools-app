import { expect, test } from 'bun:test';

import { formatEditorValue, formatValue, isJsonObject, parseValue } from './config-value.ts';

test('formatValue renders scalars and blanks anything else', () => {
  expect(formatValue('hello')).toBe('hello');
  expect(formatValue(42)).toBe('42');
  expect(formatValue(true)).toBe('true');
  expect(formatValue(undefined)).toBe('');
  expect(formatValue(null)).toBe('');
  expect(formatValue({})).toBe('');
  expect(formatValue([1])).toBe('');
});

test('parseValue coerces booleans and numbers, keeps text', () => {
  expect(parseValue('')).toBe('');
  expect(parseValue('   ')).toBe('');
  expect(parseValue('true')).toBe(true);
  expect(parseValue('false')).toBe(false);
  expect(parseValue('42')).toBe(42);
  expect(parseValue('4.5')).toBe(4.5);
  expect(parseValue('hello')).toBe('hello');
  expect(parseValue(' 12 ')).toBe(12);
});

test('formatEditorValue quotes strings and truncates long payloads', () => {
  expect(formatEditorValue('hi')).toBe('"hi"');
  expect(formatEditorValue(null)).toBe('null');
  expect(formatEditorValue(7)).toBe('7');
  expect(formatEditorValue(false)).toBe('false');
  const long = formatEditorValue({ text: 'x'.repeat(500) });
  expect(long.length).toBeLessThanOrEqual(140);
  expect(long.endsWith('...')).toBe(true);
  const pretty = formatEditorValue({ a: 1 }, true);
  expect(pretty).toContain('\n');
});

test('isJsonObject narrows plain objects only', () => {
  expect(isJsonObject({})).toBe(true);
  expect(isJsonObject({ a: 1 })).toBe(true);
  expect(isJsonObject([])).toBe(false);
  expect(isJsonObject('x')).toBe(false);
  expect(isJsonObject(null)).toBe(false);
  expect(isJsonObject(undefined)).toBe(false);
});

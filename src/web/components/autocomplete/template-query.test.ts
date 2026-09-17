import { expect, test } from 'bun:test';

import { findTemplateQuery } from './token.ts';

test('returns null outside braces (plain text, focus, urls)', () => {
  expect(findTemplateQuery('', 0)).toBeNull();
  expect(findTemplateQuery('hello world', 5)).toBeNull();
  expect(findTemplateQuery('https://example.com/a?x=1', 10)).toBeNull();
  expect(findTemplateQuery('http://localhost:3000/hook', 26)).toBeNull();
  expect(findTemplateQuery('say {{ event.user.nickname }} now', 31)).toBeNull();
});

test('detects the query inside an unclosed expression', () => {
  const query = findTemplateQuery('say {{ event.user.ni', 20);
  expect(query).not.toBeNull();
  expect(query?.start).toBe(4);
  expect(query?.query).toBe('event.user.ni');
  expect(query?.replaceEnd).toBe(20);
});

test('empty braces yield an empty query (invoke shows all)', () => {
  expect(findTemplateQuery('{{ ', 3)).toEqual({ start: 0, replaceEnd: 3, query: '' });
  expect(findTemplateQuery('a {{  ', 6)?.query).toBe('');
});

test('a closed expression before the caret is outside', () => {
  expect(findTemplateQuery('{{ event.user.nickname }}', 25)).toBeNull();
  expect(findTemplateQuery('{{ a }} and {{ b', 16)?.query).toBe('b');
});

test('replaceEnd consumes a directly-following close', () => {
  const query = findTemplateQuery('{{ event.user.ni }}', 17);
  expect(query?.start).toBe(0);
  expect(query?.replaceEnd).toBe(19);
  expect(query?.query).toBe('event.user.ni');
});

test('url-like text inside braces suggests nothing', () => {
  expect(findTemplateQuery('{{ https://example.com }}', 11)).toBeNull();
  expect(findTemplateQuery('{{ http://localhost:3000/x', 26)).toBeNull();
  expect(findTemplateQuery('{{ https:', 9)).toBeNull();
});

test('caret is clamped into range', () => {
  expect(findTemplateQuery('{{ ab', 99)?.query).toBe('ab');
  expect(findTemplateQuery('{{ ab', -5)).toBeNull();
});

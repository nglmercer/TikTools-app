import { expect, test } from 'bun:test';

import { filterByScope } from './autocomplete-controller.ts';
import type { SuggestionItem } from './types.ts';

const item = (value: string, scopes?: SuggestionItem['scopes']): SuggestionItem => ({
  value,
  label: value,
  scopes,
});

const pool = [
  item('event.user.uniqueId', ['identity', 'message']),
  item('event.data.comment', ['message']),
  item('event.intel.comment.language.top', ['message']),
  item('event.universal.note'),
];

test('generic matches every row', () => {
  expect(filterByScope(pool, 'generic').map((entry) => entry.value)).toEqual([
    'event.user.uniqueId',
    'event.data.comment',
    'event.intel.comment.language.top',
    'event.universal.note',
  ]);
});

test('message keeps message rows plus universal rows', () => {
  expect(filterByScope(pool, 'message').map((entry) => entry.value)).toEqual([
    'event.user.uniqueId',
    'event.data.comment',
    'event.intel.comment.language.top',
    'event.universal.note',
  ]);
});

test('identity keeps identity rows plus universal rows', () => {
  expect(filterByScope(pool, 'identity').map((entry) => entry.value)).toEqual([
    'event.user.uniqueId',
    'event.universal.note',
  ]);
});

test('http scopes see universal rows until Phase 3 tags variables', () => {
  expect(filterByScope(pool, 'http-url').map((entry) => entry.value)).toEqual(['event.universal.note']);
  expect(filterByScope(pool, 'http-data').map((entry) => entry.value)).toEqual(['event.universal.note']);
});

test('untagged rows are universal across all scopes', () => {
  const untagged = [item('event.anything')];
  for (const scope of ['message', 'identity', 'http-url', 'http-data', 'generic'] as const) {
    expect(filterByScope(untagged, scope)).toHaveLength(1);
  }
});

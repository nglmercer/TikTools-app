import { expect, test } from 'bun:test';

import { createAutocompleteController } from './autocomplete-controller.ts';
import type { SuggestionItem } from './types.ts';

const variables: SuggestionItem[] = [
  { value: 'event.user.uniqueId', label: 'Unique Id' },
  { value: 'event.user.nickname', label: 'Nickname' },
  { value: 'event.data.comment', label: 'Comment' },
];

function openTemplate() {
  const controller = createAutocompleteController({
    mode: 'template',
    suggestions: variables,
    supportsTemplates: true,
  });
  return controller;
}

test('no trigger on normal text, including URLs', () => {
  const controller = openTemplate();
  for (
    const input of [
      { value: 'hello world', caret: 5 },
      { value: 'https://example.com/a?x=1', caret: 10 },
      { value: 'http://localhost:3000/hook', caret: 26 },
      { value: '', caret: 0 },
    ]
  ) {
    const snapshot = controller.update({ ...input, focused: true });
    expect(snapshot.open).toBe(false);
    expect(snapshot.rowCount).toBe(0);
  }
});

test('trigger right after double braces shows every variable', () => {
  const controller = openTemplate();
  const snapshot = controller.update({ value: '{{ ', caret: 3, focused: true });
  expect(snapshot.open).toBe(true);
  expect(snapshot.query).toBe('');
  expect(snapshot.rowCount).toBe(3);
  expect(snapshot.templateQuery).toEqual({ start: 0, replaceEnd: 3, query: '' });
});

test('trigger inside an existing template filters by the typed query', () => {
  const controller = openTemplate();
  const snapshot = controller.update({ value: 'say {{ event.user.ni', caret: 20, focused: true });
  expect(snapshot.open).toBe(true);
  expect(snapshot.query).toBe('event.user.ni');
  expect(snapshot.rowCount).toBeGreaterThan(0);
  expect(snapshot.sections.flatMap((section) => section.rows).map((row) => row.item.value)).toContain(
    'event.user.nickname',
  );
});

test('closed after the closing braces: caret past `}}` shows nothing', () => {
  const controller = openTemplate();
  const value = 'say {{ event.user.nickname }} now';
  const snapshot = controller.update({ value, caret: 31, focused: true });
  expect(snapshot.open).toBe(false);
  expect(snapshot.templateQuery).toBeNull();
});

test('controller commit consumes a trailing close and preserves the caret', () => {
  const controller = openTemplate();
  const value = 'say {{ event.user.uni }}!';
  controller.update({ value, caret: 21, focused: true });
  expect(controller.snapshot().open).toBe(true);
  const committed = controller.commit(value, 21);
  expect(committed?.value).toBe('say {{ event.user.uniqueId }}!');
  expect(committed?.caret).toBe(4 + '{{ event.user.uniqueId }}'.length);
  expect('say {{ event.user.uniqueId }}!'.slice(committed?.caret ?? 0)).toBe('!');
});

test('keyboard navigation selects which row the commit inserts', () => {
  const controller = openTemplate();
  controller.update({ value: '{{ event', caret: 8, focused: true });
  expect(controller.snapshot().open).toBe(true);
  controller.key('ArrowDown');
  expect(controller.snapshot().activeIndex).toBe(1);
  const pending = controller.pendingRow();
  expect(pending?.item.value).toBe('event.user.nickname');
  const committed = controller.commit('{{ event', 8);
  expect(committed?.value).toBe('{{ event.user.nickname }}');
});

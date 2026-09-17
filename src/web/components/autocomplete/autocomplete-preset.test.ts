import { expect, test } from 'bun:test';

import { createAutocompleteController, type PresetItem } from './autocomplete-controller.ts';
import type { SuggestionItem } from './types.ts';

const presets: PresetItem[] = [
  { id: 'local-node', label: 'localhost:3000', url: 'http://localhost:3000/' },
  { id: 'local-py', label: '127.0.0.1:8000', url: 'http://127.0.0.1:8000/' },
];

const variables: SuggestionItem[] = [
  { value: 'event.user.uniqueId', label: 'Unique Id' },
  { value: 'event.data.comment', label: 'Comment' },
];

function openPreset() {
  const controller = createAutocompleteController({ mode: 'preset', presets });
  controller.update({ value: '', caret: 0, focused: true });
  return controller;
}

test('preset mode never shows template variables, even with a suggestions pool', () => {
  const controller = createAutocompleteController({ mode: 'preset', presets, suggestions: variables });
  // A matching query returns preset rows only — the suggestions pool is ignored.
  const matched = controller.update({ value: 'http', caret: 4, focused: true });
  expect(matched.open).toBe(true);
  expect(matched.rowCount).toBeGreaterThan(0);
  for (const row of matched.sections.flatMap((section) => section.rows)) {
    expect(row.item.value.startsWith('event.')).toBe(false);
    expect(row.item.value.startsWith('{{')).toBe(false);
    expect(row.item.badge).toBe('PRESET');
  }
  // Braces are literal text in preset mode: no variable rows, no crash.
  const braced = controller.update({ value: '{{ event', caret: 8, focused: true });
  for (const row of braced.sections.flatMap((section) => section.rows)) {
    expect(row.item.value.startsWith('event.')).toBe(false);
  }
});

test('preset keyboard navigation + commit inserts the highlighted preset', () => {
  const controller = openPreset();
  expect(controller.snapshot().rowCount).toBe(2);
  controller.key('ArrowDown');
  expect(controller.snapshot().activeIndex).toBe(1);
  const pending = controller.pendingRow();
  expect(pending?.item.value).toBe('http://127.0.0.1:8000/');
  const committed = controller.commit('', 0);
  expect(committed?.value).toBe('http://127.0.0.1:8000/');
  expect(committed?.caret).toBe(committed?.value.length);
  expect(committed?.row.item.badge).toBe('PRESET');
});

test('preset commit on typed text swaps the origin and keeps the path', () => {
  const controller = createAutocompleteController({ mode: 'preset', presets });
  controller.update({ value: 'http://local', caret: 12, focused: true });
  expect(controller.snapshot().open).toBe(true);
  controller.key('Home');
  const committed = controller.commit('http://local', 12);
  expect(committed?.value).toBe('http://localhost:3000/');
});

test('Escape dismisses presets and stays shut until the input changes', () => {
  const controller = openPreset();
  expect(controller.key('Escape')).toBe('dismissed');
  expect(controller.snapshot().open).toBe(false);
  controller.update({ value: '', caret: 0, focused: true });
  expect(controller.snapshot().open).toBe(false);
  const reopened = controller.update({ value: 'l', caret: 1, focused: true });
  expect(reopened.open).toBe(true);
});

test('explicit invoke with zero matches stays closed', () => {
  const controller = createAutocompleteController({ mode: 'preset', presets });
  controller.update({ value: 'zzz-no-match', caret: 12, focused: true });
  expect(controller.snapshot().open).toBe(false);
  controller.invoke();
  expect(controller.snapshot().open).toBe(false);
});

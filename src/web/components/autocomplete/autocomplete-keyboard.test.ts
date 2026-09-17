import { expect, test } from 'bun:test';

import {
  createAutocompleteController,
  isExplicitInvokeKey,
  resolveAutocompleteKey,
} from './autocomplete-controller.ts';
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
  controller.update({ value: '{{ event', caret: 8, focused: true });
  return controller;
}

test('every listbox key maps; other keys are untouched', () => {
  expect(resolveAutocompleteKey('ArrowDown')).toBe('next');
  expect(resolveAutocompleteKey('ArrowUp')).toBe('prev');
  expect(resolveAutocompleteKey('Home')).toBe('first');
  expect(resolveAutocompleteKey('End')).toBe('last');
  expect(resolveAutocompleteKey('Enter')).toBe('commit');
  expect(resolveAutocompleteKey('Tab')).toBe('commit');
  expect(resolveAutocompleteKey('Escape')).toBe('dismiss');
  for (const key of ['a', ' ', 'Backspace', 'ArrowLeft', 'ArrowRight', 'F1']) {
    expect(resolveAutocompleteKey(key)).toBeNull();
  }
});

test('only Ctrl/Cmd+Space is an explicit invoke', () => {
  expect(isExplicitInvokeKey({ key: ' ', ctrlKey: true, metaKey: false })).toBe(true);
  expect(isExplicitInvokeKey({ key: ' ', ctrlKey: false, metaKey: true })).toBe(true);
  expect(isExplicitInvokeKey({ key: ' ', ctrlKey: false, metaKey: false })).toBe(false);
  expect(isExplicitInvokeKey({ key: 'Enter', ctrlKey: true, metaKey: false })).toBe(false);
});

test('Up/Down wrap, Home/End jump', () => {
  const controller = openTemplate();
  expect(controller.snapshot().activeIndex).toBe(0);
  controller.key('ArrowDown');
  expect(controller.snapshot().activeIndex).toBe(1);
  controller.key('End');
  const last = controller.snapshot().rowCount - 1;
  expect(controller.snapshot().activeIndex).toBe(last);
  controller.key('ArrowDown');
  expect(controller.snapshot().activeIndex).toBe(0);
  controller.key('ArrowUp');
  expect(controller.snapshot().activeIndex).toBe(last);
  controller.key('Home');
  expect(controller.snapshot().activeIndex).toBe(0);
});

test('Enter/Tab request commit; Escape dismisses and stays shut until input changes', () => {
  const controller = openTemplate();
  expect(controller.snapshot().open).toBe(true);
  expect(controller.key('Enter')).toBe('commit');
  expect(controller.key('Tab')).toBe('commit');
  expect(controller.key('Escape')).toBe('dismissed');
  // Same value/caret/focus: still closed (focus stays on the input — the
  // controller never blurs; the latch reopens only on new input).
  expect(controller.snapshot().open).toBe(false);
  controller.update({ value: '{{ event', caret: 8, focused: true });
  expect(controller.snapshot().open).toBe(false);
  controller.update({ value: '{{ event.', caret: 9, focused: true });
  expect(controller.snapshot().open).toBe(true);
});

test('keys are inert while closed', () => {
  const controller = createAutocompleteController({
    mode: 'template',
    suggestions: variables,
    supportsTemplates: true,
  });
  controller.update({ value: 'plain text', caret: 10, focused: true });
  expect(controller.snapshot().open).toBe(false);
  expect(controller.key('ArrowDown')).toBeNull();
  expect(controller.key('Enter')).toBeNull();
  expect(controller.key('Escape')).toBeNull();
});

test('explicit invoke opens template rows outside braces', () => {
  const controller = createAutocompleteController({
    mode: 'template',
    suggestions: variables,
    supportsTemplates: true,
  });
  controller.update({ value: 'plain text', caret: 10, focused: true });
  expect(controller.snapshot().open).toBe(false);
  controller.invoke();
  const snapshot = controller.snapshot();
  expect(snapshot.open).toBe(true);
  expect(snapshot.rowCount).toBe(3);
});

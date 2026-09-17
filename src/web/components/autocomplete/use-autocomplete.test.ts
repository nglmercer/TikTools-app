import { expect, test } from 'bun:test';

import {
  createAutocompleteController,
  type AutocompleteControllerOptions,
} from './autocomplete-controller.ts';
import { createFallbackAutocompleteController } from './use-autocomplete.ts';
import type { SuggestionItem } from './types.ts';

const VARS: SuggestionItem[] = [
  { value: 'event.data.comment', label: 'Comment', kind: 'string' },
  { value: 'event.user.uniqueId', label: 'Unique Id', kind: 'string' },
];

function templateOptions(overrides: Partial<AutocompleteControllerOptions> = {}): AutocompleteControllerOptions {
  return { mode: 'template', suggestions: VARS, supportsTemplates: true, ...overrides };
}

function presetOptions(overrides: Partial<AutocompleteControllerOptions> = {}): AutocompleteControllerOptions {
  return {
    mode: 'preset',
    presets: [
      { id: 'local', label: 'localhost:3000', url: 'http://localhost:3000/' },
      { id: 'py', label: '127.0.0.1:8000', url: 'http://127.0.0.1:8000/' },
    ],
    ...overrides,
  };
}

test('fallback controller shows presets outside braces, variables inside', () => {
  const controller = createFallbackAutocompleteController(
    createAutocompleteController(templateOptions()),
    createAutocompleteController(presetOptions()),
  );
  const outside = controller.update({ value: 'http://local', caret: 12, focused: true });
  expect(outside.open).toBe(true);
  expect(outside.mode).toBe('preset');
  expect(outside.sections[0]?.rows.every((row) => row.item.badge === 'PRESET')).toBe(true);

  const inside = controller.update({ value: '{{ event.us', caret: 11, focused: true });
  expect(inside.open).toBe(true);
  expect(inside.mode).toBe('template');
  expect(inside.sections.flatMap((section) => section.rows).some((row) => row.item.value === 'event.user.uniqueId')).toBe(
    true,
  );
});

test('fallback controller never mixes presets and variables in one list', () => {
  const controller = createFallbackAutocompleteController(
    createAutocompleteController(templateOptions()),
    createAutocompleteController(presetOptions()),
  );
  for (
    const input of [
      { value: '', caret: 0, focused: true },
      { value: 'http://localhost:3000/api', caret: 26, focused: true },
      { value: '{{ comment', caret: 10, focused: true },
    ]
  ) {
    const snapshot = controller.update(input);
    const badges = new Set(
      snapshot.sections.flatMap((section) => section.rows).map((row) => row.item.badge ?? row.item.kind ?? ''),
    );
    expect(badges.has('PRESET') && snapshot.mode === 'template').toBe(false);
  }
});

test('fallback explicit invoke targets the template controller', () => {
  const controller = createFallbackAutocompleteController(
    createAutocompleteController(templateOptions()),
    createAutocompleteController(presetOptions()),
  );
  controller.update({ value: 'http://local', caret: 12, focused: true });
  const invoked = controller.invoke();
  expect(invoked.mode).toBe('template');
  expect(invoked.open).toBe(true);
});

test('fallback commit routes to the active controller', () => {
  const controller = createFallbackAutocompleteController(
    createAutocompleteController(templateOptions()),
    createAutocompleteController(presetOptions()),
  );
  controller.update({ value: 'http://local', caret: 12, focused: true });
  const presetCommit = controller.commit('http://local', 12);
  expect(presetCommit?.value).toBe('http://localhost:3000/');
  expect(presetCommit?.row.item.badge).toBe('PRESET');

  controller.update({ value: '{{ event.us', caret: 11, focused: true });
  const templateCommit = controller.commit('{{ event.us', 11);
  expect(templateCommit?.value).toBe('{{ event.user.uniqueId }}');
});

test('fallback dismiss closes both controllers without reopening', () => {
  const controller = createFallbackAutocompleteController(
    createAutocompleteController(templateOptions()),
    createAutocompleteController(presetOptions()),
  );
  controller.update({ value: '', caret: 0, focused: true });
  expect(controller.snapshot().open).toBe(true);
  controller.dismiss();
  expect(controller.snapshot().open).toBe(false);
});

test('fallback keyboard routes to the active list', () => {
  const controller = createFallbackAutocompleteController(
    createAutocompleteController(templateOptions()),
    createAutocompleteController(presetOptions()),
  );
  controller.update({ value: '', caret: 0, focused: true });
  expect(controller.key('ArrowDown')).toBeNull();
  const pending = controller.pendingRow();
  expect(pending?.item.badge).toBe('PRESET');
  expect(controller.key('Escape')).toBe('dismissed');
  expect(controller.snapshot().open).toBe(false);
});

test('controller pools stay live when the options holder is mutated', () => {
  // The Vue glue assigns fresh pools into one holder instead of recreating
  // controllers per render; the controllers must read pools on every compute.
  const holder: AutocompleteControllerOptions = { mode: 'template', suggestions: [], supportsTemplates: true };
  const controller = createAutocompleteController(holder);
  expect(controller.update({ value: '{{ com', caret: 6, focused: true }).open).toBe(false);
  holder.suggestions = VARS;
  const reopened = controller.update({ value: '{{ com', caret: 7, focused: true });
  expect(reopened.open).toBe(true);
  expect(reopened.sections.flatMap((section) => section.rows).some((row) => row.item.value === 'event.data.comment'))
    .toBe(true);
});

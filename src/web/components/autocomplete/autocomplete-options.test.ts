import { expect, test } from 'bun:test';

import {
  createAutocompleteController,
  shouldOpenOptions,
} from './autocomplete-controller.ts';
import type { SuggestionItem } from './types.ts';

const option = (value: string, label?: string): SuggestionItem => ({ value, label: label ?? value });

const options: SuggestionItem[] = [
  option('Alpha', 'Alpha'),
  option('Beta', 'Beta'),
  option('Gamma', 'Gamma'),
];

test('options gate opens on focus-empty, match, or explicit invoke', () => {
  expect(shouldOpenOptions({ focused: true, query: '', matchCount: 3 })).toBe(true);
  expect(shouldOpenOptions({ focused: true, query: '', matchCount: 0 })).toBe(true);
  expect(shouldOpenOptions({ focused: true, query: '', matchCount: 3, openOnFocus: false })).toBe(false);
  expect(shouldOpenOptions({ focused: true, query: 'alp', matchCount: 1 })).toBe(true);
  expect(shouldOpenOptions({ focused: true, query: 'zzz', matchCount: 0 })).toBe(false);
  expect(shouldOpenOptions({ focused: true, query: 'zzz', matchCount: 0, explicitInvoke: true })).toBe(true);
  expect(shouldOpenOptions({ focused: false, query: '', matchCount: 3 })).toBe(false);
});

test('controller: options mode opens on focus-empty with stable option keys', () => {
  const controller = createAutocompleteController({ mode: 'options', options });
  const snapshot = controller.update({ value: '', caret: 0, focused: true });
  expect(snapshot.open).toBe(true);
  expect(snapshot.rowCount).toBe(3);
  expect(snapshot.sections).toHaveLength(1);
  const keys = snapshot.sections.flatMap((section) => section.rows.map((row) => row.key));
  expect(keys).toEqual(['opt:Alpha', 'opt:Beta', 'opt:Gamma']);
  const filtered = controller.update({ value: 'alp', caret: 3, focused: true });
  expect(filtered.rowCount).toBe(1);
  expect(filtered.sections[0]?.rows[0]?.key).toBe('opt:Alpha');
});

test('controller: options mode stays closed without matches unless invoked', () => {
  const controller = createAutocompleteController({ mode: 'options', options });
  expect(controller.update({ value: 'zzz-no-match', caret: 12, focused: true }).open).toBe(false);
  expect(controller.update({ value: '', caret: 0, focused: true }).open).toBe(true);
  const empty = createAutocompleteController({ mode: 'options', options: [] });
  expect(empty.update({ value: '', caret: 0, focused: true }).open).toBe(false);
});

test('controller: options commit replaces the word under the caret', () => {
  const controller = createAutocompleteController({ mode: 'options', options });
  controller.update({ value: 'alp', caret: 3, focused: true });
  const committed = controller.commit('alp', 3);
  expect(committed?.value).toBe('Alpha');
  expect(committed?.caret).toBe('Alpha'.length);
  expect(committed?.row.key).toBe('opt:Alpha');
});

test('controller: options mode honors openOptionsOnFocus and dismissal', () => {
  const lazy = createAutocompleteController({ mode: 'options', options, openOptionsOnFocus: false });
  expect(lazy.update({ value: '', caret: 0, focused: true }).open).toBe(false);
  expect(lazy.update({ value: 'alp', caret: 3, focused: true }).open).toBe(true);

  const controller = createAutocompleteController({ mode: 'options', options });
  controller.update({ value: '', caret: 0, focused: true });
  const dismissed = controller.dismiss();
  expect(dismissed.open).toBe(false);
  expect(controller.update({ value: '', caret: 0, focused: true }).open).toBe(false);
  expect(controller.invoke().open).toBe(true);
});

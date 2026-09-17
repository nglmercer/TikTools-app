import { expect, test } from 'bun:test';

import {
  applyOptionInsert,
  applyPresetInsert,
  applyTemplateInsert,
  clampActiveIndex,
  comboboxInputAttrs,
  createAutocompleteController,
  filterOptionRows,
  filterPresetRows,
  filterTemplateRows,
  groupForTemplatePath,
  groupTemplateRows,
  iconForSuggestion,
  moveActiveIndex,
  shouldOpenPreset,
  shouldOpenTemplate,
  type PresetItem,
} from './autocomplete-controller.ts';
import type { SuggestionItem } from './types.ts';

const variable = (value: string, label?: string): SuggestionItem => ({ value, label: label ?? value });

const variables: SuggestionItem[] = [
  variable('event.user.uniqueId', 'Unique Id'),
  variable('event.user.nickname', 'Nickname'),
  variable('event.data.comment', 'Comment'),
  variable('event.intel.comment.tts.text', 'TTS Text'),
];

const presets: PresetItem[] = [
  { id: 'local-node', label: 'localhost:3000', url: 'http://localhost:3000/', hint: 'Local dev server' },
  { id: 'remote', label: 'https://', url: 'https://', hint: 'Public webhook' },
];

test('template rows filter by query and group into sections', () => {
  const rows = filterTemplateRows(variables, 'unique', 'generic');
  expect(rows.map((row) => row.item.value)).toEqual(['event.user.uniqueId']);
  const grouped = groupTemplateRows(filterTemplateRows(variables, '', 'generic'));
  expect(grouped.map((section) => section.label)).toEqual(['User', 'Message', 'Text Intelligence']);
  expect(grouped.flatMap((section) => section.rows)).toHaveLength(4);
});

test('template grouping maps identity / intel paths', () => {
  expect(groupForTemplatePath('event.user.uniqueId')).toBe('User');
  expect(groupForTemplatePath('event.intel.comment.tts.text')).toBe('Text Intelligence');
  expect(groupForTemplatePath('event.data.comment')).toBe('Message');
});

test('preset rows carry zero event variables and PRESET badges', () => {
  const rows = filterPresetRows(presets, 'local');
  expect(rows.map((row) => row.item.value)).toEqual(['http://localhost:3000/']);
  for (const row of rows) {
    expect(row.item.value.startsWith('event.')).toBe(false);
    expect(row.item.badge).toBe('PRESET');
    expect(row.item.icon).toBe('globe');
  }
  expect(filterPresetRows(presets, 'event.user')).toHaveLength(0);
});

test('options rows match dynamic lists by label or value', () => {
  const options = [variable('opt-a', 'Alpha'), variable('opt-b', 'Beta')];
  expect(filterOptionRows(options, 'alp').map((row) => row.item.value)).toEqual(['opt-a']);
  expect(filterOptionRows(options, '').map((row) => row.item.value)).toEqual(['opt-a', 'opt-b']);
});

test('template gate opens only inside braces (or explicit invoke)', () => {
  expect(shouldOpenTemplate({ supportsTemplates: true, focused: true, query: { start: 0, replaceEnd: 5, query: '' } })).toBe(true);
  expect(shouldOpenTemplate({ supportsTemplates: true, focused: true, query: null })).toBe(false);
  expect(shouldOpenTemplate({ supportsTemplates: true, focused: true, query: null, explicitInvoke: true })).toBe(true);
  expect(shouldOpenTemplate({ supportsTemplates: false, focused: true, query: { start: 0, replaceEnd: 5, query: '' } })).toBe(false);
  expect(shouldOpenTemplate({ supportsTemplates: true, focused: false, query: { start: 0, replaceEnd: 5, query: '' } })).toBe(false);
});

test('preset gate opens on focus-empty, match, or explicit invoke', () => {
  expect(shouldOpenPreset({ focused: true, query: '', matchCount: 2 })).toBe(true);
  expect(shouldOpenPreset({ focused: true, query: 'local', matchCount: 1 })).toBe(true);
  expect(shouldOpenPreset({ focused: true, query: 'zzz', matchCount: 0 })).toBe(false);
  expect(shouldOpenPreset({ focused: true, query: 'zzz', matchCount: 0, explicitInvoke: true })).toBe(true);
  expect(shouldOpenPreset({ focused: false, query: '', matchCount: 2 })).toBe(false);
});

test('active index wraps on Up/Down and clamps on hover/Home/End', () => {
  expect(moveActiveIndex(0, 1, 3)).toBe(1);
  expect(moveActiveIndex(2, 1, 3)).toBe(0);
  expect(moveActiveIndex(0, -1, 3)).toBe(2);
  expect(moveActiveIndex(0, 1, 0)).toBe(0);
  expect(clampActiveIndex(99, 3)).toBe(2);
  expect(clampActiveIndex(-4, 3)).toBe(0);
  expect(clampActiveIndex(0, 0)).toBe(0);
});

test('template insert replaces the brace span and preserves the caret', () => {
  const result = applyTemplateInsert('say {{ event.user.ni }}!', 21, 'event.user.uniqueId');
  expect(result.value).toBe('say {{ event.user.uniqueId }}!');
  expect(result.caret).toBe(4 + '{{ event.user.uniqueId }}'.length);
  expect('say {{ event.user.uniqueId }}!'.slice(result.caret)).toBe('!');
});

test('template insert outside braces wraps the word at the caret', () => {
  const result = applyTemplateInsert('hello event.us', 14, 'event.user.uniqueId');
  expect(result.value).toBe('hello {{ event.user.uniqueId }}');
  expect(result.caret).toBe(result.value.length);
});

test('preset insert swaps the origin and keeps path/query', () => {
  expect(applyPresetInsert('https://old.example.com/a?x=1', 'http://localhost:3000/').value)
    .toBe('http://localhost:3000/a?x=1');
  expect(applyPresetInsert('', 'http://localhost:3000/').value).toBe('http://localhost:3000/');
  const result = applyPresetInsert('https://', 'http://localhost:3000/');
  expect(result.value).toBe('http://localhost:3000/');
  expect(result.caret).toBe(result.value.length);
});

test('option insert replaces the word under the caret', () => {
  const result = applyOptionInsert('pick alp', 8, 'Alpha');
  expect(result.value).toBe('pick Alpha');
  expect(result.caret).toBe(10);
});

test('icons are whitelisted semantic names only', () => {
  expect(iconForSuggestion({ value: 'event.user.uniqueId', kind: 'string' })).toBe('users');
  expect(iconForSuggestion({ value: 'event.intel.comment.tts.text', kind: 'string' })).toBe('sparkles');
  expect(iconForSuggestion({ value: 'event.data.comment', kind: 'string' })).toBe('chat');
  expect(iconForSuggestion({ value: 'http://localhost:3000/', kind: 'snippet', detail: 'preset' })).toBe('globe');
  expect(iconForSuggestion({ value: 'event.data.count', kind: 'number' })).toBe('stats');
  expect(iconForSuggestion({ value: 'mystery', kind: 'unknown' })).toBe('dot');
});

test('combobox attrs expose listbox wiring for Phase 3 inputs', () => {
  const open = comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: 2, rowCount: 5, describedBy: 'ac-1-hint' });
  expect(open.role).toBe('combobox');
  expect(open['aria-expanded']).toBe(true);
  expect(open['aria-controls']).toBe('ac-1');
  expect(open['aria-activedescendant']).toBe('ac-1-option-2');
  expect(open['aria-autocomplete']).toBe('list');
  expect(open['aria-describedby']).toBe('ac-1-hint');
  const closed = comboboxInputAttrs({ listId: 'ac-1', open: false, activeIndex: 0, rowCount: 0, invalid: true, errorId: 'ac-1-err' });
  expect(closed['aria-expanded']).toBe(false);
  expect(closed['aria-controls']).toBeUndefined();
  expect(closed['aria-activedescendant']).toBeUndefined();
  expect(closed['aria-invalid']).toBe(true);
  expect(closed['aria-errormessage']).toBe('ac-1-err');
});

test('controller: template mode opens in braces, commits caret-preserving text', () => {
  const controller = createAutocompleteController({
    mode: 'template',
    suggestions: variables,
    supportsTemplates: true,
  });
  // Plain focus outside braces: closed, no event vars.
  let snapshot = controller.update({ value: 'hello', caret: 5, focused: true });
  expect(snapshot.open).toBe(false);
  expect(snapshot.rowCount).toBe(0);
  // Inside braces: open with grouped rows.
  snapshot = controller.update({ value: 'say {{ event.user.uni', caret: 21, focused: true });
  expect(snapshot.open).toBe(true);
  expect(snapshot.query).toBe('event.user.uni');
  expect(snapshot.sections.map((section) => section.label)).toEqual(['User']);
  // Hover sync moves the active index.
  snapshot = controller.hover(0);
  expect(snapshot.activeIndex).toBe(0);
  const committed = controller.commit('say {{ event.user.uni', 21);
  expect(committed?.value).toBe('say {{ event.user.uniqueId }}');
  expect(committed?.caret).toBe(committed?.value.length);
});

test('controller: generic inputs never open template rows', () => {
  const controller = createAutocompleteController({ mode: 'template', suggestions: variables });
  const snapshot = controller.update({ value: '{{ event', caret: 8, focused: true });
  expect(snapshot.open).toBe(false);
});

test('controller: preset mode opens on focus-empty with URL rows only', () => {
  const controller = createAutocompleteController({ mode: 'preset', presets });
  const snapshot = controller.update({ value: '', caret: 0, focused: true });
  expect(snapshot.open).toBe(true);
  expect(snapshot.rowCount).toBe(2);
  expect(snapshot.sections[0]?.label).toBe('Quick destinations');
  for (const row of snapshot.sections.flatMap((section) => section.rows)) {
    expect(row.item.value.startsWith('event.')).toBe(false);
  }
  const filtered = controller.update({ value: 'local', caret: 5, focused: true });
  expect(filtered.rowCount).toBe(1);
  const none = controller.update({ value: 'zzz-no-match', caret: 12, focused: true });
  expect(none.open).toBe(false);
});

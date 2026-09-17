import { expect, test } from 'bun:test';

import { comboboxInputAttrs } from './autocomplete-controller.ts';
import {
  flattenSuggestionSections,
  suggestionOptionId,
  type SuggestionSection,
} from './types.ts';

test('activedescendant clamps an out-of-range active index', () => {
  const attrs = comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: 99, rowCount: 3 });
  expect(attrs['aria-activedescendant']).toBe('ac-1-option-2');
});

test('no activedescendant when closed, empty, or unselected', () => {
  expect(
    comboboxInputAttrs({ listId: 'ac-1', open: false, activeIndex: 0, rowCount: 3 })['aria-activedescendant'],
  ).toBeUndefined();
  expect(
    comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: 0, rowCount: 0 })['aria-activedescendant'],
  ).toBeUndefined();
  expect(
    comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: -1, rowCount: 3 })['aria-activedescendant'],
  ).toBeUndefined();
});

test('option ids match the combobox activedescendant target', () => {
  expect(suggestionOptionId('ac-1', 2)).toBe('ac-1-option-2');
  const attrs = comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: 2, rowCount: 5 });
  expect(attrs['aria-activedescendant']).toBe(suggestionOptionId('ac-1', 2));
  expect(attrs['aria-controls']).toBe('ac-1');
});

test('closed inputs drop the listbox reference but keep error wiring', () => {
  const attrs = comboboxInputAttrs({
    listId: 'ac-1',
    open: false,
    activeIndex: 0,
    rowCount: 0,
    describedBy: 'ac-1-description ac-1-error',
    invalid: true,
    errorId: 'ac-1-error',
  });
  expect(attrs['aria-controls']).toBeUndefined();
  expect(attrs['aria-activedescendant']).toBeUndefined();
  expect(attrs['aria-describedby']).toBe('ac-1-description ac-1-error');
  expect(attrs['aria-errormessage']).toBe('ac-1-error');
});

test('errormessage appears only when invalid with an error id', () => {
  expect(
    comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: 0, rowCount: 1, invalid: true })[
      'aria-errormessage'
    ],
  ).toBeUndefined();
  expect(
    comboboxInputAttrs({ listId: 'ac-1', open: true, activeIndex: 0, rowCount: 1, errorId: 'ac-1-error' })[
      'aria-errormessage'
    ],
  ).toBeUndefined();
});

test('flattened sections share one keyboard/aria index space', () => {
  const sections: SuggestionSection[] = [
    {
      id: 'a',
      label: 'A',
      rows: [
        { key: 'a:1', item: { value: 'a1', label: 'A1' }, ranges: [] },
        { key: 'a:2', item: { value: 'a2', label: 'A2' }, ranges: [] },
      ],
    },
    { id: 'b', label: 'B', rows: [{ key: 'b:1', item: { value: 'b1', label: 'B1' }, ranges: [] }] },
  ];
  const flat = flattenSuggestionSections(sections);
  expect(flat.map((entry) => entry.globalIndex)).toEqual([0, 1, 2]);
  expect(flat.map((entry) => entry.sectionIndex)).toEqual([0, 0, 1]);
  expect(flat.map((entry) => entry.row.item.value)).toEqual(['a1', 'a2', 'b1']);
  expect(flattenSuggestionSections([])).toEqual([]);
});

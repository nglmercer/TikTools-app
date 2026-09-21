import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import * as barrel from './autocomplete-controller.ts';
import { groupForTemplatePath, TEMPLATE_GROUP_ORDER } from './autocomplete/groups.ts';
import { iconForSuggestion } from './autocomplete/icons.ts';
import type {
  AutocompleteController,
  AutocompleteControllerOptions,
  AutocompleteControllerSnapshot,
  AutocompleteKeyAction,
  AutocompleteLabels,
  AutocompleteMode,
  ComboboxInputAttrs,
  InsertResult,
  PresetItem,
  SuggestionItem,
  SuggestionRow,
  SuggestionScope,
  SuggestionSection,
  TemplateGroupName,
  TemplateQuery,
} from './autocomplete-controller.ts';

/**
 * The compatibility barrel must keep exporting the exact pre-split
 * `autocomplete-controller.ts` API (23 values + 15 types). Value parity is
 * checked at runtime below; every type import above is referenced by
 * `BarrelTypeParity`, so removing a type export fails `bun run typecheck`.
 */
export type BarrelTypeParity = {
  action: AutocompleteKeyAction;
  attrs: ComboboxInputAttrs;
  controller: AutocompleteController;
  insert: InsertResult;
  labels: AutocompleteLabels;
  mode: AutocompleteMode;
  options: AutocompleteControllerOptions;
  preset: PresetItem;
  row: SuggestionRow;
  section: SuggestionSection;
  snapshot: AutocompleteControllerSnapshot;
  suggestion: SuggestionItem;
  scope: SuggestionScope;
  group: TemplateGroupName;
  query: TemplateQuery;
};

const EXPECTED_VALUES = [
  'TEMPLATE_GROUP_ORDER',
  'applyOptionInsert',
  'applyPresetInsert',
  'applyTemplateInsert',
  'clampActiveIndex',
  'comboboxInputAttrs',
  'createAutocompleteController',
  'defaultAutocompleteLabels',
  'filterByScope',
  'filterOptionRows',
  'filterPresetRows',
  'filterTemplateRows',
  'findTemplateQuery',
  'groupForTemplatePath',
  'groupTemplateRows',
  'iconForSuggestion',
  'isExplicitInvokeKey',
  'moveActiveIndex',
  'resolveAutocompleteKey',
  'shouldOpenOptions',
  'shouldOpenPreset',
  'shouldOpenTemplate',
  'toPresetSections',
];

const EXPECTED_TYPES = [
  'AutocompleteController',
  'AutocompleteControllerOptions',
  'AutocompleteControllerSnapshot',
  'AutocompleteKeyAction',
  'AutocompleteLabels',
  'AutocompleteMode',
  'ComboboxInputAttrs',
  'InsertResult',
  'PresetItem',
  'SuggestionItem',
  'SuggestionRow',
  'SuggestionScope',
  'SuggestionSection',
  'TemplateGroupName',
  'TemplateQuery',
];

/** Type names re-exported via `export type {…}` or inline `type X` items. */
function typeExportsOf(source: string): string[] {
  const names: string[] = [];
  for (const block of source.matchAll(/^export\s+(type\s+)?\{([^}]*)\}/gm)) {
    const wholeBlockIsType = (block[1] ?? '').trim() === 'type';
    for (const part of (block[2] ?? '').split(',')) {
      const trimmed = part.trim();
      if (!trimmed) continue;
      const isType = wholeBlockIsType || trimmed.startsWith('type ');
      if (!isType) continue;
      const name = trimmed.split(/\s+as\s+/).pop()?.replace(/^type\s+/, '').trim() ?? '';
      if (name) names.push(name);
    }
  }
  return names.sort();
}

test('barrel exports the exact pre-split value API', () => {
  expect(Object.keys(barrel).sort()).toEqual([...EXPECTED_VALUES].sort());
});

test('barrel re-exports the exact pre-split type API', () => {
  const source = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'autocomplete-controller.ts'), 'utf8');
  expect(typeExportsOf(source)).toEqual([...EXPECTED_TYPES].sort());
});

test('moved group/icon exports are the same bindings (no barrel drift)', () => {
  expect(barrel.groupForTemplatePath).toBe(groupForTemplatePath);
  expect(barrel.TEMPLATE_GROUP_ORDER).toBe(TEMPLATE_GROUP_ORDER);
  expect(barrel.iconForSuggestion).toBe(iconForSuggestion);
});

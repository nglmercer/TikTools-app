/** Compatibility barrel for the autocomplete controller.
 *
 * The implementation moved into `./autocomplete/` (one concern per module);
 * every export below preserves the original `autocomplete-controller.ts`
 * public API, so existing importers keep working unchanged.
 */
export { findTemplateQuery } from './token.ts';
export type {
  AutocompleteMode,
  SuggestionItem,
  SuggestionRow,
  SuggestionScope,
  SuggestionSection,
  TemplateQuery,
} from './types.ts';
export {
  filterByScope,
  shouldOpenOptions,
  shouldOpenPreset,
  shouldOpenTemplate,
} from './autocomplete/gates.ts';
export { iconForSuggestion } from './autocomplete/icons.ts';
export { groupForTemplatePath, TEMPLATE_GROUP_ORDER } from './autocomplete/groups.ts';
export type { TemplateGroupName } from './autocomplete/groups.ts';
export {
  clampActiveIndex,
  isExplicitInvokeKey,
  moveActiveIndex,
  resolveAutocompleteKey,
} from './autocomplete/keyboard.ts';
export type { AutocompleteKeyAction } from './autocomplete/keyboard.ts';
export {
  applyOptionInsert,
  applyPresetInsert,
  applyTemplateInsert,
} from './autocomplete/insertion.ts';
export type { InsertResult } from './autocomplete/insertion.ts';
export { comboboxInputAttrs } from './autocomplete/aria.ts';
export type { ComboboxInputAttrs } from './autocomplete/aria.ts';
export {
  defaultAutocompleteLabels,
  filterOptionRows,
  filterPresetRows,
  filterTemplateRows,
  groupTemplateRows,
  toPresetSections,
} from './autocomplete/rows.ts';
export type {
  AutocompleteLabels,
  PresetItem,
} from './autocomplete/rows.ts';
export { createAutocompleteController } from './autocomplete/state-machine.ts';
export type {
  AutocompleteController,
  AutocompleteControllerOptions,
  AutocompleteControllerSnapshot,
} from './autocomplete/state-machine.ts';

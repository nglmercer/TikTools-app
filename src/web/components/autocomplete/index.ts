export * from './autocomplete.ts';
export * from './controller.ts';
export * from './scoring.ts';
export * from './token.ts';
// Explicit (not star) re-export: `export *` from both here and
// `./autocomplete.ts` would merge the same row-type names ambiguously
// (TS2308), so the canonical subsystem names are listed explicitly.
export {
  SUGGESTION_SCOPES,
  flattenSuggestionSections,
  suggestionOptionId,
  type AutocompleteMode,
  type AutocompleteRow,
  type AutocompleteToken,
  type FlattenedSuggestionRow,
  type SuggestionItem,
  type SuggestionRow,
  type SuggestionScope,
  type SuggestionSection,
  type TemplateQuery,
} from './types.ts';
export * from './autocomplete-controller.ts';
export * from './autocomplete-position.ts';
// NOTE: the .vue components (AutocompletePopover/List/Section/Item) are
// intentionally NOT re-exported here. `AutocompleteItem` already names the
// row type above, so Phase 3 imports components by file path — the same
// convention TemplateField/CodeEditor use for AutocompleteList today:
//   import { AutocompletePopover } from '../autocomplete/AutocompletePopover.vue';

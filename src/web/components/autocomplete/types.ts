import type { IconName } from '../icons/icon-registry.ts';

export type {
  AutocompleteItem,
  AutocompleteKind,
  AutocompleteSource,
  GenericSuggestion,
  ScoredSuggestion,
} from './autocomplete.ts';

export type AutocompleteToken = {
  start: number;
  query: string;
  inside: boolean;
};

export type AutocompleteRow<T = unknown> = {
  key?: string;
  item: import('./autocomplete.ts').AutocompleteItem;
  ranges: Array<{ start: number; end: number }>;
  meta?: T;
};

/* ------------------------------------------------------------------ */
/* Phase 2 canonical subsystem (S6): modes, scopes, sections, rows.     */
/* Per-input logic lives in NO input component — only here. Phase 3     */
/* migrates TemplateField / URL / options inputs onto this contract.    */
/* ------------------------------------------------------------------ */

/**
 * Strict autocomplete mode. Modes never mix inside one flat list — a
 * dropdown showing two sources at once must render them as groups (S7).
 *
 * - `template`: event variables, ONLY inside `{{ }}` (see
 *   `findTemplateQuery`). URL text suggests nothing.
 * - `preset`: quick URL destinations, shown on focus-empty / match /
 *   explicit-invoke. Zero event variables, always.
 * - `options`: dynamic value lists (combobox-style), matched by label.
 */
export type AutocompleteMode = 'template' | 'preset' | 'options';

/**
 * Explicit variable scope (S24). The consumer (TemplateField `scope`)
 * declares one; the controller filters rows to it. `generic` matches
 * everything and is the default for scope-less consumers.
 */
export type SuggestionScope = 'message' | 'identity' | 'http-url' | 'http-data' | 'generic';

/** All scopes, for tests and scope pickers. */
export const SUGGESTION_SCOPES: readonly SuggestionScope[] = [
  'message',
  'identity',
  'http-url',
  'http-data',
  'generic',
];

/**
 * A caret inside a double-brace expression. `start` is the index of the
 * opening `{{`; `replaceEnd` is where the typed span ends (through the
 * closing `}}` when one follows the caret, else the caret itself);
 * `query` is the trimmed text between `{{` and the caret.
 */
export type TemplateQuery = {
  start: number;
  replaceEnd: number;
  query: string;
};

/**
 * Canonical suggestion row content. Extends the legacy `AutocompleteItem`
 * shape (value/label/kind/detail/documentation/preview stay the source of
 * truth for matching) with the S11 row contract:
 *
 * - `icon`: semantic icon. MUST be an `IconName` from the whitelist —
 *   arbitrary SVG from JSON/plugins is never rendered.
 * - `badge`: subtle 9px tag (`PRESET`, type name). Rendered uppercase.
 * - `description`: one-line hint under the label.
 * - `scopes`: which `SuggestionScope` values this row belongs to.
 *   Missing/empty means universal (shown under every scope).
 * - `group`: section label override (`User`, `Message`, …).
 */
export type SuggestionItem = import('./autocomplete.ts').AutocompleteItem & {
  icon?: IconName | string;
  badge?: string;
  description?: string;
  scopes?: SuggestionScope[];
  group?: string;
};

/** One rendered row: suggestion + highlight ranges + stable key. */
export type SuggestionRow = {
  key: string;
  item: SuggestionItem;
  ranges: Array<{ start: number; end: number }>;
};

/** One dropdown group (`role="group"`): label + rows. */
export type SuggestionSection = {
  id: string;
  label: string;
  rows: SuggestionRow[];
};

/** Flattened view the listbox renders: section + global row index. */
export type FlattenedSuggestionRow = {
  section: SuggestionSection;
  sectionIndex: number;
  row: SuggestionRow;
  globalIndex: number;
};

/** Flatten sections into render order with global indices. */
export function flattenSuggestionSections(sections: SuggestionSection[]): FlattenedSuggestionRow[] {
  const out: FlattenedSuggestionRow[] = [];
  sections.forEach((section, sectionIndex) => {
    section.rows.forEach((row) => {
      out.push({ section, sectionIndex, row, globalIndex: out.length });
    });
  });
  return out;
}

/** Stable DOM id for one option row (drives `aria-activedescendant`). */
export function suggestionOptionId(listId: string, globalIndex: number): string {
  return `${listId}-option-${globalIndex}`;
}

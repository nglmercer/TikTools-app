import { findTemplateQuery } from '../token.ts';
import type { Locale } from '../../../i18n.ts';
import type {
  AutocompleteMode,
  SuggestionItem,
  SuggestionRow,
  SuggestionScope,
  SuggestionSection,
  TemplateQuery,
} from '../types.ts';
import { shouldOpenOptions, shouldOpenPreset, shouldOpenTemplate } from './gates.ts';
import {
  applyOptionInsert,
  applyPresetInsert,
  applyTemplateInsert,
  type InsertResult,
} from './insertion.ts';
import { clampActiveIndex, moveActiveIndex, resolveAutocompleteKey } from './keyboard.ts';
import {
  defaultAutocompleteLabels,
  filterOptionRows,
  filterPresetRows,
  filterTemplateRows,
  groupTemplateRows,
  toPresetSections,
  type AutocompleteLabels,
  type PresetItem,
} from './rows.ts';

/* ------------------------------------------------------------------ */
/* Controller: framework-agnostic state machine Phase 3 mounts in      */
/* TemplateField / URL / options inputs. No DOM, no Vue — bun-testable.*/
/* ------------------------------------------------------------------ */

export type AutocompleteControllerOptions = {
  mode: AutocompleteMode;
  /** Template pool (event variables). Never consulted in preset mode. */
  suggestions?: readonly SuggestionItem[];
  /** Preset pool (URL destinations). Only consulted in preset mode. */
  presets?: readonly PresetItem[];
  /** Options pool (dynamic lists). Only consulted in options mode. */
  options?: readonly SuggestionItem[];
  /** Consumer scope for template filtering (default `generic`). */
  scope?: SuggestionScope;
  /** Template gate: only dedicated TemplateField passes true (S22). */
  supportsTemplates?: boolean;
  locale?: Locale;
  labels?: AutocompleteLabels;
  limit?: number;
  presetLimit?: number;
  /** Open the options list on focus-empty (default true). */
  openOptionsOnFocus?: boolean;
};

export type AutocompleteControllerSnapshot = {
  mode: AutocompleteMode;
  open: boolean;
  sections: SuggestionSection[];
  /** Total rows across sections (listbox size). */
  rowCount: number;
  activeIndex: number;
  /** Raw text the current rows were filtered by. */
  query: string;
  templateQuery: TemplateQuery | null;
};

export type AutocompleteController = {
  snapshot: () => AutocompleteControllerSnapshot;
  update: (input: { value: string; caret: number; focused: boolean }) => AutocompleteControllerSnapshot;
  /** Ctrl+Space: open from anywhere (latch clears on commit/dismiss). */
  invoke: () => AutocompleteControllerSnapshot;
  /** Escape: close, keep focus on the input (never blurs). */
  dismiss: () => AutocompleteControllerSnapshot;
  /** Hover sync: point the active index at the hovered row. */
  hover: (globalIndex: number) => AutocompleteControllerSnapshot;
  /**
   * Handle one key. Returns `commit` when the caller should insert
   * `pendingRow()`, `dismissed` on Escape, else null. Navigation mutates
   * the active index; no other key changes state.
   */
  key: (keyValue: string) => 'commit' | 'dismissed' | null;
  /** Row the active index points at (for commit), or null when closed. */
  pendingRow: () => SuggestionRow | null;
  /**
   * Insert the pending row into `value` at `caret` (mode-correct
   * replacement) and close. Returns null when there is nothing to commit.
   */
  commit: (value: string, caret: number) => (InsertResult & { row: SuggestionRow }) | null;
};

export function createAutocompleteController(options: AutocompleteControllerOptions): AutocompleteController {
  const mode = options.mode;
  const scope = options.scope ?? 'generic';
  const supportsTemplates = mode === 'template' ? options.supportsTemplates === true : false;
  const labels = options.labels ?? defaultAutocompleteLabels(options.locale ?? 'en');
  const limit = options.limit ?? 12;
  const presetLimit = options.presetLimit ?? 8;

  let value = '';
  let caret = 0;
  let focused = false;
  let explicit = false;
  let activeIndex = 0;
  let dismissedAt: { value: string; caret: number } | null = null;

  const dismissed = (): boolean =>
    dismissedAt !== null && dismissedAt.value === value && dismissedAt.caret === caret;

  function compute(): AutocompleteControllerSnapshot {
    const templateQuery = mode === 'template' && focused ? findTemplateQuery(value, caret) : null;
    if (!focused || dismissed()) {
      return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: '', templateQuery };
    }
    if (mode === 'template') {
      const queryText = templateQuery ? templateQuery.query : '';
      if (!shouldOpenTemplate({ supportsTemplates, focused, query: templateQuery, explicitInvoke: explicit })) {
        return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: queryText, templateQuery };
      }
      const rows = filterTemplateRows(options.suggestions ?? [], queryText, scope, limit);
      if (rows.length === 0) {
        return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: queryText, templateQuery };
      }
      const sections = groupTemplateRows(rows, labels);
      activeIndex = clampActiveIndex(activeIndex, rows.length);
      return { mode, open: true, sections, rowCount: rows.length, activeIndex, query: queryText, templateQuery };
    }
    if (mode === 'preset') {
      const queryText = value.trim();
      const rows = filterPresetRows(options.presets ?? [], queryText, presetLimit);
      if (!shouldOpenPreset({ focused, explicitInvoke: explicit, query: queryText, matchCount: rows.length })) {
        return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: queryText, templateQuery };
      }
      if (rows.length === 0) {
        return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: queryText, templateQuery };
      }
      activeIndex = clampActiveIndex(activeIndex, rows.length);
      return { mode, open: true, sections: toPresetSections(rows, labels), rowCount: rows.length, activeIndex, query: queryText, templateQuery };
    }
    const queryText = value.trim();
    const rows = filterOptionRows(options.options ?? [], queryText, limit);
    if (!shouldOpenOptions({ focused, explicitInvoke: explicit, query: queryText, matchCount: rows.length, openOnFocus: options.openOptionsOnFocus })) {
      return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: queryText, templateQuery };
    }
    if (rows.length === 0) {
      return { mode, open: false, sections: [], rowCount: 0, activeIndex: 0, query: queryText, templateQuery };
    }
    activeIndex = clampActiveIndex(activeIndex, rows.length);
    return { mode, open: true, sections: [{ id: 'options', label: '', rows }], rowCount: rows.length, activeIndex, query: queryText, templateQuery };
  }

  function flatRows(snapshot: AutocompleteControllerSnapshot): SuggestionRow[] {
    return snapshot.sections.flatMap((section) => section.rows);
  }

  const controller: AutocompleteController = {
    snapshot: () => compute(),
    update: (input) => {
      const changed = input.value !== value || input.caret !== caret;
      value = input.value;
      caret = Math.max(0, Math.min(input.caret, input.value.length));
      focused = input.focused;
      if (changed) {
        dismissedAt = null;
        activeIndex = 0;
      }
      if (!focused) explicit = false;
      return compute();
    },
    invoke: () => {
      explicit = true;
      dismissedAt = null;
      return compute();
    },
    dismiss: () => {
      explicit = false;
      dismissedAt = { value, caret };
      activeIndex = 0;
      return compute();
    },
    hover: (globalIndex) => {
      const snapshot = compute();
      activeIndex = clampActiveIndex(globalIndex, snapshot.rowCount);
      return { ...snapshot, activeIndex };
    },
    key: (keyValue) => {
      const action = resolveAutocompleteKey(keyValue);
      if (!action) return null;
      const snapshot = compute();
      if (!snapshot.open) return null;
      if (action === 'dismiss') {
        controller.dismiss();
        return 'dismissed';
      }
      if (action === 'commit') return flatRows(snapshot).length > 0 ? 'commit' : null;
      if (action === 'next') activeIndex = moveActiveIndex(activeIndex, 1, snapshot.rowCount);
      else if (action === 'prev') activeIndex = moveActiveIndex(activeIndex, -1, snapshot.rowCount);
      else if (action === 'first') activeIndex = clampActiveIndex(0, snapshot.rowCount);
      else activeIndex = clampActiveIndex(snapshot.rowCount - 1, snapshot.rowCount);
      return null;
    },
    pendingRow: () => {
      const snapshot = compute();
      if (!snapshot.open) return null;
      return flatRows(snapshot)[clampActiveIndex(activeIndex, snapshot.rowCount)] ?? null;
    },
    commit: (commitValue, commitCaret) => {
      const row = controller.pendingRow();
      if (!row) return null;
      let result: InsertResult;
      if (mode === 'template') result = applyTemplateInsert(commitValue, commitCaret, row.item.value);
      else if (mode === 'preset') result = applyPresetInsert(commitValue, row.item.value);
      else result = applyOptionInsert(commitValue, commitCaret, row.item.value);
      explicit = false;
      activeIndex = 0;
      dismissedAt = { value: result.value, caret: result.caret };
      value = result.value;
      caret = result.caret;
      return { ...result, row };
    },
  };
  return controller;
}

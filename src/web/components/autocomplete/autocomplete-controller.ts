import type { IconName } from '../icons/icon-registry.ts';
import { t, type Locale } from '../../i18n.ts';
import { filterSuggestions, type AutocompleteItem } from './autocomplete.ts';
import { findTemplateQuery } from './token.ts';
import type {
  AutocompleteMode,
  SuggestionItem,
  SuggestionRow,
  SuggestionScope,
  SuggestionSection,
  TemplateQuery,
} from './types.ts';

export { findTemplateQuery };
export type { AutocompleteMode, SuggestionItem, SuggestionRow, SuggestionScope, SuggestionSection, TemplateQuery };

/* ------------------------------------------------------------------ */
/* Scopes (S24): explicit per-consumer filtering.                      */
/* ------------------------------------------------------------------ */

/**
 * Keep rows for one consumer scope. Rows without a `scopes` tag are
 * universal (shown everywhere); `generic` matches every row. Consumers
 * pass their TemplateField `scope` straight through.
 */
export function filterByScope<T extends Pick<SuggestionItem, 'scopes'>>(
  items: readonly T[],
  scope: SuggestionScope,
): T[] {
  if (scope === 'generic') return [...items];
  return items.filter((item) => !item.scopes || item.scopes.length === 0 || item.scopes.includes(scope));
}

/* ------------------------------------------------------------------ */
/* Modes (S7/S8/S10): template opens only in braces, presets carry     */
/* zero event variables, options match dynamic lists.                  */
/* ------------------------------------------------------------------ */

/** Template gate: field supports templates + focused + caret in `{{ }}`. */
export function shouldOpenTemplate(input: {
  supportsTemplates: boolean;
  focused: boolean;
  query: TemplateQuery | null;
  explicitInvoke?: boolean;
}): boolean {
  if (!input.supportsTemplates || !input.focused) return false;
  if (input.query) return true;
  // Explicit invoke (Ctrl+Space) is the only path outside braces; plain
  // focus must NOT show event variables (S8).
  return input.explicitInvoke === true;
}

/**
 * Preset gate: focus-empty shows all, typing filters, explicit invoke
 * forces. Preset rows are URL destinations only — zero event vars (S10).
 */
export function shouldOpenPreset(input: {
  focused: boolean;
  explicitInvoke?: boolean;
  query: string;
  matchCount: number;
}): boolean {
  if (!input.focused) return false;
  if (input.explicitInvoke === true) return true;
  if (input.query.trim().length === 0) return true;
  return input.matchCount > 0;
}

/** Options gate: same shape as presets, over a dynamic list. */
export function shouldOpenOptions(input: {
  focused: boolean;
  explicitInvoke?: boolean;
  query: string;
  matchCount: number;
  openOnFocus?: boolean;
}): boolean {
  if (!input.focused) return false;
  if (input.explicitInvoke === true) return true;
  if (input.query.trim().length === 0) return input.openOnFocus !== false;
  return input.matchCount > 0;
}

/* ------------------------------------------------------------------ */
/* Row building: filter → rows → sections.                             */
/* ------------------------------------------------------------------ */

export type TemplateGroupName = 'User' | 'Message' | 'Text Intelligence';

/** Section order for grouped template rows (S11). */
export const TEMPLATE_GROUP_ORDER: readonly TemplateGroupName[] = ['User', 'Message', 'Text Intelligence'];

/**
 * Group one template path. Identity paths → User, Text Intelligence
 * views → Text Intelligence, everything else → Message.
 */
export function groupForTemplatePath(path: string): TemplateGroupName {
  const normalized = path.trim().toLowerCase();
  if (normalized === 'event.user' || normalized.startsWith('event.user.') || normalized.startsWith('event.intel.user.')) {
    return 'User';
  }
  if (normalized === 'event.intel' || normalized.startsWith('event.intel.')) return 'Text Intelligence';
  return 'Message';
}

/** Localized group/section labels. Callers may override every label. */
export type AutocompleteLabels = {
  userGroup: string;
  messageGroup: string;
  textIntelligenceGroup: string;
  presetSection: string;
};

export function defaultAutocompleteLabels(locale: Locale): AutocompleteLabels {
  return {
    userGroup: t(locale, 'autocompleteGroupUser'),
    messageGroup: t(locale, 'autocompleteGroupMessage'),
    textIntelligenceGroup: t(locale, 'autocompleteGroupTextIntelligence'),
    presetSection: t(locale, 'autocompleteQuickDestinations'),
  };
}

function toSuggestionRow(
  item: AutocompleteItem,
  ranges: Array<{ start: number; end: number }>,
  keyPrefix: string,
): SuggestionRow {
  const suggestion = item as SuggestionItem;
  const badge = suggestion.badge ?? item.detail ?? item.kind;
  return {
    key: `${keyPrefix}:${item.value}`,
    item: {
      ...item,
      badge,
      description: suggestion.description ?? item.documentation ?? item.preview,
      icon: suggestion.icon ?? iconForSuggestion(item),
    },
    ranges,
  };
}

/** Template rows: scope-filtered, fuzzy-matched, ungrouped. */
export function filterTemplateRows(
  suggestions: readonly SuggestionItem[],
  query: string,
  scope: SuggestionScope,
  limit = 12,
): SuggestionRow[] {
  const pool = filterByScope(suggestions, scope);
  return filterSuggestions(pool, query, limit).map((entry) => toSuggestionRow(entry.item, entry.matchRanges, 'tpl'));
}

/** Group template rows into User / Message / Text Intelligence (S11). */
export function groupTemplateRows(rows: readonly SuggestionRow[], labels?: AutocompleteLabels): SuggestionSection[] {
  const resolved = labels ?? defaultAutocompleteLabels('en');
  const names: Record<TemplateGroupName, string> = {
    User: resolved.userGroup,
    Message: resolved.messageGroup,
    'Text Intelligence': resolved.textIntelligenceGroup,
  };
  const buckets = new Map<TemplateGroupName, SuggestionRow[]>();
  for (const row of rows) {
    const group = row.item.group as TemplateGroupName | undefined;
    const name = group && TEMPLATE_GROUP_ORDER.includes(group) ? group : groupForTemplatePath(row.item.value);
    const bucket = buckets.get(name) ?? [];
    bucket.push(row);
    buckets.set(name, bucket);
  }
  return TEMPLATE_GROUP_ORDER.filter((name) => (buckets.get(name) ?? []).length > 0).map((name) => ({
    id: `template-${name.toLowerCase().replace(/[^a-z]+/g, '-')}`,
    label: names[name],
    rows: buckets.get(name) ?? [],
  }));
}

export type PresetItem = {
  /** Stable id; also the row key suffix. */
  id: string;
  /** Short chip label, e.g. `localhost:3000`. */
  label: string;
  /** Full URL to apply. */
  url: string;
  /** Tooltip / description line. */
  hint?: string;
};

function toPresetSuggestion(preset: PresetItem): SuggestionItem {
  return {
    value: preset.url,
    label: preset.label,
    kind: 'snippet',
    detail: 'preset',
    documentation: preset.hint ?? preset.url,
    preview: preset.label,
    icon: 'globe',
    badge: 'PRESET',
    description: preset.hint ?? preset.url,
  };
}

/**
 * Preset rows: URL destinations matched by label or URL (S10). The pool is
 * presets only — event variables can never appear here by construction.
 */
export function filterPresetRows(
  presets: readonly PresetItem[],
  query: string,
  limit = 8,
): SuggestionRow[] {
  const pool = presets.map(toPresetSuggestion);
  return filterSuggestions(pool, query, limit).map((entry) => ({
    key: `preset:${presets.find((preset) => preset.url === entry.item.value)?.id ?? entry.item.value}`,
    item: { ...entry.item, icon: 'globe' as IconName, badge: 'PRESET' },
    ranges: entry.matchRanges,
  }));
}

/** Single-section wrapper for preset rows (`Quick destinations`). */
export function toPresetSections(rows: readonly SuggestionRow[], labels?: AutocompleteLabels): SuggestionSection[] {
  if (rows.length === 0) return [];
  return [{ id: 'preset-destinations', label: (labels ?? defaultAutocompleteLabels('en')).presetSection, rows: [...rows] }];
}

/** Options rows: dynamic lists matched by label/value (S7). */
export function filterOptionRows(
  options: readonly SuggestionItem[],
  query: string,
  limit = 12,
): SuggestionRow[] {
  return filterSuggestions([...options], query, limit).map((entry) =>
    toSuggestionRow(entry.item, entry.matchRanges, 'opt'),
  );
}

/* ------------------------------------------------------------------ */
/* Semantic icons (S12): IconName only, never arbitrary SVG.            */
/* ------------------------------------------------------------------ */

/**
 * Semantic icon for one row. Presets always use the server icon; template
 * paths map by section; option rows fall back to their kind. The return
 * is always a whitelisted `IconName` — untrusted `icon` strings from
 * JSON/plugins are ignored (validated at render via `readIconName`).
 */
export function iconForSuggestion(item: Pick<AutocompleteItem, 'value' | 'kind' | 'detail'>): IconName {
  if (item.detail === 'preset' || item.kind === 'snippet') return 'globe';
  const group = groupForTemplatePath(item.value);
  if (group === 'User') return 'users';
  if (group === 'Text Intelligence') return 'sparkles';
  switch (item.kind) {
    case 'number': return 'stats';
    case 'boolean': return 'check';
    case 'object':
    case 'array': return 'json';
    case 'path': return 'link';
    default: break;
  }
  if (item.value.startsWith('event.data.')) return 'chat';
  if (item.value.startsWith('event.')) return 'code';
  if (/^https?:\/\//i.test(item.value)) return 'globe';
  return 'dot';
}

/* ------------------------------------------------------------------ */
/* Keyboard (S14): Up/Down/Enter/Escape/Tab/Home/End + hover sync.      */
/* ------------------------------------------------------------------ */

export type AutocompleteKeyAction = 'next' | 'prev' | 'first' | 'last' | 'commit' | 'dismiss';

/**
 * Map one key to its listbox action, or null when the key is not handled.
 * Up/Down move (wrapping), Home/End jump, Enter/Tab commit, Escape
 * dismisses. Escape closes WITHOUT moving focus — focus stays on the
 * input (S15); the controller never blurs.
 */
export function resolveAutocompleteKey(key: string): AutocompleteKeyAction | null {
  switch (key) {
    case 'ArrowDown': return 'next';
    case 'ArrowUp': return 'prev';
    case 'Home': return 'first';
    case 'End': return 'last';
    case 'Enter':
    case 'Tab': return 'commit';
    case 'Escape': return 'dismiss';
    default: return null;
  }
}

/** Ctrl/Meta+Space explicitly invokes the dropdown from anywhere. */
export function isExplicitInvokeKey(event: { key: string; ctrlKey: boolean; metaKey: boolean }): boolean {
  return event.key === ' ' && (event.ctrlKey || event.metaKey);
}

/** Wrap-around step for Up/Down. Empty lists stay at 0. */
export function moveActiveIndex(index: number, delta: number, length: number): number {
  if (length <= 0) return 0;
  return (index + delta + length) % length;
}

/** Clamp any index (hover sync, Home/End) into range. */
export function clampActiveIndex(index: number, length: number): number {
  if (length <= 0) return 0;
  return Math.max(0, Math.min(index, length - 1));
}

/* ------------------------------------------------------------------ */
/* Insertion (S14): caret-preserving, range-replacing.                 */
/* ------------------------------------------------------------------ */

export type InsertResult = { value: string; caret: number };

/**
 * Insert a template variable: replaces the `{{ …` span under the caret
 * with `{{ value }}` (consuming a directly-following `}}` so no stray
 * braces remain). Outside braces (explicit invoke) inserts `{{ value }}`
 * at the caret, replacing the current word when one is under it.
 * Surrounding text and the trailing caret position are preserved.
 */
export function applyTemplateInsert(value: string, caret: number, suggestionValue: string): InsertResult {
  const safeCaret = Math.max(0, Math.min(caret, value.length));
  const query = findTemplateQuery(value, safeCaret);
  const inserted = `{{ ${suggestionValue} }}`;
  if (query) {
    const nextValue = `${value.slice(0, query.start)}${inserted}${value.slice(query.replaceEnd)}`;
    const nextCaret = query.start + inserted.length;
    return { value: nextValue, caret: nextCaret };
  }
  const word = /[A-Za-z0-9_$.]*$/.exec(value.slice(0, safeCaret))?.[0] ?? '';
  const start = safeCaret - word.length;
  const nextValue = `${value.slice(0, start)}${inserted}${value.slice(safeCaret)}`;
  return { value: nextValue, caret: start + inserted.length };
}

/**
 * Insert a URL preset: swaps only the `scheme://host` origin, keeping the
 * typed path/query. With no origin yet (empty field, bare `https://`),
 * the preset URL is used as-is. Caret lands at the end.
 */
export function applyPresetInsert(current: string, presetUrl: string): InsertResult {
  const origin = /^https?:\/\/[^/?#\s]*/i.exec(presetUrl.trim())?.[0]?.replace(/\/+$/, '') ?? presetUrl.trim();
  const match = /^https?:\/\/[^/?#\s]*/i.exec(current);
  if (!match) {
    const nextValue = presetUrl;
    return { value: nextValue, caret: nextValue.length };
  }
  const rest = current.slice(match[0].length);
  const nextValue = !rest ? `${origin}/` : /^[/?#]/.test(rest) ? `${origin}${rest}` : `${origin}/${rest}`;
  return { value: nextValue, caret: nextValue.length };
}

/** Insert an option: replaces the word under the caret with the value. */
export function applyOptionInsert(value: string, caret: number, optionValue: string): InsertResult {
  const safeCaret = Math.max(0, Math.min(caret, value.length));
  const word = /[A-Za-z0-9_$.]*$/.exec(value.slice(0, safeCaret))?.[0] ?? '';
  const start = safeCaret - word.length;
  const nextValue = `${value.slice(0, start)}${optionValue}${value.slice(safeCaret)}`;
  return { value: nextValue, caret: start + optionValue.length };
}

/* ------------------------------------------------------------------ */
/* ARIA (S15): combobox input attributes.                              */
/* ------------------------------------------------------------------ */

export type ComboboxInputAttrs = {
  role: 'combobox';
  'aria-expanded': boolean;
  'aria-controls': string | undefined;
  'aria-activedescendant': string | undefined;
  'aria-autocomplete': 'list';
  'aria-describedby': string | undefined;
  'aria-invalid': boolean;
  'aria-errormessage': string | undefined;
};

/**
 * Attributes Phase 3 spreads onto the text input. `listId` is the popup
 * `listbox` id; `activeIndex` selects the `aria-activedescendant` option
 * while open. `describedBy` should already include help/error ids.
 */
export function comboboxInputAttrs(input: {
  listId: string;
  open: boolean;
  activeIndex: number;
  rowCount: number;
  describedBy?: string;
  invalid?: boolean;
  errorId?: string;
}): ComboboxInputAttrs {
  const hasActive = input.open && input.rowCount > 0 && input.activeIndex >= 0;
  return {
    role: 'combobox',
    'aria-expanded': input.open,
    'aria-controls': input.open ? input.listId : undefined,
    'aria-activedescendant': hasActive ? `${input.listId}-option-${clampActiveIndex(input.activeIndex, input.rowCount)}` : undefined,
    'aria-autocomplete': 'list',
    'aria-describedby': input.describedBy,
    'aria-invalid': input.invalid === true,
    'aria-errormessage': input.invalid === true ? input.errorId : undefined,
  };
}

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
      const all = filterPresetRows(options.presets ?? [], '', presetLimit);
      const rows = queryText.length === 0 ? all : filterPresetRows(options.presets ?? [], queryText, presetLimit);
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

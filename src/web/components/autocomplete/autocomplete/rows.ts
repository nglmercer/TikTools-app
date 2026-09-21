import type { IconName } from '../../icons/icon-registry.ts';
import { t, type Locale } from '../../../i18n.ts';
import { filterSuggestions, type AutocompleteItem } from '../autocomplete.ts';
import type { SuggestionItem, SuggestionRow, SuggestionScope, SuggestionSection } from '../types.ts';
import { filterByScope } from './gates.ts';
import { iconForSuggestion } from './icons.ts';

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
  // URL -> id index built once: the per-row `find` was O(n) per result.
  // First preset wins on duplicate URLs, matching the old `find` lookup.
  const idByUrl = new Map<string, string>();
  for (const preset of presets) {
    if (!idByUrl.has(preset.url)) idByUrl.set(preset.url, preset.id);
  }
  return filterSuggestions(pool, query, limit).map((entry) => ({
    key: `preset:${idByUrl.get(entry.item.value) ?? entry.item.value}`,
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

import type { EventFilter, FilterOperator, Localized } from './types.ts';

/**
 * Generic condition templates for the event editor's "Only if…" section.
 *
 * Hand-writing filters is expert work (paths, operators, numeric edges), so
 * templates bundle ready-made rows behind a name: "Moderation: High" fills
 * the section in one click. Templates are data, registered per trigger, so
 * any feature or plugin integration can contribute its own without touching
 * the editor: the Templates button appears only while templates exist for
 * the current trigger.
 *
 * Two shapes:
 *
 * - Simple: fixed rows, applied as-is.
 * - Configurable: rows reference `params` (`{ param: 'threshold' }`); the
 *   editor opens a config modal with one typed field per param, then builds
 *   the filters from the answers. Params keep templates reusable instead of
 *   hardcoding one author's magic numbers.
 *
 * Engine contract (do not break it): filter values are literal strings on
 * both matchers (`matchesFilter` here and the host's `filters.rs`) — a
 * `{{path}}` value would compare literally and never match. Template rows
 * therefore store plain dotted paths and literal values; cross-field
 * references need a matcher change on both sides first. Applying a template
 * replaces the section's rows; every row stays editable inline afterwards.
 *
 * UI-agnostic on purpose: `src/automation` never imports from `src/web`.
 * Labels are `Localized` so editors resolve them with `i18nText`.
 */
export type ConditionTemplateParamKind = 'text' | 'number';

export interface ConditionTemplateParam {
  /** Stable within its template; rows reference it as `{ param: key }`. */
  key: string;
  label: Localized;
  hint?: Localized;
  kind: ConditionTemplateParamKind;
  /** Default answer, in exactly the string form the filter stores. */
  default: string;
  /** Number-only editor bounds; ignored for text params. */
  min?: number;
  max?: number;
  step?: number;
}

/** A literal filter value, or a reference to the template's config answer. */
export type ConditionTemplateValue = string | { param: string };

export interface ConditionTemplateRow {
  /** Plain dotted path, exactly what the filter stores (never `{{}}`). */
  path: string;
  operator: FilterOperator;
  value: ConditionTemplateValue;
  /** `in` only: carried through untouched. */
  values?: string[];
}

export interface ConditionTemplate {
  /** Stable across triggers; re-registering an id replaces the template. */
  id: string;
  title: Localized;
  description?: Localized;
  /** Event types this template applies to; absent means every trigger. */
  triggers?: string[];
  params?: ConditionTemplateParam[];
  rows: ConditionTemplateRow[];
}

const TEMPLATE_REGISTRY = new Map<string, ConditionTemplate>();

function cleanParam(param: ConditionTemplateParam): ConditionTemplateParam | undefined {
  const key = param.key.trim();
  if (!key) return undefined;
  if (param.kind !== 'text' && param.kind !== 'number') return undefined;
  return { ...param, key };
}

function cleanRow(row: ConditionTemplateRow): ConditionTemplateRow | undefined {
  const path = row.path.trim();
  if (!path || !row.operator) return undefined;
  return { ...row, path };
}

/**
 * Register (or replace by id) condition templates. Entries with an empty
 * id, no rows, or rows without a path are skipped; the registry never
 * holds a template that cannot produce filters.
 */
export function registerConditionTemplates(templates: ConditionTemplate[]): void {
  for (const template of templates) {
    const id = template.id.trim();
    if (!id) continue;
    const rows = template.rows.map(cleanRow).filter((row): row is ConditionTemplateRow => row !== undefined);
    if (rows.length === 0) continue;
    const params = (template.params ?? [])
      .map(cleanParam)
      .filter((param): param is ConditionTemplateParam => param !== undefined);
    const triggers = [...new Set((template.triggers ?? []).map((trigger) => trigger.trim()).filter(Boolean))];
    TEMPLATE_REGISTRY.set(id, {
      ...template,
      id,
      rows,
      params: params.length > 0 ? params : undefined,
      triggers: triggers.length > 0 ? triggers : undefined,
    });
  }
}

/** Templates applying to a trigger, in registration order. Empty when none. */
export function conditionTemplatesFor(trigger: string): ConditionTemplate[] {
  return [...TEMPLATE_REGISTRY.values()].filter(
    (template) => !template.triggers || template.triggers.includes(trigger),
  );
}

/** Clear the whole registry (tests). */
export function clearConditionTemplates(): void {
  TEMPLATE_REGISTRY.clear();
}

function resolveParamValue(
  param: ConditionTemplateParam | undefined,
  answer: string | undefined,
): string {
  const candidate = (answer ?? param?.default ?? '').trim();
  if (!param || param.kind !== 'number') return candidate;
  // Numbers normalize (`0.70` → `0.7`); garbage falls back to a valid
  // default, else empty — the table flags it as missing instead of
  // storing a value that can never match.
  if (candidate !== '' && Number.isFinite(Number(candidate))) return String(Number(candidate));
  const fallback = (param.default ?? '').trim();
  return fallback !== '' && Number.isFinite(Number(fallback)) ? String(Number(fallback)) : '';
}

/**
 * Build the section's filters from a template and its config answers.
 * Missing answers fall back to param defaults; unknown param keys resolve
 * leniently so one bad row never breaks the whole apply.
 */
export function applyConditionTemplate(
  template: ConditionTemplate,
  answers: Record<string, string>,
): EventFilter[] {
  const params = new Map((template.params ?? []).map((param) => [param.key, param]));
  return template.rows.map((row) => {
    const filter: EventFilter = {
      path: row.path,
      operator: row.operator,
      value: typeof row.value === 'string'
        ? row.value
        : resolveParamValue(params.get(row.value.param), answers[row.value.param]),
    };
    if (row.values) filter.values = [...row.values];
    return filter;
  });
}

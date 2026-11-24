import { fieldsForTrigger, findField, operatorsFor, operatorsForPath, type FieldValueKind } from '../../../../automation/behavior/fields.ts';
import type { EventFilter, FilterOperator, LiveEvent } from '../../../../automation/behavior/types.ts';
import { fieldsForEventType } from '../../../../automation/event-registry.ts';

export type { FieldValueKind };

export interface ConditionField {
  path: string;
  label: string;
  hint: string;
  kind: FieldValueKind;
  options: { value: string; label: string }[];
}

type Language = 'en' | 'es';

export const OPERATOR_CODE: Record<FilterOperator, string> = {
  gte: '>=',
  gt: '>',
  lte: '<=',
  lt: '<',
  eq: '=',
  neq: '!=',
  in: 'in',
  contains: '∋',
  'starts-with': '^=',
  'is-true': '= true',
  'is-false': '= false',
};

/** Operators that compare against a value or a value list. */
const VALUE_OPERATORS: ReadonlySet<FilterOperator> = new Set([
  'gte', 'gt', 'lte', 'lt', 'eq', 'neq', 'contains', 'starts-with', 'in',
]);

function language(locale: string): Language {
  return locale.startsWith('es') ? 'es' : 'en';
}

export function conditionFields(trigger: string, locale: string): ConditionField[] {
  const lang = language(locale);
  const registry = new Map(fieldsForEventType(trigger).map((field) => [field.path, field]));
  return fieldsForTrigger(trigger).map((field) => {
    const source = registry.get(field.path);
    return {
      path: field.path,
      label: source?.label[lang] ?? field.label.default,
      hint: source?.hint?.[lang] ?? field.hint.default,
      kind: field.kind,
      options: (source?.options ?? []).map((option) => ({ value: option.value, label: option.label[lang] })),
    };
  });
}

export function fieldKind(trigger: string, path: string): FieldValueKind {
  return findField(trigger, path)?.kind ?? 'text';
}

export function isKnownField(trigger: string, path: string): boolean {
  return findField(trigger, path) !== undefined;
}

export function operatorsForFilter(trigger: string, path: string): FilterOperator[] {
  return operatorsForPath(trigger, path);
}

export function needsValue(operator: FilterOperator): boolean {
  return operator !== 'is-true' && operator !== 'is-false';
}

/** Whether the operator reads a `values` list (everything but `in`-only legacy use is covered). */
export function takesValues(operator: FilterOperator): boolean {
  return VALUE_OPERATORS.has(operator);
}

/** The effective candidate list: `values` when stored, else the single `value`. */
export function filterValues(filter: EventFilter): string[] {
  if (filter.values !== undefined) return [...filter.values];
  return filter.value ? [filter.value] : [];
}

const CAPTURE_ALIAS = /^[a-zA-Z][a-zA-Z0-9_]{0,31}$/;
const RESERVED_CAPTURE_KEYS = new Set(['match', 'event', 'globals', 'data', 'user']);

/** Same alias rules as the host matcher; anything else falls back to the position. */
export function captureAlias(value: string | undefined): string | undefined {
  if (!value || !CAPTURE_ALIAS.test(value) || RESERVED_CAPTURE_KEYS.has(value)) return undefined;
  return value;
}

/** `match.*` paths an event can publish, in declaration order. */
export function captureVariables(filters: readonly EventFilter[]): string[] {
  return filters.map((filter, index) => `match.${captureAlias(filter.as) ?? index}`);
}

export function eventCaptures(event: Pick<LiveEvent, 'filters' | 'anyOf'>): string[] {
  const paths = new Set(captureVariables(event.filters));
  for (const group of event.anyOf ?? []) {
    for (const path of captureVariables(group)) paths.add(path);
  }
  return [...paths];
}

export function isFilterMissing(filter: EventFilter, options: readonly { value: string }[] = []): boolean {
  if (!needsValue(filter.operator)) return false;
  if (filterValues(filter).length > 0) return false;
  return !(filter.operator !== 'in' && options.some((option) => option.value === filter.value));
}

export function newFilter(trigger: string): EventFilter {
  const first = fieldsForTrigger(trigger)[0];
  return {
    path: first?.path ?? 'event.user.uniqueId',
    operator: operatorsFor(first?.kind ?? 'text')[0]!,
    value: '',
    values: undefined,
  };
}

/** Quantifier carried across edits; `neq` lists always mean «none of». */
function carriedQuantifier(filter: EventFilter, operator: FilterOperator, values: readonly string[] | undefined): EventFilter['quantifier'] {
  if (operator === 'in' || operator === 'is-true' || operator === 'is-false') return undefined;
  if (operator === 'neq' && (values?.length ?? 0) > 0) return 'all';
  return filter.quantifier === 'all' ? 'all' : undefined;
}

export function withField(trigger: string, filter: EventFilter, path: string | null): EventFilter {
  if (path === null) return { path: '', operator: 'eq', value: '', values: undefined };
  const operators = operatorsFor(fieldKind(trigger, path));
  const operator = operators.includes(filter.operator) ? filter.operator : operators[0]!;
  const values = takesValues(operator) ? filter.values : undefined;
  return {
    path,
    operator,
    value: needsValue(operator) && values === undefined ? filter.value : '',
    values,
    quantifier: carriedQuantifier(filter, operator, values),
    as: takesValues(operator) ? filter.as : undefined,
  };
}

export function withOperator(filter: EventFilter, operator: FilterOperator): EventFilter {
  const picked = filterValues(filter);
  const values = takesValues(operator) ? (filter.values ?? (picked.length ? picked : undefined)) : undefined;
  return {
    ...filter,
    operator,
    values,
    quantifier: carriedQuantifier(filter, operator, values),
    as: takesValues(operator) ? filter.as : undefined,
    value: values === undefined && needsValue(operator) ? filter.value || picked[0] || '' : '',
  };
}

export function withValues(filter: EventFilter, values: readonly string[]): EventFilter {
  if (!takesValues(filter.operator)) return filter;
  return { ...filter, values: [...values], value: '' };
}

export function withQuantifier(filter: EventFilter, quantifier: 'any' | 'all'): EventFilter {
  if (!takesValues(filter.operator) || filter.operator === 'in') return filter;
  // `neq` lists always mean «none of»: `any` is normalized to `all`.
  if (filter.operator === 'neq') return { ...filter, quantifier: 'all' };
  return { ...filter, quantifier: quantifier === 'all' ? 'all' : undefined };
}

export function describeFilter(filter: EventFilter, fieldLabel: string, operatorLabel: string): string {
  const field = fieldLabel || filter.path.replace(/^event\.(data|user)\./, '');
  if (!needsValue(filter.operator)) return `${field} ${operatorLabel}`;
  const value = (filter.values?.length ? filter.values.join(', ') : filter.value) || '…';
  const quantifier = filter.quantifier === 'all' && (filter.values?.length ?? 0) > 1 ? 'all of ' : '';
  return `${field} ${operatorLabel} ${quantifier}${value}`;
}

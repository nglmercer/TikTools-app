import { fieldsForTrigger, findField, operatorsFor, operatorsForPath, type FieldValueKind } from '../../../../automation/behavior/fields.ts';
import type { EventFilter, FilterOperator } from '../../../../automation/behavior/types.ts';
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

export function filterValues(filter: EventFilter): string[] {
  if (filter.operator === 'in') return filter.values ?? [];
  return filter.value ? [filter.value] : [];
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

export function withField(trigger: string, filter: EventFilter, path: string | null): EventFilter {
  if (path === null) return { path: '', operator: 'eq', value: '', values: undefined };
  const operators = operatorsFor(fieldKind(trigger, path));
  const operator = operators.includes(filter.operator) ? filter.operator : operators[0]!;
  return {
    path,
    operator,
    value: needsValue(operator) ? filter.value : '',
    values: operator === 'in' ? (filter.values ?? []) : undefined,
  };
}

export function withOperator(filter: EventFilter, operator: FilterOperator): EventFilter {
  const picked = filterValues(filter);
  return {
    ...filter,
    operator,
    values: operator === 'in' ? (filter.values ?? picked) : undefined,
    value: operator === 'in' || !needsValue(operator) ? '' : filter.value || picked[0] || '',
  };
}

export function withValues(filter: EventFilter, values: readonly string[]): EventFilter {
  if (filter.operator === 'in') return { ...filter, values: [...values] };
  return { ...filter, value: values[0] ?? '' };
}

export function describeFilter(filter: EventFilter, fieldLabel: string, operatorLabel: string): string {
  const field = fieldLabel || filter.path.replace(/^event\.(data|user)\./, '');
  if (!needsValue(filter.operator)) return `${field} ${operatorLabel}`;
  const value = filter.operator === 'in' ? (filter.values ?? []).join(', ') : filter.value;
  return `${field} ${operatorLabel} ${value || '…'}`;
}

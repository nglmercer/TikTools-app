import { fieldsForEventType as registryFieldsFor } from '../event-registry.ts';

import type { FilterOperator, Localized } from './types.ts';

/**
 * The condition editor never asks anyone to type `event.data.diamondCount`.
 * It offers the fields a trigger actually carries, each with the kind of value
 * it holds — which in turn decides the operators on offer and the editor used
 * for the value (a gift picker, a viewer picker, a number, a switch).
 *
 * Field candidates come from the checked-in native event registry — never
 * from hardcoded per-trigger lists. What is presentation-only stays here:
 * which picker a path opens (`kind`) and its glyph (`icon`).
 */
export type FieldIcon = 'gift' | 'gem' | 'user' | 'star' | 'repeat' | 'text' | 'hash' | 'clock';

export type FieldValueKind = 'gift' | 'user' | 'number' | 'text' | 'boolean';

export interface EventFieldDefinition {
  /** Dotted path, exactly what the filter stores. */
  path: string;
  label: Localized;
  icon: FieldIcon;
  kind: FieldValueKind;
  options?: Array<{ value: string; label: Localized }>;
  /** Shown behind the info icon: what the field means, in plain words. */
  hint: Localized;
}

const NUMBER_OPS: FilterOperator[] = ['gte', 'gt', 'lte', 'lt', 'eq', 'neq'];
const TEXT_OPS: FilterOperator[] = ['eq', 'neq', 'in', 'contains', 'starts-with'];
const PICK_OPS: FilterOperator[] = ['eq', 'neq', 'in'];
const BOOL_OPS: FilterOperator[] = ['is-true', 'is-false'];

export function operatorsFor(kind: FieldValueKind): FilterOperator[] {
  switch (kind) {
    case 'number':
      return NUMBER_OPS;
    case 'boolean':
      return BOOL_OPS;
    case 'gift':
    case 'user':
      return PICK_OPS;
    default:
      return TEXT_OPS;
  }
}

/** Envelope meta nobody filters on: ids, timestamps, whole-object dumps. */
const EXCLUDED_PATHS = new Set([
  'event.id',
  'event.type',
  'event.timestamp',
  'event.connectionId',
  'event.sourceEventId',
  'event.user',
  'event.creator',
  'event.data',
  'event.points',
]);

/** Which value editor a registry path asks for. */
function kindForPath(path: string, tsKind: string): FieldValueKind | null {
  if (tsKind === 'boolean') return 'boolean';
  if (tsKind === 'number') return 'number';
  if (tsKind !== 'string') return null;
  const leaf = path.split('.').pop() ?? '';
  if (leaf === 'giftName') return 'gift';
  if (['uniqueId', 'userId', 'nickname'].includes(leaf)) return 'user';
  return 'text';
}

/** Which glyph a registry path draws. */
function iconForPath(path: string, kind: FieldValueKind): FieldIcon {
  const leaf = path.split('.').pop() ?? '';
  if (kind === 'gift') return 'gift';
  if (leaf === 'diamondCount') return 'gem';
  if (kind === 'user' || leaf === 'viewers') return 'user';
  if (leaf === 'delta' || leaf === 'totalPoints' || leaf === 'currencyName') return 'star';
  if (leaf === 'repeatEnd') return 'repeat';
  if (kind === 'number') return 'hash';
  return 'text';
}

/** Trigger-specific data fields first, then the per-viewer identity fields. */
export function fieldsForTrigger(trigger: string): EventFieldDefinition[] {
  const data: EventFieldDefinition[] = [];
  const identity: EventFieldDefinition[] = [];
  for (const field of registryFieldsFor(trigger)) {
    if (EXCLUDED_PATHS.has(field.path)) continue;
    // Array-element shapes are not filterable scalar fields.
    if (field.path.includes('.0.')) continue;
    if (!field.path.startsWith('event.data.') && !field.path.startsWith('event.user.')) continue;
    const kind = kindForPath(field.path, field.kind);
    if (!kind) continue;
    const labelKey = field.i18key ?? `automation.event.field.${field.path}.label`;
    const hintEn = field.hint?.en ?? `${field.path} (${field.tsType})`;
    const hintKey = field.hint && field.i18key ? field.i18key.replace(/\.label$/, '.hint') : `automation.event.field.${field.path}.hint`;
    const definition: EventFieldDefinition = {
      path: field.path,
      label: { default: field.label.en, i18key: labelKey },
      options: field.options?.map((option) => ({ value: option.value, label: { default: option.label.en, i18key: '' } })),
      icon: iconForPath(field.path, kind),
      kind,
      hint: { default: hintEn, i18key: hintKey },
    };
    if (field.path.startsWith('event.data.')) data.push(definition);
    else identity.push(definition);
  }
  return [...data, ...identity];
}

export function findField(trigger: string, path: string): EventFieldDefinition | undefined {
  return fieldsForTrigger(trigger).find((field) => field.path === path);
}

/** A path the user wrote by hand still has to render: treat it as free text. */
export function fieldKindFor(trigger: string, path: string): FieldValueKind {
  return findField(trigger, path)?.kind ?? 'text';
}

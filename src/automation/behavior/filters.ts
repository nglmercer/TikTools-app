import type { AutomationEvent, JsonValue } from '../types.ts';
import { readEventPath } from './templates.ts';
import type { EventFilter } from './types.ts';

/** Every filter of an event is an AND; the `in` operator expresses OR. */
export function matchesFilter(filter: EventFilter, event: AutomationEvent): boolean {
  const raw = readEventPath(event, filter.path);
  const left = readString(raw);
  const leftNumber = Number(left);
  const rightNumber = Number(filter.value);
  const numeric = Number.isFinite(leftNumber) && Number.isFinite(rightNumber) && filter.value.trim() !== '';

  switch (filter.operator) {
    case 'gte': return numeric && leftNumber >= rightNumber;
    case 'gt': return numeric && leftNumber > rightNumber;
    case 'lte': return numeric && leftNumber <= rightNumber;
    case 'lt': return numeric && leftNumber < rightNumber;
    case 'eq': return numeric ? leftNumber === rightNumber : left === filter.value;
    case 'neq': return numeric ? leftNumber !== rightNumber : left !== filter.value;
    case 'contains': return left.toLowerCase().includes(filter.value.toLowerCase());
    case 'starts-with': return left.toLowerCase().startsWith(filter.value.toLowerCase());
    case 'in': return (filter.values ?? []).some((entry) => entry.trim().toLowerCase() === left.trim().toLowerCase());
    case 'is-true': return raw === true || left === 'true' || left === '1';
    case 'is-false': return raw === false || left === 'false' || left === '0' || left === '';
    default: return false;
  }
}

function readString(value: JsonValue | undefined): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

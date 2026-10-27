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
    case 'crosses-multiple': {
      // An explicit opt-in for recurring milestones on observed counter
      // transitions, never on the TikTok room-wide total snapshot.
      if (event.type !== 'tiktok.like' ||
          !['event.data.aggregation.global', 'event.data.aggregation.user'].includes(filter.path) ||
          !/^[1-9][0-9]*$/.test(filter.value)) return false;
      const step = Number(filter.value);
      if (!Number.isSafeInteger(step) || !raw || typeof raw !== 'object' || Array.isArray(raw)) return false;
      const transition = raw as Record<string, unknown>;
      const previous = transition.previous;
      const total = transition.total;
      return typeof previous === 'number' && typeof total === 'number' &&
        Number.isSafeInteger(previous) && Number.isSafeInteger(total) &&
        previous >= 0 && total > previous &&
        Math.floor(total / step) > Math.floor(previous / step);
    }
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

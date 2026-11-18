import type { JsonValue } from '../../../../automation/types.ts';

/** Host records are typed interfaces without an index signature; the review
 * JSON components accept JsonValue, so cast once at this boundary. */
export function reviewJson(value: unknown): JsonValue | undefined {
  return value === undefined ? undefined : value as JsonValue;
}

export function reviewHasValue(before: unknown, after: unknown): boolean {
  return before !== undefined || after !== undefined;
}

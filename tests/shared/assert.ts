/** Narrowing helpers for tests: fail with a readable message instead of a bare `!` that would throw later. */

/** The value, or a thrown error naming what was missing. Replaces `value!`. */
export function defined<T>(value: T, what = 'value'): NonNullable<T> {
  if (value === null || value === undefined) throw new Error(`Expected ${what} to be defined`);
  return value;
}

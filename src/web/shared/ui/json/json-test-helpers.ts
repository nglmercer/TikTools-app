import { expect } from 'vitest';

/**
 * Assert that a rendered JSON document is byte-for-byte what JSON.stringify
 * produces with two-space indentation. Used by the tokenizer/diff tests so
 * the reference serialization is written once.
 */
export function expectPrettyJson(value: unknown, actual: string, label?: string): void {
  expect(actual, label ?? JSON.stringify(value)).toBe(JSON.stringify(value, null, 2));
}

/** Runtime-checked JSON helpers for tests: parse once, narrow with a guard, never cast. */
export type JsonRecord = Record<string, unknown>;

export const isRecord = (value: unknown): value is JsonRecord => typeof value === 'object' && value !== null && !Array.isArray(value);

/** `JSON.parse` that must produce an object. */
export function parseRecord(text: string): JsonRecord {
  const value: unknown = JSON.parse(text);
  if (!isRecord(value)) throw new Error(`Expected a JSON object, got ${text.slice(0, 40)}`);
  return value;
}

/** A property of an unknown value, or undefined when it is not an object. */
export const propertyOf = (value: unknown, key: string): unknown => (isRecord(value) ? value[key] : undefined);

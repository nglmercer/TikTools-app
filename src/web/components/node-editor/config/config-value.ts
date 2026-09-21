import type { JsonObject, JsonValue } from '../../../../automation/types.ts';

/** Plain-text rendering of a scalar config value for text inputs. */
export function formatValue(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'object') return '';
  return String(value);
}

/** Compact (or pretty, bounded) JSON rendering of a script-analysis value. */
export function formatEditorValue(value: JsonValue, pretty = false): string {
  if (typeof value === 'string') return JSON.stringify(value);
  if (value === null) return 'null';
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    const serialized = JSON.stringify(value, pretty ? null : undefined, pretty ? 2 : undefined) ?? String(value);
    if (pretty) return serialized.length > 8_000 ? `${serialized.slice(0, 7_997)}...` : serialized;
    return serialized.length > 140 ? `${serialized.slice(0, 137)}...` : serialized;
  } catch {
    return String(value);
  }
}

/** Parses a compare operand: empty stays empty, booleans/numbers coerce, the rest stays text. */
export function parseValue(value: string): JsonValue {
  const trimmed = value.trim();
  if (!trimmed) return '';
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const number = Number(trimmed);
  return Number.isFinite(number) && trimmed !== '' ? number : value;
}

/** Narrows unknown JSON to a plain object (never an array). */
export function isJsonObject(value: JsonValue | undefined): value is JsonObject {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

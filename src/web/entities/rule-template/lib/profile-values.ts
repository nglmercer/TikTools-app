import type { JsonObject } from '../../../../automation/types.ts';

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function readText(value: unknown, max: number): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (trimmed === '') return null;
  return Array.from(trimmed).slice(0, max).join('');
}

export function readParams(value: unknown): JsonObject | null {
  if (value === undefined) return {};
  if (!isRecord(value)) return null;
  return value as JsonObject;
}

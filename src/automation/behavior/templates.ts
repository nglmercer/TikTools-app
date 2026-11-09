import type { AutomationEvent, JsonObject, JsonValue } from '../types.ts';

const PLACEHOLDER = /\{\{\s*([^}]+?)\s*\}\}/g;

export function renderTemplate(template: string, event: AutomationEvent): string {
  return template.replace(PLACEHOLDER, (_match, rawPath: string) => readString(readEventPath(event, rawPath.trim())));
}

/**
 * Render a JSON template in a single pass. Values that land inside a JSON
 * string literal are escaped, so viewer text containing quotes, backslashes
 * or newlines (or `{{...}}` of its own) cannot break or rewrite the document.
 */
export function renderJsonTemplate(template: string, event: AutomationEvent): JsonValue {
  let rendered = '';
  let last = 0;
  let inString = false;
  let escaped = false;
  for (const match of template.matchAll(PLACEHOLDER)) {
    const literal = template.slice(last, match.index);
    for (const char of literal) {
      if (escaped) escaped = false;
      else if (inString && char === '\\') escaped = true;
      else if (char === '"') inString = !inString;
    }
    const value = readString(readEventPath(event, (match[1] ?? '').trim()));
    rendered += literal + (inString ? JSON.stringify(value).slice(1, -1) : value);
    escaped = false;
    last = match.index + match[0].length;
  }
  rendered += template.slice(last);
  try {
    const parsed = JSON.parse(rendered) as unknown;
    if (!isJsonValue(parsed)) throw new Error('JSON value is not safe.');
    return parsed;
  } catch {
    throw new Error('Template did not produce valid JSON.');
  }
}

export function readEventPath(event: AutomationEvent, path: string): JsonValue | undefined {
  const root: JsonObject = { event: event as JsonValue, data: event.data, user: event.user ?? null };
  let current: JsonValue | undefined = root;
  for (const part of path.split('.').map((entry) => entry.trim()).filter(Boolean)) {
    if (current === null || current === undefined || typeof current !== 'object') return undefined;
    // Own keys only: `constructor`/`__proto__` must not resolve to prototype members.
    if (!Object.hasOwn(current, part)) return undefined;
    current = Array.isArray(current) ? current[Number(part)] : (current as JsonObject)[part];
  }
  return current;
}

function readString(value: JsonValue | undefined): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

function isJsonValue(value: unknown): value is JsonValue {
  if (value === null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return true;
  if (Array.isArray(value)) return value.every(isJsonValue);
  return Boolean(value) && typeof value === 'object' && Object.values(value as Record<string, unknown>).every((entry) => entry === undefined || isJsonValue(entry));
}

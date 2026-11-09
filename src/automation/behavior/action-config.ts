import type { AutomationEvent, JsonObject, JsonValue } from '../types.ts';
import type { ActionField, ActionTypeDefinition, Localized } from './types.ts';
import { renderJsonTemplate, renderTemplate } from './templates.ts';

const MAX_TEXT = 4_096;
const MAX_CODE = 20_000;

/** Converts compact field descriptors into the JSON contract used by the UI. */
export function schemaForActionType(type: ActionTypeDefinition): JsonObject {
  if (type.configSchema) return type.configSchema;
  const properties: JsonObject = {};
  for (const field of type.fields ?? []) {
    properties[field.key] = {
      type: field.kind === 'number' || field.kind === 'range' ? 'number' : field.kind === 'boolean' ? 'boolean' : field.kind === 'keyvalue' ? 'object' : 'string',
      title: jsonLocalized(field.label),
      default: defaultFieldValue(field),
      enum: field.options?.map((option) => option.value),
      minimum: field.min,
      maximum: field.max,
      multipleOf: field.step,
    };
  }
  return { type: 'object', properties };
}

function jsonLocalized(value: Localized): JsonObject {
  return value;
}

/** Presentation metadata for fields that JSON Schema intentionally does not own. */
export function uiHintsForActionType(type: ActionTypeDefinition): JsonObject {
  if (type.uiHints) return type.uiHints;
  const fields: JsonObject = {};
  for (const field of type.fields ?? []) {
    fields[field.key] = {
      kind: field.kind,
      placeholder: field.placeholder,
      template: field.template,
      advanced: field.advanced,
      hint: field.hint,
      showIf: field.showIf,
      options: field.options,
    } as unknown as JsonValue;
  }
  return { fields };
}

export function defaultActionConfig(type: ActionTypeDefinition): JsonObject {
  const result: JsonObject = {};
  if (type.fields) {
    for (const field of type.fields) {
      result[field.key] = defaultFieldValue(field);
    }
    return result;
  }

  const properties = objectPropertySchemas(schemaForActionType(type));
  for (const [key, schema] of Object.entries(properties)) {
    if (schema.default !== undefined) result[key] = schema.default;
    else if (schema.type === 'object') result[key] = {};
    else if (schema.type === 'array') result[key] = [];
    else if (schema.type === 'boolean') result[key] = false;
    else result[key] = '';
  }
  return result;
}

/** Normalize and validate a saved action config without executing plugin code. */
export function normalizeActionConfig(type: ActionTypeDefinition, value: unknown): JsonObject {
  const raw = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {};

  if (type.fields) {
    const result: JsonObject = {};
    for (const field of type.fields) {
      const entry = raw[field.key];
      if (field.kind === 'keyvalue') {
        result[field.key] = stringMap(entry);
        continue;
      }
      const fallback = defaultFieldValue(field);
      const limit = field.kind === 'code' || field.kind === 'textarea' ? MAX_CODE : MAX_TEXT;
      if (field.kind === 'number' || field.kind === 'range') {
        result[field.key] = typeof entry === 'number' && Number.isFinite(entry)
          ? entry
          : typeof entry === 'string' && entry.trim() !== '' && Number.isFinite(Number(entry))
            ? entry
            : fallback;
      } else if (field.kind === 'boolean') {
        result[field.key] = typeof entry === 'boolean' ? entry : typeof entry === 'string' ? entry === 'true' : fallback;
      } else if (field.kind === 'media') {
        result[field.key] = mediaValue(entry, fallback);
      } else {
        result[field.key] = typeof entry === 'string' ? entry.slice(0, limit) : fallback;
      }
    }
    return result;
  }

  const schema = schemaForActionType(type);
  const properties = objectPropertySchemas(schema);
  const result: JsonObject = {};
  const required = Array.isArray(schema.required)
    ? schema.required.filter((entry): entry is string => typeof entry === 'string')
    : [];

  for (const name of required) {
    if (raw[name] === undefined) throw new Error(`Missing required action field: ${name}`);
  }
  for (const [name, fieldSchema] of Object.entries(properties)) {
    const entry = raw[name] ?? fieldSchema.default;
    if (entry === undefined) continue;
    assertSchemaValue(fieldSchema, entry, name);
    result[name] = limitJsonValue(entry, fieldSchema);
  }
  return result;
}

/** Resolve host-owned templates immediately before a registered action runs. */
export function resolveActionConfig(type: ActionTypeDefinition, config: JsonObject, event: AutomationEvent): JsonObject {
  const properties = objectPropertySchemas(schemaForActionType(type));
  const hints = objectPropertySchemas((uiHintsForActionType(type).fields ?? {}) as JsonObject);
  const resolved: JsonObject = { ...config };
  for (const [key, fieldSchema] of Object.entries(properties)) {
    const value = resolved[key];
    if (typeof value !== 'string') continue;
    const hint = hints[key];
    const templated = hint?.template === true || fieldSchema['x-tiktools-template'] === true;
    // JSON fields render exactly once and escape string context; rendering
    // twice would expand `{{...}}` found inside viewer-supplied values.
    if (fieldSchema.format === 'json') resolved[key] = renderJsonTemplate(value, event);
    else if (templated) resolved[key] = renderTemplate(value, event);
  }
  return resolved;
}

function defaultFieldValue(field: ActionField): JsonValue {
  if (field.kind === 'keyvalue') return stringMap(parseKeyValueDefault(field.value));
  if (field.kind === 'boolean') return field.value === 'true';
  if ((field.kind === 'number' || field.kind === 'range') && field.value.trim() !== '' && Number.isFinite(Number(field.value))) return Number(field.value);
  return field.value;
}

function objectPropertySchemas(schema: JsonObject): Record<string, JsonObject> {
  const properties = schema.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return {};
  return Object.fromEntries(Object.entries(properties).filter((entry): entry is [string, JsonObject] => isJsonObject(entry[1])));
}

function assertSchemaValue(schema: JsonObject, value: unknown, name: string): void {
  const type = schema.type;
  if (type === 'string' && typeof value !== 'string') throw new Error(`Action field ${name} must be text.`);
  if (type === 'number' && (typeof value !== 'number' || !Number.isFinite(value))) throw new Error(`Action field ${name} must be a number.`);
  if (type === 'integer' && (typeof value !== 'number' || !Number.isInteger(value))) throw new Error(`Action field ${name} must be an integer.`);
  if (type === 'boolean' && typeof value !== 'boolean') throw new Error(`Action field ${name} must be boolean.`);
  if (type === 'object' && (!value || typeof value !== 'object' || Array.isArray(value))) throw new Error(`Action field ${name} must be an object.`);
  if (type === 'array' && !Array.isArray(value)) throw new Error(`Action field ${name} must be an array.`);
  if (Array.isArray(schema.enum) && !schema.enum.some((entry) => JSON.stringify(entry) === JSON.stringify(value))) {
    throw new Error(`Action field ${name} has an invalid option.`);
  }
  if (typeof schema.minLength === 'number' && typeof value === 'string' && value.length < schema.minLength) throw new Error(`Action field ${name} is too short.`);
  if (typeof schema.maxLength === 'number' && typeof value === 'string' && value.length > schema.maxLength) throw new Error(`Action field ${name} is too long.`);
  if (typeof schema.minimum === 'number' && typeof value === 'number' && value < schema.minimum) throw new Error(`Action field ${name} is below the minimum.`);
  if (typeof schema.maximum === 'number' && typeof value === 'number' && value > schema.maximum) throw new Error(`Action field ${name} is above the maximum.`);
}

function limitJsonValue(value: unknown, schema: JsonObject): JsonValue {
  if (typeof value === 'string') return value.slice(0, schema.format === 'code' ? MAX_CODE : MAX_TEXT);
  if (typeof value === 'number' || typeof value === 'boolean' || value === null) return value;
  if (Array.isArray(value)) return value.slice(0, 64).map((entry) => limitJsonValue(entry, {}));
  if (typeof value === 'object') {
    const result: JsonObject = {};
    for (const [key, entry] of Object.entries(value as Record<string, unknown>).slice(0, 64)) {
      result[key.slice(0, 120)] = limitJsonValue(entry, {});
    }
    return result;
  }
  return null;
}

function parseKeyValueDefault(value: string): Record<string, string> {
  const entries: Record<string, string> = {};
  for (const line of value.split('\n')) {
    const index = line.indexOf('=');
    if (index <= 0) continue;
    entries[line.slice(0, index).trim()] = line.slice(index + 1).trim();
  }
  return entries;
}

function stringMap(value: unknown): JsonObject {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  const entries: JsonObject = {};
  for (const [key, entry] of Object.entries(value as Record<string, unknown>).slice(0, 64)) {
    if (!key.trim()) continue;
    entries[key.trim().slice(0, 120)] = typeof entry === 'string' ? entry.slice(0, MAX_TEXT) : String(entry ?? '');
  }
  return entries;
}

function mediaValue(value: unknown, fallback: JsonValue): JsonValue {
  if (typeof value === 'string') return value.slice(0, MAX_TEXT);
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fallback;
  const path = (value as Record<string, unknown>).path;
  if (typeof path !== 'string' || path.trim() === '') return fallback;
  // Preserve the host-provided metadata when available, but keep the same
  // bounded JSON shape used by every other action field. The Rust host still
  // rebuilds and validates metadata immediately before playback.
  return limitJsonValue(value, {});
}

function isJsonObject(value: unknown): value is JsonObject {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

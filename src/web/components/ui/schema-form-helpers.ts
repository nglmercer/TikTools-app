import type { FormSchema } from './control-events.ts';
import type { AutomationEvent, AutomationEventType, JsonObject, JsonValue } from '../../../automation/types.ts';
import type { ActionTypeDefinition } from '../../../automation/behavior/types.ts';
import { getTemplateSuggestions, type TemplateSuggestionScope } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { resolveAutocompleteSources as mergeAutocompleteSources, suggestionsFromObject } from '../autocomplete/index.ts';
import { globalSuggestionItems } from '../../features/globals.ts';
import { i18nText, type Locale } from '../../i18n.ts';

export type FieldOption = { value: string; label: string };
/** Default scope per field name so Call-URL-like forms work with zero config. */
export function defaultScopeFor(name: string, template: boolean): TemplateSuggestionScope {
  const key = name.toLowerCase();
  if (key.includes('url') || key.includes('link') || key.includes('endpoint') || key.includes('webhook')) return 'http-url';
  if (key.includes('uniqueid') || key.includes('viewer') || key.includes('user') || key === 'key' || key === 'type') return 'identity';
  if (key.includes('file') || key.includes('sound') || key.includes('audio') || key.includes('path')) return 'sound-file';
  if (key.includes('comment') || key.includes('message') || key.includes('text')) return 'text';
  if (key.includes('leftpath') || key.includes('path')) return 'compare';
  return template ? 'http-data' : 'message';
}
/**
 * Shared suggestion resolver so custom editors (e.g. the fetch layout) offer
 * exactly the same variables as the generic form: trigger scope plus any
 * object pushed via `suggestionContext`.
 */
export function resolveAutocompleteSources(args: {
  locale: Locale;
  suggestionContext?: JsonValue | AutomationEvent;
  suggestionScopes?: Partial<Record<string, TemplateSuggestionScope>>;
  eventType?: AutomationEventType;
  lastEvent?: AutomationEvent;
  templateSuggestions?: AutocompleteItem[];
  /** Runtime globals merged as `globals.*` rows (every field can use them). */
  globals?: Record<string, string>;
}): (name: string, template: boolean) => AutocompleteItem[] {
  return (name: string, template: boolean): AutocompleteItem[] => {
    const {
      locale,
      suggestionContext,
      suggestionScopes = {},
      eventType,
      lastEvent,
      templateSuggestions = [],
      globals,
    } = args;
    let contextItems: AutocompleteItem[] = [];
    if (suggestionContext !== undefined) {
      const root = suggestionContext as JsonValue;
      // `AutomationEvent` arrives as `{ type, user, data… }` — expose as `event.*`.
      if (root !== null && typeof root === 'object' && !Array.isArray(root) && 'type' in (root as JsonObject) && !('event' in (root as JsonObject))) {
        contextItems = suggestionsFromObject({ event: root } as unknown as JsonValue, '', { maxItems: 80 });
      } else {
        contextItems = suggestionsFromObject(root, '', { maxItems: 80 });
      }
    }
    const scope = suggestionScopes[name] ?? defaultScopeFor(name, template);
    const scoped = getTemplateSuggestions(eventType, locale, lastEvent, scope, undefined);
    const globalItems = globals ? globalSuggestionItems(globals, locale) : undefined;
    return mergeAutocompleteSources(scoped, contextItems, templateSuggestions, globalItems);
  };
}
export function schemaForAction(type: ActionTypeDefinition): { schema: JsonObject; uiHints?: JsonObject } {
  return {
    schema: type.configSchema ?? schemaFromFields(type),
    uiHints: type.uiHints ?? hintsFromFields(type),
  };
}
export function formSchemaFromJsonSchema(schema: JsonObject): FormSchema {
  const properties = objectProperties(schema.properties);
  const entries = Object.entries(properties).map(([name, field]) => {
    const type = field.format === 'json'
      ? 'json'
      : field.type === 'boolean'
        ? 'boolean'
        : field.type === 'integer'
          ? 'integer'
          : field.type === 'number'
            ? 'number'
            : 'string';
    return [name, { type, defaultValue: field.default }] as const;
  });
  return Object.fromEntries(entries) as FormSchema;
}
export function objectProperties(value: JsonValue | undefined): Record<string, JsonObject> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(Object.entries(value).filter((entry): entry is [string, JsonObject] => Boolean(entry[1]) && typeof entry[1] === 'object' && !Array.isArray(entry[1])));
}

export function localized(value: JsonValue | undefined, locale: Locale): string {
  return i18nText(locale, value);
}

export function applies(value: JsonValue | undefined, config: JsonObject): boolean {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return true;
  const condition = value as JsonObject;
  const key = typeof condition.key === 'string' ? condition.key : '';
  if (!key) return true;
  const current = String(config[key] ?? '');
  const equals = Array.isArray(condition.equals) ? condition.equals : [];
  const notEquals = Array.isArray(condition.notEquals) ? condition.notEquals : [];
  if (equals.length > 0 && !equals.some((entry) => String(entry) === current)) return false;
  if (notEquals.some((entry) => String(entry) === current)) return false;
  return true;
}

export function toDisplayValue(value: JsonValue | undefined, type: JsonValue | undefined): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return type === 'object' || type === 'array' ? formatJson(value) : JSON.stringify(value);
}

/**
 * Schema-aware gate for values entering settings state. A DOM Event is not a
 * valid JsonValue, yet a bubbled/fallthrough listener can deliver one at
 * runtime despite the types. Reject by declared scalar type so event objects
 * (or any mistyped payload) can never be stored — and later serialized into
 * controls — instead of a real setting. Never logs the value: schema fields
 * can contain secrets.
 */
export function acceptSchemaFieldValue(field: JsonObject, next: unknown): next is JsonValue {
  switch (field.type) {
    case 'string':
      return typeof next === 'string';
    case 'boolean':
      return typeof next === 'boolean';
    case 'number':
    case 'integer':
      return typeof next === 'number';
    default:
      return true;
  }
}

export function formatJson(value: JsonValue | undefined): string {
  if (value === undefined) return '';
  try { return JSON.stringify(value, null, 2) ?? ''; } catch { return ''; }
}

/**
 * Select display value for schema enum/dynamic option lists. Defaults are
 * applied deliberately at the settings/form state boundary
 * (`withSchemaDefaults`, plus the host overlay), so this only fills a
 * genuinely absent value (`undefined`/`null`) with the schema default. An
 * invalid stored value stays observable instead of silently masquerading as
 * the default — display substitution must never hide stale state.
 */
export function resolveSelectDisplayValue(
  value: JsonValue | undefined,
  displayValue: string,
  schemaDefault: string | undefined,
  optionValues: ReadonlySet<string>,
): string {
  const hasExplicitValue = value !== undefined && value !== null;
  if (!hasExplicitValue && schemaDefault !== undefined && optionValues.has(schemaDefault)) {
    return schemaDefault;
  }
  return displayValue;
}

export function schemaFromFields(type: ActionTypeDefinition): JsonObject {
  const properties: JsonObject = {};
  for (const field of type.fields ?? []) properties[field.key] = { type: field.kind === 'number' || field.kind === 'range' ? 'number' : field.kind === 'boolean' ? 'boolean' : field.kind === 'keyvalue' ? 'object' : 'string', title: field.label, default: field.value, minimum: field.min, maximum: field.max, multipleOf: field.step };
  return { type: 'object', properties };
}

export function hintsFromFields(type: ActionTypeDefinition): JsonObject {
  const fields: JsonObject = {};
  for (const field of type.fields ?? []) fields[field.key] = { kind: field.kind, placeholder: field.placeholder, template: field.template, advanced: field.advanced, hint: field.hint, showIf: field.showIf, options: field.options, optionsFrom: field.optionsFrom } as unknown as JsonValue;
  return { fields };
}

import type { AutomationEvent, JsonValue } from '../../../automation/types.ts';
import {
  allRegistryFields,
  fieldsForEventType,
  registryEntryFor,
  type RegistryField,
} from '../../../automation/event-registry.ts';
import { mergeSuggestions, suggestionsFromObject, type AutocompleteItem } from '../autocomplete/index.ts';
import type { Locale } from '../../i18n.ts';

export type TemplateSuggestionScope =
  | 'message'
  | 'identity'
  | 'text'
  | 'sound-file'
  | 'http-url'
  | 'http-data'
  | 'compare';

type ObservedPathMode = 'all' | 'identity' | 'text' | 'path';

/**
 * Declarative input contracts. A form chooses one scope — a filter over the
 * event registry — instead of receiving hardcoded path lists. The candidates
 * always come from the generated automation registry (the native Rust event
 * boundary) plus
 * whatever the last live event actually carried.
 */
export const TEMPLATE_INPUT_DEFINITIONS: Record<TemplateSuggestionScope, { observed: ObservedPathMode }> = {
  message: { observed: 'all' },
  identity: { observed: 'identity' },
  text: { observed: 'text' },
  'sound-file': { observed: 'path' },
  // URL destination: the host itself stays literal (engine allowlist), but the
  // path/query can embed any event variable (`?user={{ event.user.uniqueId }}`).
  // Filtering to path-like keys here left the field nearly empty, so offer all.
  'http-url': { observed: 'all' },
  'http-data': { observed: 'all' },
  compare: { observed: 'all' },
};

export function getTemplateSuggestions(
  eventType: string | undefined,
  locale: Locale,
  lastEvent?: AutomationEvent,
  scope: TemplateSuggestionScope = 'message',
  extraContext?: JsonValue,
): AutocompleteItem[] {
  const definition = TEMPLATE_INPUT_DEFINITIONS[scope];
  const matchingLastEvent = lastEvent && (!eventType || lastEvent.type === eventType) ? lastEvent : undefined;
  const registryFields = eventType ? fieldsForEventType(eventType) : allRegistryFields();
  const base: AutocompleteItem[] = registryFields
    .filter((field) => matchesPathScope(field.path, undefined, definition.observed))
    .map((field) => toSuggestion(field, locale, eventType, matchingLastEvent ? readTemplatePath(matchingLastEvent, field.path) : undefined));

  // Existent data: paths the last live event really carried (custom payloads,
  // plugin emits) that the static registry cannot know about.
  const observed: AutocompleteItem[] = matchingLastEvent
    ? flattenJsonPaths(matchingLastEvent, 'event')
      .filter((path) => matchesPathScope(path, readTemplatePath(matchingLastEvent, path), definition.observed))
      .filter((path) => !base.some((entry) => entry.value === path))
      .map((path) => {
        const liveValue = readTemplatePath(matchingLastEvent, path);
        return {
          value: path,
          label: humanizePath(path),
          kind: inferSuggestionKind(liveValue),
          detail: inferSuggestionKind(liveValue),
          documentation: `${humanizePath(path)} · ${path}`,
          preview: formatTemplateValue(liveValue),
        };
      })
    : [];

  const merged = mergeSuggestions(base, observed);
  if (extraContext === undefined) return merged;
  // Generic: push any object as extra autocomplete items (custom schema/event).
  const extra = suggestionsFromObject(extraContext, 'event', { maxItems: 60 });
  return mergeSuggestions(merged, extra);
}

function toSuggestion(
  field: RegistryField,
  locale: Locale,
  eventType: string | undefined,
  liveValue: JsonValue | undefined,
): AutocompleteItem {
  const label = field.label[locale === 'es' ? 'es' : 'en'];
  const hint = field.hint?.[locale === 'es' ? 'es' : 'en'];
  const source = sourceDetail(eventType, field);
  return {
    value: field.path,
    label,
    kind: field.kind,
    detail: field.tsType,
    documentation: [hint ?? `${label} · ${field.path}`, source].filter(Boolean).join('\n'),
    preview: liveValue === undefined ? undefined : formatTemplateValue(liveValue),
  };
}

/** Which native event field a registry entry derives from, for the hover card. */
function sourceDetail(eventType: string | undefined, field: RegistryField): string | undefined {
  if (!eventType || !field.sourceField) return undefined;
  const entry = registryEntryFor(eventType);
  if (!entry || entry.sourceInterface === '-') return undefined;
  const sourceField = entry.sourceFields.find((candidate) => candidate.name === field.sourceField);
  const tsType = sourceField ? sourceField.tsType : field.tsType;
  return `native ${entry.sourceInterface}.${field.sourceField}: ${tsType}`;
}

function inferSuggestionKind(value: JsonValue | undefined): AutocompleteItem['kind'] {
  if (value === undefined) return 'unknown';
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  switch (typeof value) {
    case 'string': return 'string';
    case 'number': return 'number';
    case 'boolean': return 'boolean';
    case 'object': return 'object';
    default: return 'unknown';
  }
}

function matchesPathScope(path: string, value: JsonValue | undefined, mode: ObservedPathMode): boolean {
  if (mode === 'all') return true;
  const key = path.split('.').pop()?.toLowerCase() ?? '';
  if (mode === 'identity') {
    return /^(uniqueid|userid|nickname|username|roomid|creator|user)$/.test(key);
  }
  if (mode === 'text') {
    return /^(comment|message|text|nickname|giftname|action|reason|currencyname|name)$/.test(key)
      || (typeof value === 'string' && !/^(method|timestamp|type)$/.test(key));
  }
  return /(?:path|file|sound|audio|url|uri|asset|link|endpoint|webhook)/.test(key)
    || (typeof value === 'string' && /\.(wav|mp3|ogg|m4a|flac|aac)(?:[?#].*)?$/i.test(value));
}

export function flattenJsonPaths(value: JsonValue, prefix: string, depth = 0): string[] {
  if (value === null || typeof value !== 'object' || Array.isArray(value) || depth >= 4) return [prefix];
  const entries = Object.entries(value).filter(([, entry]) => entry !== undefined);
  if (entries.length === 0) return [prefix];
  return entries.flatMap(([key, entry]) => flattenJsonPaths(entry ?? null, `${prefix}.${key}`, depth + 1));
}

export function readTemplatePath(event: AutomationEvent, path: string): JsonValue | undefined {
  const parts = path.split('.').filter(Boolean);
  if (parts[0] === 'event') parts.shift();
  let current: JsonValue | undefined = event;
  for (const part of parts) {
    if (current === undefined || current === null || typeof current !== 'object') return undefined;
    if (Array.isArray(current)) {
      const index = Number(part);
      if (!Number.isInteger(index)) return undefined;
      current = current[index];
    } else {
      current = current[part];
    }
  }
  return current;
}

export function formatTemplateValue(value: JsonValue | undefined): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value === 'string') return truncate(`"${value}"`);
  if (value === null) return 'null';
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return truncate(JSON.stringify(value) ?? String(value));
  } catch {
    return String(value);
  }
}

function humanizePath(path: string): string {
  const last = path.split('.').pop() ?? path;
  return last.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/^./, (character) => character.toUpperCase());
}

function truncate(value: string): string {
  return value.length > 96 ? `${value.slice(0, 93)}...` : value;
}

/* ------------------------------------------------------------------ */
/* Fetch URL presets (quick targets for the Call-URL endpoint field).  */
/* ------------------------------------------------------------------ */

export type FetchUrlTemplate = {
  /** Stable id; re-registering the same id replaces the preset. */
  id: string;
  /** Short chip label, e.g. `localhost:3000`. */
  label: string;
  /** Full URL to apply, e.g. `http://localhost:3000/`. */
  url: string;
  /** Tooltip shown on hover. */
  hint?: string;
};

const BUILTIN_FETCH_URL_TEMPLATES: FetchUrlTemplate[] = [
  { id: 'local-node', label: 'localhost:3000', url: 'http://localhost:3000/', hint: 'Local dev server (Node, Vite…)' },
  { id: 'local-py', label: '127.0.0.1:8000', url: 'http://127.0.0.1:8000/', hint: 'Local dev server (Python, …)' },
  { id: 'local-lan', label: '192.168.1.100:3000', url: 'http://192.168.1.100:3000/', hint: 'Example host on your LAN — edit the IP' },
  { id: 'remote-https', label: 'https://', url: 'https://', hint: 'Public webhook (Discord, StreamElements, …)' },
];

/** Host/plugin-registered presets. Import this module and call
 * `registerFetchUrlTemplate({ id, label, url })` to add project targets. */
const customFetchUrlTemplates = new Map<string, FetchUrlTemplate>();

export function registerFetchUrlTemplate(template: FetchUrlTemplate): void {
  const id = template.id.trim();
  const url = template.url.trim();
  if (!id || !url) throw new Error('A URL template needs an id and a url.');
  if (!/^https?:\/\//i.test(url)) throw new Error('A URL template must start with http:// or https://.');
  customFetchUrlTemplates.set(id, {
    id: id.slice(0, 64),
    label: template.label.trim().slice(0, 48) || url,
    url: url.slice(0, 512),
    hint: template.hint?.slice(0, 160),
  });
}

export function getFetchUrlTemplates(): FetchUrlTemplate[] {
  return [...BUILTIN_FETCH_URL_TEMPLATES, ...customFetchUrlTemplates.values()];
}

/**
 * True when the URL points at this machine / LAN. Mirrors the engine's
 * `isPrivateHostname` (see `src/automation/services/http-service.ts`): such
 * targets only run with “Allow local network” enabled.
 */
export function isLocalFetchUrl(rawUrl: string): boolean {
  const hostname = extractFetchHostname(rawUrl);
  if (
    hostname === 'localhost'
    || hostname.endsWith('.localhost')
    || hostname.endsWith('.local')
    || hostname === '::1'
    || hostname === '0:0:0:0:0:0:0:1'
    || hostname === '::'
  ) return true;
  if (hostname.includes(':')) return true;
  const octets = hostname.split('.').map(Number);
  if (octets.length !== 4 || octets.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) return false;
  const [first = -1, second = -1] = octets;
  return first === 0
    || first === 10
    || first === 127
    || (first === 100 && second >= 64 && second <= 127)
    || (first === 169 && second === 254)
    || (first === 172 && second >= 16 && second <= 31)
    || (first === 192 && second === 168);
}

/** Host without port/brackets, lowercased. Uses the URL parser first so
 * bracketed IPv6 (`http://[::1]:3000/`) survives; falls back to a
 * bracket-aware regex split for half-typed input while editing. */
function extractFetchHostname(rawUrl: string): string {
  const trimmed = rawUrl.trim();
  try {
    const hostname = new URL(trimmed).hostname.toLowerCase().replace(/^\[|\]$/g, '');
    if (hostname) return hostname;
  } catch {
    // Half-typed while editing (`https://`, `http://local…`) — fall through.
  }
  const host = /^https?:\/\/([^/?#\s]+)/i.exec(trimmed)?.[1]?.toLowerCase() ?? '';
  if (!host) return '';
  if (host.startsWith('[')) return /^\[([^\]]+)\]/.exec(host)?.[1] ?? '';
  return host.split(':')[0] ?? '';
}

/**
 * Apply a preset keeping the user's path/query: only the `scheme://host`
 * origin is swapped. With no origin yet (empty field, `https://`), the
 * preset URL is used as-is.
 */
export function applyFetchUrlTemplate(current: string, templateUrl: string): string {
  const origin = /^https?:\/\/[^/?#\s]*/i.exec(templateUrl.trim())?.[0]?.replace(/\/+$/, '') ?? templateUrl.trim();
  const match = /^https?:\/\/[^/?#\s]*/i.exec(current);
  if (!match) return templateUrl;
  const rest = current.slice(match[0].length);
  if (!rest) return `${origin}/`;
  if (/^[/?#]/.test(rest)) return `${origin}${rest}`;
  return `${origin}/${rest}`;
}

import type { AutomationEventType, JsonObject, JsonValue } from '../types.ts';
import { BUILTIN_EVENT_TYPES } from '../contracts/events.ts';
import type {
  ActionTypeDefinition,
  EventFilter,
  FilterOperator,
  LiveAction,
  LiveEvent,
} from './types.ts';

/** Events an event record can listen to. Kept explicit so the picker cannot offer a type the host never publishes. */
export const BEHAVIOR_TRIGGERS = BUILTIN_EVENT_TYPES;

const OPERATORS: FilterOperator[] = [
  'gte',
  'gt',
  'lte',
  'lt',
  'eq',
  'neq',
  'contains',
  'starts-with',
  'in',
  'is-true',
  'is-false',
];

const MAX_CODE = 20_000;

/** Bridge-only normalization for action ids that may belong to an unloaded plugin. */
export function normalizeUnresolvedAction(value: unknown): LiveAction {
  const raw = record(value, 'action');
  const id = identifier(raw.id, 'action.id');
  const typeId = text(raw.typeId, 'action.typeId');
  const config = raw.config && typeof raw.config === 'object' && !Array.isArray(raw.config)
    ? limitUnknownConfig(raw.config as Record<string, unknown>)
    : {};
  return {
    schemaVersion: raw.schemaVersion === 2 ? 2 : 1,
    id,
    name: text(raw.name, 'action.name').slice(0, 120),
    typeId,
    enabled: raw.enabled !== false,
    config,
  };
}

/** `extraTriggers` carries the plugin-declared types from the snapshot. */
export function normalizeEvent(value: unknown, extraTriggers: string[] = []): LiveEvent {
  const raw = record(value, 'event');
  const trigger = raw.trigger;
  if (typeof trigger !== 'string' || (!BEHAVIOR_TRIGGERS.includes(trigger as AutomationEventType) && !extraTriggers.includes(trigger))) {
    throw new Error(`Unknown event trigger: ${String(trigger)}`);
  }

  const rawFilters = Array.isArray(raw.filters) ? raw.filters : [];
  const filters: EventFilter[] = [];
  for (const entry of rawFilters.slice(0, 12)) {
    const filter = normalizeFilter(entry);
    if (filter) filters.push(filter);
  }

  // Flat OR groups, bounded like the host matcher: 4 groups of 12 filters.
  // Empty groups are dropped — they would match vacuously.
  const anyOf: EventFilter[][] = [];
  if (Array.isArray(raw.anyOf)) {
    for (const group of raw.anyOf.slice(0, 4)) {
      if (!Array.isArray(group)) continue;
      const normalized = group
        .slice(0, 12)
        .map((entry) => normalizeFilter(entry))
        .filter((entry): entry is EventFilter => entry !== null);
      if (normalized.length > 0) anyOf.push(normalized);
    }
  }

  const actionIds = (Array.isArray(raw.actionIds) ? raw.actionIds : [])
    .filter((entry): entry is string => typeof entry === 'string')
    .slice(0, 16);

  return {
    schemaVersion: 1,
    id: identifier(raw.id, 'event.id'),
    name: text(raw.name, 'event.name').slice(0, 120),
    enabled: raw.enabled === true,
    trigger: trigger as AutomationEventType,
    filters,
    ...(raw.matchMode === 'or' ? { matchMode: 'or' as const } : {}),
    ...(anyOf.length > 0 ? { anyOf } : {}),
    cooldownMs: clamp(numberOr(raw.cooldownMs, 0), 0, 24 * 60 * 60 * 1000),
    cooldownScope: raw.cooldownScope === 'global' ? 'global' : 'user',
    actionIds,
    runMode: raw.runMode === 'random' ? 'random' : 'all',
  };
}

const VALUE_OPERATORS: ReadonlySet<FilterOperator> = new Set([
  'gte', 'gt', 'lte', 'lt', 'eq', 'neq', 'contains', 'starts-with', 'in',
]);
const CAPTURE_ALIAS = /^[a-zA-Z][a-zA-Z0-9_]{0,31}$/;
const RESERVED_CAPTURE_KEYS = new Set(['match', 'event', 'globals', 'data', 'user']);

function normalizeCaptureAlias(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  const alias = value.trim();
  if (!CAPTURE_ALIAS.test(alias) || RESERVED_CAPTURE_KEYS.has(alias)) return undefined;
  return alias;
}

function normalizeFilter(value: unknown): EventFilter | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const raw = value as Record<string, unknown>;
  const path = typeof raw.path === 'string' ? raw.path.trim().replace(/^\{\{\s*|\s*\}\}$/g, '') : '';
  if (!path) return null;
  const operator = OPERATORS.find((entry) => entry === raw.operator) ?? 'eq';
  const takesValue = VALUE_OPERATORS.has(operator);
  const values = Array.isArray(raw.values)
    ? raw.values.filter((entry): entry is string => typeof entry === 'string').slice(0, 24)
    : undefined;
  // `in` reads `values` only; other value operators fall back to `value`
  // when no list is stored. Boolean assertions take neither.
  const keptValues = takesValue ? (operator === 'in' ? values ?? [] : values) : undefined;
  // `neq` with a list means «none of»: `any` is normalized to `all` so a
  // stored draft cannot express the almost-always-true form.
  const quantifier =
    takesValue && operator !== 'in' && (raw.quantifier === 'all' || operator === 'neq' && (keptValues?.length ?? 0) > 0)
      ? 'all'
      : undefined;
  const alias = takesValue ? normalizeCaptureAlias(raw.as) : undefined;

  return {
    path: path.slice(0, 200),
    operator,
    value: typeof raw.value === 'string' ? raw.value.slice(0, 200) : '',
    values: keptValues && keptValues.length > 0 ? keptValues : operator === 'in' ? [] : undefined,
    quantifier,
    as: alias,
  };
}

/**
 * Permissions are computed from the saved action, never typed by hand, so the
 * editor shows exactly what the engine will allow.
 */
export function deriveActionPermissions(action: LiveAction, type?: ActionTypeDefinition): { network: string[]; capabilities: string[]; localNetwork: boolean } {
  const capabilities = type ? [...type.requiredCapabilities] : [];
  const network: string[] = [];
  let localNetwork = false;

  if (action.typeId === 'core.fetch') {
    const host = hostFromUrlTemplate(readString(action.config.url));
    if (host) network.push(host);
    localNetwork = readString(action.config.allowPrivateNetwork) === 'true';
  }

  if (action.typeId === 'core.code') {
    const source = readString(action.config.source);
    if (/\bfetch\b/.test(source)) capabilities.push('http.request');
    if (/\bemit\b/.test(source)) capabilities.push('event.emit');
    for (const host of hostsInSource(source)) network.push(host);
  }

  return {
    network: [...new Set(network)],
    capabilities: [...new Set(capabilities)],
    localNetwork,
  };
}

/**
 * Reads the host out of a URL that may still contain `{{ }}` placeholders. A
 * templated host is refused: the allowlist has to be knowable before the event
 * arrives.
 */
export function hostFromUrlTemplate(url: string): string | null {
  const match = /^https?:\/\/([^/?#\s]+)/i.exec(url.trim());
  const host = match?.[1];
  if (!host || host.includes('{{')) return null;
  return host.toLowerCase();
}

function hostsInSource(source: string): string[] {
  const hosts: string[] = [];
  const pattern = /https?:\/\/([a-z0-9.-]+)/gi;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    if (match[1]) hosts.push(match[1].toLowerCase());
  }
  return hosts;
}

export function readString(value: JsonValue | undefined): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (!Array.isArray(value) && typeof value.path === 'string') return value.path;
  return JSON.stringify(value);
}

export function readStringMap(value: JsonValue | undefined): Record<string, string> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  const entries: Record<string, string> = {};
  for (const [key, entry] of Object.entries(value)) {
    if (entry === undefined) continue;
    entries[key] = readString(entry);
  }
  return entries;
}

function limitUnknownConfig(value: Record<string, unknown>): JsonObject {
  const config: JsonObject = {};
  for (const [key, entry] of Object.entries(value).slice(0, 64)) {
    if (!key.trim()) continue;
    if (entry === null || typeof entry === 'string' || typeof entry === 'number' || typeof entry === 'boolean') config[key.slice(0, 120)] = typeof entry === 'string' ? entry.slice(0, MAX_CODE) : entry;
    else if (Array.isArray(entry)) config[key.slice(0, 120)] = entry.slice(0, 64).filter((item): item is JsonValue => item === null || typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean');
    else if (typeof entry === 'object') config[key.slice(0, 120)] = limitUnknownConfig(entry as Record<string, unknown>);
  }
  return config;
}

/** Internal event names stay dotted lower-case so they cannot collide with TikTok types by accident. */
export function normalizeEmitType(value: string): string {
  const cleaned = value.trim().toLowerCase().replace(/[^a-z0-9._-]/g, '');
  if (!cleaned) throw new Error('An internal event needs a name.');
  return cleaned.slice(0, 64);
}

function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`Invalid ${label}: an object was expected.`);
  }
  return value as Record<string, unknown>;
}

function text(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) throw new Error(`Invalid ${label}: text was expected.`);
  return value.trim();
}

function identifier(value: unknown, label: string): string {
  const id = text(value, label);
  if (!/^[a-z0-9][a-z0-9._-]{1,127}$/i.test(id)) throw new Error(`Invalid ${label}: ${id}`);
  return id;
}

function numberOr(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function createActionId(): string {
  return `act-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export function createEventId(): string {
  return `evt-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

import {
  EVENT_REGISTRY_VERSION as GENERATED_EVENT_REGISTRY_VERSION,
  GENERATED_EVENT_REGISTRY,
} from './contracts/generated/event-registry.generated.ts';
import type { AutomationEvent, JsonObject } from './types.ts';
import type { PluginEventType } from './behavior/types.ts';

/**
 * Runtime loader for the generated event registry
 * (`src/automation/contracts/generated/event-registry.generated.ts`, emitted
 * from the Rust automation schema).
 *
 * This module is UI-agnostic on purpose: `src/automation` never imports from
 * `src/web`. It exposes registry fields (path, TS type, kind, labels, live
 * sample) and sample events. Every autocomplete list, condition field and
 * sample event in the app derives from here — see
 * `web/components/node-editor/template-suggestions.ts` and
 * `automation/behavior/fields.ts`.
 */

export type RegistryFieldKind = 'string' | 'number' | 'boolean' | 'object' | 'array' | 'null' | 'unknown';

export interface RegistryField {
  path: string;
  tsType: string;
  kind: RegistryFieldKind;
  optional: boolean;
  options?: Array<{ value: string; label: { en: string; es: string } }>;

  i18key?: string;
  label: { en: string; es: string };
  hint?: { en: string; es: string };
  sample?: unknown;
  sourceField?: string;
}

export interface RegistrySourceField {
  name: string;
  tsType: string;
  optional: boolean;
}

export interface RegistryEventEntry {
  dataInterface: string;
  sourceInterface: string;
  note?: string;
  sampleEvent: AutomationEvent;
  fields: RegistryField[];
  sourceFields: RegistrySourceField[];
}

interface RegistryFile {
  version: number;
  generatedBy: string;
  generatedFrom: string[];
  events: Record<string, RegistryEventEntry>;
}

const REGISTRY = GENERATED_EVENT_REGISTRY as unknown as RegistryFile;

export const EVENT_REGISTRY_VERSION = GENERATED_EVENT_REGISTRY_VERSION;

/**
 * Plugin-declared event types merged from the behavior snapshot (see
 * setPluginEventTypes). Built-in entries stay frozen from JSON; plugin
 * entries live here so conditions, filters, samples, and autocomplete treat
 * both uniformly without touching generated code.
 */
const PLUGIN_OVERLAY = new Map<string, { entry: RegistryEventEntry; meta: PluginEventType }>();

function overlayEntryFor(type: PluginEventType): RegistryEventEntry {
  const pluginId = type.source.kind === 'plugin' ? type.source.pluginId : 'unknown';
  const fields: RegistryField[] = (type.fields ?? [])
    .filter((field) => typeof field.path === 'string' && (field.path.startsWith('event.data.') || field.path.startsWith('event.user.')))
    .map((field) => {
      const leaf = field.path.split('.').pop() ?? field.path;
      const kind: RegistryFieldKind = field.kind === 'number' ? 'number' : field.kind === 'boolean' ? 'boolean' : 'string';
      const options = (field.options ?? [])
        .filter((option) => option && typeof option.value === 'string')
        .map((option) => ({
          value: option.value,
          label: { en: option.label?.default ?? option.value, es: option.label?.default ?? option.value },
        }));
      return {
        path: field.path,
        tsType: kind,
        kind,
        optional: true,
        options: options.length > 0 ? options : undefined,
        i18key: field.label?.i18key,
        label: { en: field.label?.default ?? leaf, es: field.label?.default ?? leaf },
        hint: field.hint ? { en: field.hint.default, es: field.hint.default } : undefined,
      };
    });
  const data = type.sample && typeof type.sample === 'object' && !Array.isArray(type.sample)
    ? (type.sample as JsonObject)
    : {};
  return {
    dataInterface: 'JsonObject',
    sourceInterface: 'plugin:' + pluginId,
    note: 'Declared by plugin ' + pluginId,
    sampleEvent: {
      id: 'sample-event',
      type: type.type,
      timestamp: Date.now(),
      user: { uniqueId: 'usuario_demo' },
      data,
    } as AutomationEvent,
    fields,
    sourceFields: [],
  };
}

/** Replace the plugin overlay (called on every behavior snapshot). */
export function setPluginEventTypes(types: PluginEventType[]): void {
  PLUGIN_OVERLAY.clear();
  for (const type of types) {
    if (!type || typeof type.type !== 'string' || !type.type) continue;
    PLUGIN_OVERLAY.set(type.type, { entry: overlayEntryFor(type), meta: type });
  }
}

/** Raw plugin declarations currently in the overlay. */
export function pluginEventTypes(): PluginEventType[] {
  return [...PLUGIN_OVERLAY.values()].map((item) => item.meta);
}

/** Every event type the registry knows, in generation order. */

export function registryEventTypes(): string[] {
  return [...Object.keys(REGISTRY.events), ...PLUGIN_OVERLAY.keys()];
}

export function registryEntryFor(eventType: string): RegistryEventEntry | undefined {
  return REGISTRY.events[eventType] ?? PLUGIN_OVERLAY.get(eventType)?.entry;
}

/** All `event.*` paths (with types, labels, samples) for one trigger. */
export function fieldsForEventType(eventType: string): RegistryField[] {
  return registryEntryFor(eventType)?.fields ?? [];
}

/** Union of every trigger's fields, deduplicated by path. */
export function allRegistryFields(): RegistryField[] {
  const seen = new Set<string>();
  const out: RegistryField[] = [];
  const entries = [
    ...Object.values(REGISTRY.events),
    ...[...PLUGIN_OVERLAY.values()].map(({ entry }) => entry),
  ];
  for (const entry of entries) {
    for (const field of entry.fields) {
      if (seen.has(field.path)) continue;
      seen.add(field.path);
      out.push(field);
    }
  }
  return out;
}

/** Fresh sample event for a trigger (timestamp refreshed, safe to mutate). */
export function sampleEventForType(eventType: string): AutomationEvent {
  const entry = registryEntryFor(eventType);
  if (!entry) {
    return {
      id: 'sample-event', type: eventType, timestamp: Date.now(),
      user: { uniqueId: 'usuario_demo' }, data: {},
    } as AutomationEvent;
  }
  const clone = structuredClone(entry.sampleEvent) as AutomationEvent;
  clone.timestamp = Date.now();
  return clone;
}

/** The `data` payload of the sample event, for script/language services. */
export function sampleDataForType(eventType: string): JsonObject {
  const event = sampleEventForType(eventType);
  const data = event.data;
  return (data !== null && typeof data === 'object' && !Array.isArray(data) ? data : {}) as JsonObject;
}

/** True when the registry documents a path for a trigger (drift guard). */
export function registryHasPath(eventType: string, path: string): boolean {
  return fieldsForEventType(eventType).some((field) => field.path === path);
}

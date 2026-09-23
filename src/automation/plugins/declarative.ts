import type {
  ActionSource,
  Localized,
  PluginAutocompleteContribution,
  PluginAutocompleteField,
  PluginPageDescriptor,
  PluginPageSection,
  PluginPageSectionKind,
  PluginTemplateDescriptor,
} from '../behavior/types.ts';
import type { JsonObject, JsonValue } from '../types.ts';
// Leaf imports only (see behavior/types.ts): these modules never import
// automation, so the legacy snapshot merge cannot cycle.
import type { PluginUiDescriptor } from '../../plugin-ui/contracts.ts';
import { normalizeUiDescriptor } from '../../plugin-ui/normalize.ts';

/** Prefix for host-resolved plugin option sources. */
export const OPTION_SOURCE_PREFIX = 'plugin-action-options:';

/** Placeholder the host emits for secret settings; round-trips untouched. */
export const SECRET_PLACEHOLDER = '••••••••';

/** Fixed widget set a plugin page section may use. */
export const PLUGIN_PAGE_SECTION_KINDS: readonly PluginPageSectionKind[] = [
  'text',
  'form',
  'connection',
  'list',
  'tts',
];

/** Builds the canonical option source id for an action type field. */
export function optionSourceId(actionType: string, field: string): string {
  return `${OPTION_SOURCE_PREFIX}${actionType}:${field}`;
}

/** Splits a canonical option source id into action type and field. */
export function parseOptionSourceId(source: string): { actionType: string; field: string } | undefined {
  if (!source.startsWith(OPTION_SOURCE_PREFIX)) return undefined;
  const rest = source.slice(OPTION_SOURCE_PREFIX.length);
  const separator = rest.indexOf(':');
  if (separator <= 0 || separator !== rest.lastIndexOf(':')) return undefined;
  const actionType = rest.slice(0, separator);
  const field = rest.slice(separator + 1);
  if (!actionType || !field) return undefined;
  return { actionType, field };
}

const TOKEN_PATTERN = /^[a-z][a-z0-9._-]{0,127}$/;
const FIELD_PATTERN = /^[a-zA-Z][a-zA-Z0-9._-]{0,127}$/;

/**
 * Normalizes an `optionsFrom` marker to a requestable source id. Accepts the
 * canonical string plus the object form
 * `{ source: 'plugin-action-options', actionType, field }`. A page-supplied
 * `optionsUrl` is deliberately ignored: only manifest-declared endpoints are
 * ever fetched, so the WebView cannot steer the host at arbitrary URLs.
 */
export function normalizeOptionsFrom(value: JsonValue | undefined): string | undefined {
  if (typeof value === 'string') {
    const source = value.trim();
    if (!source || source.length > 256) return undefined;
    if (TOKEN_PATTERN.test(source)) return source;
    const parsed = parseOptionSourceId(source);
    if (!parsed) return undefined;
    if (!TOKEN_PATTERN.test(parsed.actionType) || !FIELD_PATTERN.test(parsed.field)) return undefined;
    return source;
  }
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    const record = value as JsonObject;
    if (record.source !== 'plugin-action-options') return undefined;
    const actionType = record.actionType;
    const field = record.field;
    if (typeof actionType !== 'string' || typeof field !== 'string') return undefined;
    if (!TOKEN_PATTERN.test(actionType) || !FIELD_PATTERN.test(field)) return undefined;
    return optionSourceId(actionType, field);
  }
  return undefined;
}

/**
 * Fields whose select options come from the host on demand (`optionsFrom`
 * markers in `uiHints.fields`). Shared by action editors, settings forms,
 * and plugin pages.
 */
export function optionFields(uiHints: JsonObject | undefined): Array<{ key: string; source: string }> {
  if (!uiHints || typeof uiHints !== 'object' || Array.isArray(uiHints)) return [];
  const fields = uiHints.fields;
  if (!fields || typeof fields !== 'object' || Array.isArray(fields)) return [];
  const result: Array<{ key: string; source: string }> = [];
  for (const [key, hint] of Object.entries(fields)) {
    if (hint && typeof hint === 'object' && !Array.isArray(hint)) {
      const source = normalizeOptionsFrom((hint as JsonObject).optionsFrom as JsonValue);
      if (source) result.push({ key, source });
    }
  }
  return result;
}

/** True when a schema field or UI hint marks a setting as secret. */
export function isSecretField(schema: JsonValue | undefined, hint: JsonValue | undefined): boolean {
  for (const candidate of [schema, hint]) {
    if (candidate && typeof candidate === 'object' && !Array.isArray(candidate)) {
      if ((candidate as JsonObject).secret === true) return true;
    }
  }
  return false;
}

/** Host-side namespaced template id: `<pluginId>/<templateId>`. */
export function namespacedTemplateId(pluginId: string, templateId: string): string {
  return `${pluginId}/${templateId}`;
}

/** Splits a namespaced template id back into plugin and template ids. */
export function splitTemplateId(id: string): { pluginId: string; templateId: string } | undefined {
  const separator = id.indexOf('/');
  if (separator <= 0 || separator === id.length - 1) return undefined;
  return { pluginId: id.slice(0, separator), templateId: id.slice(separator + 1) };
}

/** Navigation tab id for a plugin page: `plugin:<pluginId>:<pageId>`. */
export function pluginNavId(pluginId: string, pageId: string): `plugin:${string}:${string}` {
  return `plugin:${pluginId}:${pageId}`;
}

/** Splits a plugin navigation tab id into plugin and page ids. */
export function parsePluginNavId(tab: string): { pluginId: string; pageId: string } | undefined {
  if (!tab.startsWith('plugin:')) return undefined;
  const rest = tab.slice('plugin:'.length);
  const separator = rest.indexOf(':');
  if (separator <= 0 || separator !== rest.lastIndexOf(':')) return undefined;
  const pluginId = rest.slice(0, separator);
  const pageId = rest.slice(separator + 1);
  if (!pluginId || !pageId) return undefined;
  return { pluginId, pageId };
}

function isRecord(value: JsonValue | undefined): value is JsonObject {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function readLocalized(value: JsonValue | undefined): Localized | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.default !== 'string' || !value.default.trim()) return undefined;
  return {
    default: value.default,
    i18key: typeof value.i18key === 'string' ? value.i18key : '',
  };
}

function readNodeList(value: JsonValue | undefined): Array<{ type: string; config?: JsonObject }> | undefined {
  if (!Array.isArray(value) || value.length === 0 || value.length > 16) return undefined;
  const nodes: Array<{ type: string; config?: JsonObject }> = [];
  for (const entry of value) {
    if (!isRecord(entry) || typeof entry.type !== 'string' || !entry.type) return undefined;
    const node: { type: string; config?: JsonObject } = { type: entry.type };
    if (entry.config !== undefined) {
      if (!isRecord(entry.config)) return undefined;
      node.config = entry.config;
    }
    nodes.push(node);
  }
  return nodes;
}

/**
 * Converts a host-stamped template entry to a typed descriptor. The host
 * already validated the shape; this guards the renderer against malformed
 * snapshots (old hosts, corrupt caches) by rejecting them instead of
 * rendering partial forms.
 */
export function toPluginTemplateDescriptor(value: JsonValue): PluginTemplateDescriptor | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.id !== 'string' || !value.id) return undefined;
  if (typeof value.pluginId !== 'string' || !value.pluginId) return undefined;
  const title = readLocalized(value.title);
  if (!title) return undefined;
  if (typeof value.eventType !== 'string' || !value.eventType) return undefined;
  if (!Array.isArray(value.requiredNodeTypes) || value.requiredNodeTypes.length === 0) return undefined;
  if (!value.requiredNodeTypes.every((entry): entry is string => typeof entry === 'string' && !!entry)) {
    return undefined;
  }
  const workflow = isRecord(value.workflow) ? readNodeList(value.workflow.nodes) : undefined;
  if (!workflow) return undefined;
  const source = isRecord(value.source) && value.source.kind === 'plugin' && value.source.pluginId === value.pluginId
    ? (value.source as ActionSource)
    : { kind: 'plugin', pluginId: value.pluginId } as ActionSource;
  const descriptor: PluginTemplateDescriptor = {
    id: value.id,
    pluginId: value.pluginId,
    title,
    eventType: value.eventType,
    requiredNodeTypes: [...value.requiredNodeTypes] as string[],
    workflow: { nodes: workflow },
    source,
  };
  const description = readLocalized(value.description);
  if (description) descriptor.description = description;
  if (typeof value.icon === 'string' && value.icon) descriptor.icon = value.icon;
  if (typeof value.category === 'string' && value.category) descriptor.category = value.category;
  if (isRecord(value.params)) descriptor.params = value.params;
  if (isRecord(value.uiHints)) descriptor.uiHints = value.uiHints;
  return descriptor;
}

function toPluginPageSection(value: JsonValue): PluginPageSection | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.kind !== 'string') return undefined;
  if (!(PLUGIN_PAGE_SECTION_KINDS as readonly string[]).includes(value.kind)) return undefined;
  const section: PluginPageSection = { kind: value.kind as PluginPageSectionKind };
  const title = readLocalized(value.title);
  if (title) section.title = title;
  switch (section.kind) {
    case 'text': {
      const text = readLocalized(value.text);
      if (!text) return undefined;
      section.text = text;
      return section;
    }
    case 'form': {
      if (value.schema !== undefined) {
        if (!isRecord(value.schema)) return undefined;
        section.schema = value.schema;
      }
      if (value.uiHints !== undefined) {
        if (!isRecord(value.uiHints)) return undefined;
        section.uiHints = value.uiHints;
      }
      return section;
    }
    case 'connection':
      return section;
    case 'list': {
      if (typeof value.optionsFrom !== 'string' || !normalizeOptionsFrom(value.optionsFrom)) {
        return undefined;
      }
      section.optionsFrom = value.optionsFrom;
      return section;
    }
    case 'tts': {
      if (typeof value.actionType !== 'string' || !value.actionType.trim() || value.actionType.length > 128) {
        return undefined;
      }
      if (typeof value.voicesFrom !== 'string' || !normalizeOptionsFrom(value.voicesFrom)) {
        return undefined;
      }
      section.actionType = value.actionType;
      section.voicesFrom = value.voicesFrom;
      // Server-side audio outputs are optional: a malformed marker hides the
      // selector instead of dropping the whole TTS panel. (The host already
      // rejects malformed markers at discovery; this is defense in depth.)
      if (value.outputsFrom !== undefined) {
        const outputs = typeof value.outputsFrom === 'string' ? normalizeOptionsFrom(value.outputsFrom) : undefined;
        if (outputs) section.outputsFrom = outputs;
      }
      return section;
    }
  }
}

/** Converts a host-stamped page entry to a typed descriptor (see template note). */
export function toPluginPageDescriptor(value: JsonValue): PluginPageDescriptor | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.id !== 'string' || !value.id) return undefined;
  if (typeof value.pluginId !== 'string' || !value.pluginId) return undefined;
  const title = readLocalized(value.title);
  if (!title) return undefined;
  if (!Array.isArray(value.sections) || value.sections.length === 0 || value.sections.length > 16) {
    return undefined;
  }
  const sections: PluginPageSection[] = [];
  for (const entry of value.sections) {
    const section = toPluginPageSection(entry);
    if (!section) return undefined;
    sections.push(section);
  }
  const source = isRecord(value.source) && value.source.kind === 'plugin' && value.source.pluginId === value.pluginId
    ? (value.source as ActionSource)
    : { kind: 'plugin', pluginId: value.pluginId } as ActionSource;
  const descriptor: PluginPageDescriptor = {
    id: value.id,
    pluginId: value.pluginId,
    title,
    sections,
    source,
  };
  if (typeof value.icon === 'string' && value.icon) descriptor.icon = value.icon;
  return descriptor;
}

/**
 * Merges host-stamped plugin templates into the selectable list. Invalid
 * entries are dropped; builtin ids win over colliding plugin ids.
 */
export function mergePluginTemplates(
  builtinIds: readonly string[],
  stamped: readonly unknown[] | undefined,
): PluginTemplateDescriptor[] {
  if (!stamped) return [];
  const seen = new Set(builtinIds);
  const merged: PluginTemplateDescriptor[] = [];
  for (const entry of stamped) {
    const descriptor = toPluginTemplateDescriptor(entry as JsonValue);
    if (!descriptor || seen.has(descriptor.id)) continue;
    seen.add(descriptor.id);
    merged.push(descriptor);
  }
  return merged;
}

/** Same merge contract for plugin configuration pages. */
export function mergePluginPages(stamped: readonly unknown[] | undefined): PluginPageDescriptor[] {
  if (!stamped) return [];
  const seen = new Set<string>();
  const merged: PluginPageDescriptor[] = [];
  for (const entry of stamped) {
    const descriptor = toPluginPageDescriptor(entry as JsonValue);
    if (!descriptor) continue;
    const key = pluginNavId(descriptor.pluginId, descriptor.id);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(descriptor);
  }
  return merged;
}

/**
 * Merges host-stamped typed `ui` descriptors. Entries were validated at
 * discovery; the frontend re-validates at merge so corrupt snapshots fail
 * closed instead of rendering partial UI.
 */
export function mergePluginUis(stamped: readonly unknown[] | undefined): PluginUiDescriptor[] {
  if (!stamped) return [];
  const seen = new Set<string>();
  const merged: PluginUiDescriptor[] = [];
  for (const entry of stamped) {
    const descriptor = normalizeUiDescriptor(entry as JsonValue);
    if (!descriptor) continue;
    if (seen.has(descriptor.pluginId)) continue;
    seen.add(descriptor.pluginId);
    merged.push(descriptor);
  }
  return merged;
}

/** Defense-in-depth caps mirroring the host validator (see `validate_plugin_autocomplete`). */
const MAX_AUTOCOMPLETE_LIST = 32;
const MAX_AUTOCOMPLETE_TEXT = 256;

function readAutocompletePaths(value: JsonValue | undefined, max: number): string[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || value.length > max) return undefined;
  const out: string[] = [];
  for (const entry of value) {
    if (typeof entry !== 'string') return undefined;
    const trimmed = entry.trim();
    if (!trimmed || trimmed.length > MAX_AUTOCOMPLETE_TEXT) return undefined;
    out.push(trimmed);
  }
  return out;
}

function readAutocompleteFields(value: JsonValue | undefined): PluginAutocompleteField[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || value.length > MAX_AUTOCOMPLETE_LIST) return undefined;
  const out: PluginAutocompleteField[] = [];
  for (const entry of value) {
    if (!isRecord(entry)) return undefined;
    if (typeof entry.path !== 'string') return undefined;
    const path = entry.path.trim();
    if (!path || path.length > MAX_AUTOCOMPLETE_TEXT || path.endsWith('.')) return undefined;
    if (entry.kind !== 'string' && entry.kind !== 'number' && entry.kind !== 'boolean') return undefined;
    const label = readLocalized(entry.label);
    if (!label) return undefined;
    const field: PluginAutocompleteField = { path, kind: entry.kind as PluginAutocompleteField['kind'], label };
    if (entry.hint !== undefined) {
      const hint = readLocalized(entry.hint);
      if (!hint) return undefined;
      field.hint = hint;
    }
    out.push(field);
  }
  return out;
}

/**
 * Converts a host-stamped autocomplete entry to a typed descriptor. The host
 * already validated the shape and namespace; this guards the registry
 * against malformed snapshots by rejecting them instead of gating on
 * garbage paths.
 */
export function toPluginAutocompleteContribution(value: JsonValue): PluginAutocompleteContribution | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.id !== 'string' || !value.id.trim() || value.id.length > MAX_AUTOCOMPLETE_TEXT) {
    return undefined;
  }
  if (typeof value.pluginId !== 'string' || !value.pluginId.trim() || value.pluginId.length > 128) {
    return undefined;
  }
  const prefixes = readAutocompletePaths(value.prefixes, 16);
  const paths = readAutocompletePaths(value.paths, MAX_AUTOCOMPLETE_LIST);
  const fields = readAutocompleteFields(value.fields);
  const triggers = readAutocompletePaths(value.triggers, MAX_AUTOCOMPLETE_LIST);
  if (prefixes === undefined && value.prefixes !== undefined) return undefined;
  if (paths === undefined && value.paths !== undefined) return undefined;
  if (fields === undefined && value.fields !== undefined) return undefined;
  if (triggers === undefined && value.triggers !== undefined) return undefined;
  if ((prefixes?.length ?? 0) + (paths?.length ?? 0) + (fields?.length ?? 0) === 0) return undefined;
  const source = isRecord(value.source) && value.source.kind === 'plugin' && value.source.pluginId === value.pluginId
    ? (value.source as ActionSource)
    : { kind: 'plugin', pluginId: value.pluginId } as ActionSource;
  const descriptor: PluginAutocompleteContribution = {
    id: value.id,
    pluginId: value.pluginId,
    source,
  };
  if (prefixes) descriptor.prefixes = prefixes;
  if (paths) descriptor.paths = paths;
  if (fields) descriptor.fields = fields;
  if (triggers) descriptor.triggers = triggers;
  return descriptor;
}

/**
 * Merges host-stamped autocomplete contributions. Invalid entries are
 * dropped; later duplicates of a namespaced id lose to the first.
 */
export function mergePluginAutocomplete(
  stamped: readonly unknown[] | undefined,
): PluginAutocompleteContribution[] {
  if (!stamped) return [];
  const seen = new Set<string>();
  const merged: PluginAutocompleteContribution[] = [];
  for (const entry of stamped) {
    const descriptor = toPluginAutocompleteContribution(entry as JsonValue);
    if (!descriptor || seen.has(descriptor.id)) continue;
    seen.add(descriptor.id);
    merged.push(descriptor);
  }
  return merged;
}

/**
 * True when a plugin page is connection-only: it carries at least one
 * `connection` section and no `form`/`list`/`tts` sections (`text` intros
 * don't count). Such pages are owned by the Connections tab, which renders
 * the same connection card inline — the nav rail hides them so every
 * server-style plugin doesn't mint its own duplicate minimal tab.
 */
export function isConnectionOnlyPage(page: { sections: readonly { kind: string }[] }): boolean {
  let hasConnection = false;
  for (const section of page.sections) {
    if (section.kind === 'connection') {
      hasConnection = true;
    } else if (section.kind === 'form' || section.kind === 'list' || section.kind === 'tts') {
      return false;
    }
  }
  return hasConnection;
}

/** Connection probe result held per plugin id by the app controller. */
export type PluginConnectionState = {
  ok: boolean;
  latencyMs: number;
  error?: string;
  at: number;
};

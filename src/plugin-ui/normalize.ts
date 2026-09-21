/**
 * Normalization (validation + shaping) for untrusted plugin UI descriptors.
 *
 * The host already validates manifests at discovery; this guards the
 * renderer against malformed snapshots (old hosts, corrupt caches, future
 * `plugins.ui.describe` payloads) by rejecting them instead of rendering
 * partial UI. Unknown node types fail closed — never rendered, never
 * executed.
 */

import type { Localized } from '../shared/localized.ts';
import type { JsonObject, JsonValue } from '../shared/json.ts';
import { normalizeAction } from './actions.ts';
import { parseBinding } from './bindings.ts';
import {
  MAX_UI_PAGES,
  PLUGIN_UI_MODES,
  PLUGIN_UI_NODE_TYPES,
  PLUGIN_UI_VERSION,
  type PluginUiDescriptor,
  type PluginUiDescriptorPage,
  type PluginUiMode,
  type PluginUiNode,
  type PluginUiPage,
} from './contracts.ts';

/** Maximum nesting depth for container nodes (stack/card). */
export const MAX_UI_DEPTH = 8;
/** Maximum children per container node. */
export const MAX_UI_CHILDREN = 32;
/** Maximum static options on a select node. */
export const MAX_UI_OPTIONS = 256;

function isRecord(value: JsonValue | undefined): value is JsonObject {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function readLocalized(value: JsonValue | undefined): Localized | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.default !== 'string' || !value.default.trim()) return undefined;
  if (value.default.length > 1_024) return undefined;
  const i18key = typeof value.i18key === 'string' ? value.i18key : '';
  if (i18key.length > 256) return undefined;
  return { default: value.default, i18key };
}

function readOptionalLocalized(value: JsonValue | undefined): Localized | undefined {
  if (value === undefined) return undefined;
  return readLocalized(value);
}

function readNodeKey(value: JsonValue | undefined): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== 'string' || !value.trim() || value.length > 64) return undefined;
  if (!/^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(value)) return undefined;
  return value;
}

function readOptionsFrom(value: JsonValue | undefined): string | undefined {
  if (typeof value !== 'string') return undefined;
  const source = value.trim();
  if (!source || source.length > 256) return undefined;
  return source;
}

function normalizeChildren(value: JsonValue | undefined, depth: number): PluginUiNode[] | undefined {
  if (!Array.isArray(value) || value.length === 0 || value.length > MAX_UI_CHILDREN) {
    return undefined;
  }
  const children: PluginUiNode[] = [];
  for (const entry of value) {
    const node = normalizeNode(entry, depth);
    if (!node) return undefined;
    children.push(node);
  }
  return children;
}

function readNumber(value: JsonValue | undefined): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== 'number' || !Number.isFinite(value)) return undefined;
  return value;
}

/** Normalizes one untrusted UI node (recursive, depth-limited). */
export function normalizeNode(value: JsonValue, depth = 0): PluginUiNode | undefined {
  if (!isRecord(value) || typeof value.type !== 'string') return undefined;
  if (depth > MAX_UI_DEPTH) return undefined;
  if (!(PLUGIN_UI_NODE_TYPES as readonly string[]).includes(value.type)) return undefined;
  const key = readNodeKey(value.key);
  if (value.key !== undefined && key === undefined) return undefined;
  const title = readOptionalLocalized(value.title);
  if (value.title !== undefined && title === undefined) return undefined;
  const label = readOptionalLocalized(value.label);
  if (value.label !== undefined && label === undefined) return undefined;

  switch (value.type) {
    case 'stack':
    case 'card': {
      const children = normalizeChildren(value.children, depth + 1);
      if (!children) return undefined;
      const node: PluginUiNode =
        value.type === 'stack' ? { type: 'stack', children } : { type: 'card', children };
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'text':
    case 'status': {
      const text = readLocalized(value.text);
      if (!text) return undefined;
      if (value.type === 'text') {
        const node: PluginUiNode = { type: 'text', text };
        if (key) node.key = key;
        if (title) node.title = title;
        if (label) node.label = label;
        return node;
      }
      const tone = value.tone;
      if (tone !== undefined && tone !== 'info' && tone !== 'ok' && tone !== 'error') {
        return undefined;
      }
      const node: PluginUiNode = { type: 'status', text };
      if (tone === 'info' || tone === 'ok' || tone === 'error') node.tone = tone;
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'form': {
      if (value.schema !== undefined && !isRecord(value.schema)) return undefined;
      if (value.uiHints !== undefined && !isRecord(value.uiHints)) return undefined;
      const node: PluginUiNode = { type: 'form' };
      if (isRecord(value.schema)) node.schema = value.schema;
      if (isRecord(value.uiHints)) node.uiHints = value.uiHints;
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'select': {
      if (typeof value.bind !== 'string' || !parseBinding(value.bind)) return undefined;
      const node: PluginUiNode = { type: 'select', bind: value.bind.trim() };
      if (value.options !== undefined) {
        if (!Array.isArray(value.options) || value.options.length > MAX_UI_OPTIONS) {
          return undefined;
        }
        const options: Array<{ value: string; label: Localized }> = [];
        for (const entry of value.options) {
          if (!isRecord(entry) || typeof entry.value !== 'string' || entry.value.length > 256) {
            return undefined;
          }
          const optionLabel = readLocalized(entry.label);
          if (!optionLabel) return undefined;
          options.push({ value: entry.value, label: optionLabel });
        }
        node.options = options;
      }
      if (value.optionsFrom !== undefined) {
        const optionsFrom = readOptionsFrom(value.optionsFrom);
        if (!optionsFrom) return undefined;
        node.optionsFrom = optionsFrom;
      }
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'range': {
      if (typeof value.bind !== 'string' || !parseBinding(value.bind)) return undefined;
      const min = readNumber(value.min);
      const max = readNumber(value.max);
      const step = readNumber(value.step);
      if (value.min !== undefined && min === undefined) return undefined;
      if (value.max !== undefined && max === undefined) return undefined;
      if (value.step !== undefined && (step === undefined || step <= 0)) return undefined;
      if (min !== undefined && max !== undefined && min > max) return undefined;
      const node: PluginUiNode = { type: 'range', bind: value.bind.trim() };
      if (min !== undefined) node.min = min;
      if (max !== undefined) node.max = max;
      if (step !== undefined) node.step = step;
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'checkbox': {
      if (typeof value.bind !== 'string' || !parseBinding(value.bind)) return undefined;
      const node: PluginUiNode = { type: 'checkbox', bind: value.bind.trim() };
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'button': {
      const action = normalizeAction(value.action);
      if (!action) return undefined;
      const variant = value.variant;
      if (variant !== undefined && variant !== 'primary' && variant !== 'danger') return undefined;
      const node: PluginUiNode = { type: 'button', action };
      if (variant === 'primary' || variant === 'danger') node.variant = variant;
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'list': {
      const optionsFrom = readOptionsFrom(value.optionsFrom);
      if (!optionsFrom) return undefined;
      const node: PluginUiNode = { type: 'list', optionsFrom };
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'separator': {
      const node: PluginUiNode = { type: 'separator' };
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    case 'connection': {
      const node: PluginUiNode = { type: 'connection' };
      if (key) node.key = key;
      if (title) node.title = title;
      if (label) node.label = label;
      return node;
    }
    default:
      return undefined;
  }
}

/** Normalizes one untrusted UI page descriptor. */
export function normalizePage(value: JsonValue): PluginUiPage | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.id !== 'string' || !value.id || value.id.length > 128) return undefined;
  if (typeof value.pluginId !== 'string' || !value.pluginId || value.pluginId.length > 128) {
    return undefined;
  }
  const title = readLocalized(value.title);
  if (!title) return undefined;
  if (value.icon !== undefined) {
    if (typeof value.icon !== 'string' || !value.icon.trim() || value.icon.length > 64) {
      return undefined;
    }
  }
  if (value.body === undefined) return undefined;
  const body = normalizeNode(value.body, 0);
  if (!body) return undefined;
  const page: PluginUiPage = { id: value.id, pluginId: value.pluginId, title, body };
  if (typeof value.icon === 'string') page.icon = value.icon;
  return page;
}

const PLUGIN_ID_PATTERN = /^[a-z][a-z0-9._-]{1,127}$/;

/** Mirrors `is_valid_plugin_id` (Rust is canonical). */
function isValidPageId(value: string): boolean {
  return PLUGIN_ID_PATTERN.test(value);
}

/**
 * Mirrors `is_valid_ui_entry` (Rust is canonical): webview entries stay
 * inside the plugin's `ui/` asset directory — relative, no `..`, no
 * absolute paths, HTML only.
 */
function isValidUiEntry(entry: string): boolean {
  if (entry.length === 0 || entry.length > 256) return false;
  if (!entry.startsWith('ui/') || !entry.endsWith('.html')) return false;
  if (entry.includes('\\') || entry.includes('\0')) return false;
  if (entry.startsWith('/') || entry.includes(':/')) return false;
  const parts = entry.split('/');
  return parts.length > 0 && parts.every((part) => part.length > 0 && part !== '..');
}

/**
 * Normalizes one stamped `ui` descriptor (re-validation at render time).
 * Unknown modes, unsafe entries, and mode/body mismatches fail closed.
 */
export function normalizeUiDescriptor(value: JsonValue): PluginUiDescriptor | undefined {
  if (!isRecord(value)) return undefined;
  if (typeof value.pluginId !== 'string' || !value.pluginId || value.pluginId.length > 128) {
    return undefined;
  }
  if (value.apiVersion !== PLUGIN_UI_VERSION) return undefined;
  const mode: PluginUiMode =
    value.mode === undefined ? 'declarative' : (value.mode as PluginUiMode);
  if (!(PLUGIN_UI_MODES as readonly string[]).includes(mode)) return undefined;
  if (!Array.isArray(value.pages) || value.pages.length === 0 || value.pages.length > MAX_UI_PAGES) {
    return undefined;
  }
  const entry = value.entry;
  if (mode === 'declarative' && entry !== undefined) return undefined;
  if (mode === 'webview' && (typeof entry !== 'string' || !isValidUiEntry(entry))) {
    return undefined;
  }
  const pages: PluginUiDescriptorPage[] = [];
  for (const raw of value.pages) {
    if (!isRecord(raw)) return undefined;
    if (typeof raw.id !== 'string' || !isValidPageId(raw.id)) return undefined;
    const title = readLocalized(raw.title);
    if (!title) return undefined;
    if (raw.icon !== undefined) {
      if (typeof raw.icon !== 'string' || !raw.icon.trim() || raw.icon.length > 64) {
        return undefined;
      }
    }
    const page: PluginUiDescriptorPage = { id: raw.id, title };
    if (typeof raw.icon === 'string') page.icon = raw.icon;
    if (mode === 'declarative') {
      if (raw.body === undefined) return undefined;
      const body = normalizeNode(raw.body, 0);
      if (!body) return undefined;
      page.body = body;
    } else if (raw.body !== undefined) {
      return undefined;
    }
    pages.push(page);
  }
  const descriptor: PluginUiDescriptor = {
    pluginId: value.pluginId,
    apiVersion: PLUGIN_UI_VERSION,
    mode,
    pages,
  };
  if (typeof entry === 'string') descriptor.entry = entry;
  return descriptor;
}

/**
 * Collects every option-source id a page body references (list/select
 * nodes, refresh-source actions). The host fetches each through the
 * normal option pipeline; unknown markers are skipped, never fetched.
 */
export function collectOptionSources(body: PluginUiNode): string[] {
  const sources: string[] = [];
  const push = (source: string | undefined): void => {
    if (source && !sources.includes(source)) sources.push(source);
  };
  const walk = (node: PluginUiNode): void => {
    switch (node.type) {
      case 'stack':
      case 'card':
        for (const child of node.children) walk(child);
        break;
      case 'list':
        push(node.optionsFrom);
        break;
      case 'select':
        push(node.optionsFrom);
        break;
      case 'button':
        if (node.action.type === 'refresh-source') push(node.action.source);
        break;
      default:
        break;
    }
  };
  walk(body);
  return sources;
}

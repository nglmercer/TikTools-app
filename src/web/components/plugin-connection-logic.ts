import type { JsonObject, JsonValue } from '../../automation/types.ts';
import { isSecretField, SECRET_PLACEHOLDER } from '../../automation/plugins/declarative.ts';
import type { Locale } from '../i18n.ts';
import { localized } from './ui/schema-form-helpers.ts';

/** Idle window between the last edit and an autosave round-trip. */
export const AUTOSAVE_DEBOUNCE_MS = 800;
/** Host echo wait before an autosave is reported as failed. */
export const AUTOSAVE_CONFIRM_TIMEOUT_MS = 5000;
/** Summary rows cap so long schemas stay a compact card. */
export const SUMMARY_ROW_LIMIT = 4;
const MAX_URL_LEN = 2048;

/** True for absolute `http(s)` URLs with a hostname. */
export function isHttpUrl(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > MAX_URL_LEN) return false;
  try {
    const url = new URL(trimmed);
    return (url.protocol === 'http:' || url.protocol === 'https:') && url.hostname.length > 0;
  } catch {
    return false;
  }
}

/**
 * True when the URL points at this machine. This only picks UI copy (the
 * trusted-mode note); the host enforces the real loopback trust boundary.
 */
export function isLoopbackUrl(value: string): boolean {
  let host: string;
  try {
    host = new URL(value.trim()).hostname.toLowerCase();
  } catch {
    return false;
  }
  if (host === 'localhost' || host === '::1' || host === '[::1]') return true;
  return /^127\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(host);
}

function schemaProperties(schema: JsonObject | undefined): Array<[string, JsonObject]> {
  const properties = schema?.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return [];
  const entries: Array<[string, JsonObject]> = [];
  for (const [key, field] of Object.entries(properties as JsonObject)) {
    if (field && typeof field === 'object' && !Array.isArray(field)) {
      entries.push([key, field as JsonObject]);
    }
  }
  return entries;
}

function fieldHint(uiHints: JsonObject | undefined, key: string): JsonObject | undefined {
  const fields = uiHints?.fields;
  if (!fields || typeof fields !== 'object' || Array.isArray(fields)) return undefined;
  const hint = (fields as JsonObject)[key];
  return hint && typeof hint === 'object' && !Array.isArray(hint) ? (hint as JsonObject) : undefined;
}

/**
 * First string field whose schema declares `format: "uri"`: the server URL
 * the connection card validates inline. No name sniffing: manifests opt in
 * explicitly, and pages without one simply skip URL validation.
 */
export function findServerUrlKey(schema: JsonObject | undefined): string | undefined {
  for (const [key, field] of schemaProperties(schema)) {
    if (field.type === 'string' && field.format === 'uri') return key;
  }
  return undefined;
}

/** Display text for a flat setting value; empty when there is nothing to show. */
export function settingDisplayText(value: JsonValue | undefined): string {
  if (typeof value === 'string') return value.trim();
  if (typeof value === 'number' && Number.isFinite(value)) return String(value);
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  return '';
}

export type ConnectionSummaryRow = {
  key: string;
  label: string;
  value: string;
};

/**
 * Compact summary rows for a connected card: schema order, URL and secrets
 * excluded, empties skipped, capped so long schemas stay one glance.
 */
export function connectionSummaryRows(
  values: JsonObject,
  schema: JsonObject | undefined,
  uiHints: JsonObject | undefined,
  urlKey: string | undefined,
  locale: Locale,
): ConnectionSummaryRow[] {
  const rows: ConnectionSummaryRow[] = [];
  for (const [key, field] of schemaProperties(schema)) {
    if (key === urlKey) continue;
    if (isSecretField(field, fieldHint(uiHints, key))) continue;
    const text = settingDisplayText(values[key]);
    if (!text) continue;
    rows.push({ key, label: localized(field.title, locale) || key, value: text });
    if (rows.length >= SUMMARY_ROW_LIMIT) break;
  }
  return rows;
}

/**
 * Fills missing non-secret scalar defaults for display. The host applies the
 * same overlay, so this is belt-and-braces for old hosts and corrupt caches:
 * secrets are never defaulted, matching `apply_settings_defaults`.
 */
export function withSchemaDefaults(values: JsonObject, schema: JsonObject | undefined): JsonObject {
  const properties = schemaProperties(schema);
  if (properties.length === 0) return values;
  let filled: JsonObject | undefined;
  for (const [key, field] of properties) {
    if (values[key] !== undefined) continue;
    if (field.secret === true) continue;
    const fallback = field.default;
    if (typeof fallback !== 'string' && typeof fallback !== 'number' && typeof fallback !== 'boolean') {
      continue;
    }
    filled ??= { ...values };
    filled[key] = fallback;
  }
  return filled ?? values;
}

/** Order-insensitive equality for flat settings maps. */
export function settingsEqual(a: JsonObject, b: JsonObject): boolean {
  return stableSettingsJson(a) === stableSettingsJson(b);
}

/**
 * Secret-aware equality for draft-vs-display comparisons. A redacted echo
 * stands in for any stored secret, so a typed (or untouched placeholder)
 * secret matches the echoed placeholder — this is what lets a just-saved
 * token read as clean while the typed value stays in the draft (masked) for
 * Show/Hide to reveal. An explicit clearing (`''` vs placeholder) is a real
 * change until the host confirms it.
 */
export function settingsMatch(
  a: JsonObject,
  b: JsonObject,
  secretKeys: readonly string[] = [],
): boolean {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  for (const key of keys) {
    const left = a[key];
    const right = b[key];
    if (left === right) continue;
    if (
      secretKeys.includes(key)
      && (left === SECRET_PLACEHOLDER || right === SECRET_PLACEHOLDER)
      && left !== ''
      && right !== ''
    ) {
      continue;
    }
    return false;
  }
  return true;
}

/**
 * Settings keys rendered as masked secret fields (schema `secret: true` or a
 * secret UI hint). The host never reveals stored secrets: every WebView
 * payload carries {@link SECRET_PLACEHOLDER} instead, and saving the
 * placeholder back preserves the stored value.
 */
export function secretSettingKeys(
  schema: JsonObject | undefined,
  uiHints: JsonObject | undefined,
): string[] {
  const keys: string[] = [];
  for (const [key, field] of schemaProperties(schema)) {
    if (isSecretField(field, fieldHint(uiHints, key))) keys.push(key);
  }
  return keys;
}

/**
 * True when a host settings echo carries every sent key back unchanged.
 * Extra echo keys are fine: the host overlays schema defaults the payload
 * never carried.
 *
 * Secret keys are exempt from the strict comparison: the host redacts them
 * to the placeholder on every echo, so a placeholder echo confirms any sent
 * secret (a fresh value the host just stored, or the placeholder itself
 * preserving the stored value). Without this, typing a token could never
 * confirm and every secret save would end in an error.
 */
export function echoConfirmsSave(
  echo: JsonObject,
  sent: JsonObject,
  secretKeys: readonly string[] = [],
): boolean {
  return Object.entries(sent).every(([key, value]) => {
    if (echo[key] === value) return true;
    return secretKeys.includes(key) && echo[key] === SECRET_PLACEHOLDER;
  });
}

export function stableSettingsJson(values: JsonObject): string {
  const keys = Object.keys(values).sort();
  return JSON.stringify(keys.map((key) => [key, values[key]]));
}

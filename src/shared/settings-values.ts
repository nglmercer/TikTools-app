/**
 * Canonical plugin settings-value helpers shared by every settings UI.
 *
 * Settings values are full JSON (`Record<string, JsonValue>`): nested
 * objects and arrays round-trip to the host untouched. Nothing is
 * silently discarded — only `undefined` is dropped, because JSON has no
 * undefined. Secret marking applies at top-level keys (see
 * `secret_setting_keys`): a top-level key marked secret round-trips as a
 * placeholder wholesale, including nested content beneath it.
 */

import type { JsonObject } from './json.ts';
import type { PluginSettingValues } from './messages.ts';

/** Converts form state to a settings payload, preserving nested JSON. */
export function toSettingValues(value: JsonObject): PluginSettingValues {
  const clean: PluginSettingValues = {};
  for (const [key, entry] of Object.entries(value)) {
    if (entry !== undefined) clean[key] = entry;
  }
  return clean;
}

/** Reads a dotted settings path (`settings.audio.volume` segments). */
export function readSettingsPath(values: JsonObject | undefined, segments: string[]): unknown {
  let current: unknown = values;
  for (const segment of segments) {
    if (!current || typeof current !== 'object' || Array.isArray(current)) return undefined;
    current = (current as JsonObject)[segment];
  }
  return current;
}

/** Returns a copy of `values` with `value` written at `segments`. */
export function writeSettingsPath(
  values: JsonObject,
  segments: string[],
  value: unknown,
): JsonObject {
  if (segments.length === 0) return values;
  const [head, ...rest] = segments as [string, ...string[]];
  const existing = values[head];
  const child: JsonObject =
    existing && typeof existing === 'object' && !Array.isArray(existing)
      ? (existing as JsonObject)
      : {};
  const next: JsonObject = { ...values };
  next[head] =
    rest.length === 0 ? (value as JsonObject[string]) : writeSettingsPath(child, rest, value);
  return next;
}

/**
 * Binding and option-source parsing for the declarative plugin UI contract.
 *
 * Bindings are intentionally limited to `settings.*`, `local.*`, and
 * `source.*` — no expressions, no `eval`, no `new Function`, no inline
 * plugin scripts. Anything else fails closed (returns undefined) so the
 * renderer skips the node instead of guessing.
 */

import type { PluginUiBinding, PluginUiBindingScope } from './contracts.ts';

const SCOPE_PATTERN = /^(settings|local|source)\.([A-Za-z][A-Za-z0-9._-]{0,127})$/;
const SETTINGS_SEGMENT = /^[A-Za-z][A-Za-z0-9_-]{0,63}$/;

/** Parses a binding reference into scope + path, or undefined when invalid. */
export function parseBinding(raw: string | undefined): PluginUiBinding | undefined {
  if (typeof raw !== 'string') return undefined;
  const trimmed = raw.trim();
  if (!trimmed || trimmed.length > 144) return undefined;
  const match = SCOPE_PATTERN.exec(trimmed);
  if (!match?.[1] || !match[2]) return undefined;
  const scope = match[1] as PluginUiBindingScope;
  const path = match[2];
  // `settings.` paths may be dotted; every segment must be a plain key so a
  // binding can never address prototypes or array internals.
  if (scope === 'settings') {
    const segments = path.split('.');
    if (segments.length === 0 || segments.length > 8) return undefined;
    if (!segments.every((segment) => SETTINGS_SEGMENT.test(segment))) return undefined;
    if (segments.includes('__proto__') || segments.includes('prototype')) return undefined;
  } else if (path.includes('.')) {
    // `local.*` / `source.*` are single flat names.
    return undefined;
  }
  return { raw: trimmed, scope, path };
}

/** Splits a validated settings binding path into key segments. */
export function settingsPathSegments(binding: PluginUiBinding): string[] {
  if (binding.scope !== 'settings') return [];
  return binding.path.split('.');
}

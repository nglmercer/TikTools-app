import { BEHAVIOR_TRIGGERS } from './schema.ts';
import type { PluginEventType, PluginStatus } from './types.ts';

/**
 * Plugin-declared event triggers (hotkey.pressed, timer.tick…).
 *
 * The built-in trigger union stays closed so the picker can never offer a
 * type the host does not publish; plugin types arrive at runtime inside the
 * behavior snapshot and are validated by the host catalog merge. Everything
 * here is pure and UI-agnostic: locale-aware labels live in
 * `web/views/behavior/helpers.vue` (`triggerLabel`).
 */

/** True for host-owned triggers; anything else must come from a plugin. */
export function isBuiltinTrigger(type: string): boolean {
  return (BEHAVIOR_TRIGGERS as readonly string[]).includes(type);
}

export function findEventType(
  eventTypes: PluginEventType[],
  type: string,
): PluginEventType | undefined {
  return eventTypes.find((entry) => entry.type === type);
}

/**
 * Plugin triggers the event picker may offer: only types whose plugin is
 * installed and enabled. Mirrors `availableActionTypes` for actions.
 */
export function availableEventTypes(
  plugins: PluginStatus[],
  eventTypes: PluginEventType[],
): PluginEventType[] {
  const active = new Set<string>();
  for (const plugin of plugins) {
    if (plugin.installed && plugin.enabled) active.add(plugin.descriptor.id);
  }
  return eventTypes.filter(
    (entry) => entry.source.kind === 'plugin' && active.has(entry.source.pluginId),
  );
}

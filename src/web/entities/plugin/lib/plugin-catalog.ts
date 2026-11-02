import type { ActionTypeDefinition, LiveAction, PluginDescriptor, PluginStatus } from '../../../../automation/behavior/types.ts';

export const MAX_PLUGIN_TAGS = 4;

export type PluginCatalogTab = 'installed' | 'browse' | 'updates';

type Localize = (value: unknown) => string;

export function pluginTags(descriptor: PluginDescriptor, actionTypes: readonly ActionTypeDefinition[]): string[] {
  const declared = (descriptor.tags ?? []).filter((tag): tag is string => typeof tag === 'string' && tag.length > 0);
  if (declared.length > 0) return declared.slice(0, MAX_PLUGIN_TAGS);
  const derived: string[] = [];
  for (const id of descriptor.actionTypeIds) {
    const tag = actionTypes.find((action) => action.id === id)?.tag?.trim().toLowerCase();
    if (tag && !derived.includes(tag)) derived.push(tag);
    if (derived.length >= MAX_PLUGIN_TAGS) return derived;
  }
  if (descriptor.eventTypeIds.length > 0 && derived.length < MAX_PLUGIN_TAGS) derived.push('events');
  return derived;
}

export function matchesPluginQuery(
  plugin: PluginStatus,
  actionTypes: readonly ActionTypeDefinition[],
  query: string,
  localize: Localize,
): boolean {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const haystack = [
    plugin.descriptor.id,
    localize(plugin.descriptor.name),
    localize(plugin.descriptor.description),
    ...pluginTags(plugin.descriptor, actionTypes),
  ]
    .join(' ')
    .toLowerCase();
  return words.every((word) => haystack.includes(word));
}

export function catalogPlugins(
  plugins: readonly PluginStatus[],
  tab: PluginCatalogTab,
  actionTypes: readonly ActionTypeDefinition[],
  query: string,
  localize: Localize,
): PluginStatus[] {
  return plugins.filter(
    (plugin) => plugin.installed === (tab === 'installed') && matchesPluginQuery(plugin, actionTypes, query, localize),
  );
}

export function pluginActionUsage(
  pluginId: string,
  actions: readonly LiveAction[],
  actionTypes: readonly ActionTypeDefinition[],
): number {
  return actions.filter((action) => {
    const type = actionTypes.find((entry) => entry.id === action.typeId);
    return type?.source.kind === 'plugin' && type.source.pluginId === pluginId;
  }).length;
}

export type PluginState = 'available' | 'active' | 'disabled' | 'unavailable';

export function pluginState(plugin: PluginStatus): PluginState {
  if (!plugin.available) return 'unavailable';
  if (!plugin.installed) return 'available';
  return plugin.enabled ? 'active' : 'disabled';
}

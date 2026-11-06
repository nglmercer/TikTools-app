export type PluginDetailsTab = 'general' | 'settings' | 'connection' | 'versions';

export const pluginDetailsTabs: readonly PluginDetailsTab[] = ['general', 'settings', 'connection', 'versions'];

export function isPluginDetailsTab(value: unknown): value is PluginDetailsTab {
  return typeof value === 'string' && (pluginDetailsTabs as readonly string[]).includes(value);
}

/** Commands the plugins page can hand to the details panel. */
export type PluginDetailsRequest = 'uninstall' | 'test';

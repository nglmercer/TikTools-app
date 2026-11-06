import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import type { MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import { findTestConnectionAction } from './plugin-test-action.ts';

/** Commands offered by a plugin's context menu; ids are the menu action ids. */
export type PluginCommand =
  | 'open'
  | 'toggle-enabled'
  | 'settings'
  | 'connection'
  | 'versions'
  | 'test'
  | 'install'
  | 'remove'
  | 'uninstall';

export const pluginCommands: readonly PluginCommand[] = [
  'open', 'toggle-enabled', 'settings', 'connection', 'versions', 'test', 'install', 'remove', 'uninstall',
];

export function isPluginCommand(value: string): value is PluginCommand {
  return (pluginCommands as readonly string[]).includes(value);
}

/** Only user packages own files that can be deleted; others are hidden from the installed list. */
export function isRemovablePackage(plugin: PluginStatus): boolean {
  return plugin.descriptor.source === 'user';
}

/** The same availability rules as the plugin details tabs. */
export function pluginCapabilities(plugin: PluginStatus, actionTypes: readonly ActionTypeDefinition[]) {
  const { descriptor } = plugin;
  const hasSettings = plugin.installed && descriptor.hasSettings === true;
  return {
    hasSettings,
    hasConnection: plugin.installed && plugin.enabled && descriptor.hasConnectionProbe === true,
    hasVersions: isRemovablePackage(plugin),
    canTest: hasSettings && plugin.enabled && plugin.available
      && findTestConnectionAction(actionTypes, descriptor.id) !== undefined,
  };
}

export function pluginMenu(
  plugin: PluginStatus,
  actionTypes: readonly ActionTypeDefinition[],
  t: (key: string) => string,
): MenuEntry[] {
  const can = pluginCapabilities(plugin, actionTypes);
  const removable = isRemovablePackage(plugin);
  const entries: MenuEntry[] = [{ id: 'open', label: t('plugins.menu.open') }];

  if (!plugin.installed) {
    entries.push(
      { type: 'separator', id: 'sep-install' },
      {
        id: 'install',
        label: t(removable ? 'plugins.details.activate' : 'plugins.details.install'),
        disabled: !plugin.available,
      },
    );
    return entries;
  }

  entries.push(
    { id: 'toggle-enabled', label: t('plugins.details.enabled'), checked: plugin.enabled },
    { type: 'separator', id: 'sep-tabs' },
    { id: 'settings', label: t('plugins.details.tabs.settings'), disabled: !can.hasSettings },
    { id: 'connection', label: t('plugins.details.tabs.connection'), disabled: !can.hasConnection },
  );
  if (can.hasVersions) entries.push({ id: 'versions', label: t('plugins.details.tabs.versions') });
  entries.push(
    { id: 'test', label: t('plugins.settings.test'), disabled: !can.canTest },
    { type: 'separator', id: 'sep-remove' },
    removable
      ? { id: 'uninstall', label: t('plugins.details.uninstall') }
      : { id: 'remove', label: t('plugins.menu.remove') },
  );
  return entries;
}

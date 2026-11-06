import { describe, expect, it } from 'vitest';
import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import { isMenuAction } from '@/shared/ui/menu/menu-types.ts';
import { pluginMenu } from './plugin-menu.ts';

const t = (key: string) => key;

function plugin(overrides: Partial<PluginStatus> = {}, descriptor: Partial<PluginStatus['descriptor']> = {}): PluginStatus {
  return {
    installed: true, enabled: true, available: true, ...overrides,
    descriptor: {
      id: 'deck', source: 'user', name: { default: 'Deck', i18key: '' }, version: '1.0.0',
      description: { default: '', i18key: '' }, dependency: { default: '', i18key: '' },
      permissions: [], actionTypeIds: [], eventTypeIds: [], ...descriptor,
    },
  } as PluginStatus;
}

const actions = (entries: ReturnType<typeof pluginMenu>) =>
  entries.filter(isMenuAction).map((entry) => `${entry.id}${entry.disabled ? ':off' : ''}${entry.checked === undefined ? '' : `:${entry.checked}`}`);

describe('pluginMenu', () => {
  it('offers uninstall for user packages and remove for other sources', () => {
    expect(actions(pluginMenu(plugin(), [], t))).toEqual([
      'open', 'toggle-enabled:true', 'settings:off', 'connection:off', 'versions', 'test:off', 'uninstall',
    ]);
    expect(actions(pluginMenu(plugin({ available: false, enabled: false }, { source: 'development' }), [], t))).toEqual([
      'open', 'toggle-enabled:false', 'settings:off', 'connection:off', 'test:off', 'remove',
    ]);
  });

  it('enables tabs and the connection test from the plugin capabilities', () => {
    const types = [{ id: 'deck.test_connection', source: { kind: 'plugin', pluginId: 'deck' } }] as unknown as ActionTypeDefinition[];
    const menu = pluginMenu(plugin({}, { hasSettings: true, hasConnectionProbe: true }), types, t);
    expect(actions(menu)).toEqual(['open', 'toggle-enabled:true', 'settings', 'connection', 'versions', 'test', 'uninstall']);
  });

  it('offers install for plugins that are not installed, disabled when unavailable', () => {
    expect(actions(pluginMenu(plugin({ installed: false, enabled: false }, { source: 'builtin' }), [], t))).toEqual(['open', 'install']);
    expect(actions(pluginMenu(plugin({ installed: false, enabled: false, available: false }), [], t))).toEqual(['open', 'install:off']);
  });
});

import { describe, expect, it } from 'vitest';
import type { ActionTypeDefinition, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import { catalogPlugins, matchesPluginQuery, pluginActionUsage, pluginState, pluginTags } from './plugin-catalog.ts';

const text = (value: string) => ({ default: value, i18key: '' });
const localize = (value: unknown) => (value as { default: string }).default;

function plugin(id: string, overrides: Partial<PluginStatus> = {}, tags?: string[]): PluginStatus {
  return {
    installed: true,
    enabled: true,
    available: true,
    ...overrides,
    descriptor: {
      id,
      name: text(`${id} plugin`),
      version: '1.0.0',
      description: text(`Controls ${id}`),
      dependency: text(''),
      permissions: [],
      actionTypeIds: [`${id}.run`, `${id}.stop`],
      eventTypeIds: id === 'deck' ? ['deck.scene'] : [],
      tags,
    },
  };
}

const actionTypes = [
  { id: 'deck.run', tag: 'Scenes', source: { kind: 'plugin', pluginId: 'deck' } },
  { id: 'deck.stop', tag: 'scenes', source: { kind: 'plugin', pluginId: 'deck' } },
  { id: 'say', tag: 'audio', source: { kind: 'builtin' } },
] as unknown as ActionTypeDefinition[];

describe('plugin catalog', () => {
  it('prefers declared tags and derives the rest from actions and events', () => {
    expect(pluginTags(plugin('x', {}, ['a', '', 'b', 'c', 'd', 'e']).descriptor, actionTypes)).toEqual(['a', 'b', 'c', 'd']);
    expect(pluginTags(plugin('deck').descriptor, actionTypes)).toEqual(['scenes', 'events']);
  });

  it('matches every query word against id, name, description and tags', () => {
    const deck = plugin('deck');
    expect(matchesPluginQuery(deck, actionTypes, '', localize)).toBe(true);
    expect(matchesPluginQuery(deck, actionTypes, 'SCENES controls', localize)).toBe(true);
    expect(matchesPluginQuery(deck, actionTypes, 'scenes audio', localize)).toBe(false);
  });

  it('splits plugins by install state before filtering', () => {
    const plugins = [plugin('deck'), plugin('tts', { installed: false })];
    expect(catalogPlugins(plugins, 'installed', actionTypes, '', localize).map((entry) => entry.descriptor.id)).toEqual(['deck']);
    expect(catalogPlugins(plugins, 'browse', actionTypes, 'tts', localize).map((entry) => entry.descriptor.id)).toEqual(['tts']);
    expect(catalogPlugins(plugins, 'browse', actionTypes, 'deck', localize)).toEqual([]);
  });

  it('counts actions backed by the plugin', () => {
    const actions = [{ typeId: 'deck.run' }, { typeId: 'deck.stop' }, { typeId: 'say' }, { typeId: 'missing' }] as LiveAction[];
    expect(pluginActionUsage('deck', actions, actionTypes)).toBe(2);
    expect(pluginActionUsage('tts', actions, actionTypes)).toBe(0);
  });

  it('derives a single display state', () => {
    expect(pluginState(plugin('a', { available: false }))).toBe('unavailable');
    expect(pluginState(plugin('a', { installed: false }))).toBe('available');
    expect(pluginState(plugin('a'))).toBe('active');
    expect(pluginState(plugin('a', { enabled: false }))).toBe('disabled');
  });
});

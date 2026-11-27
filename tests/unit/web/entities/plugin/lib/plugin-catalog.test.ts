import { describe, expect, it } from 'vitest';
import type { PluginStatus } from '../../../../../../src/automation/behavior/types.ts';
import { catalogPlugins, matchesPluginQuery, pluginActionUsage, pluginTags } from '../../../../../../src/web/entities/plugin/lib/plugin-catalog.ts';
import { ACTION_TYPE } from '../../../../../shared/index.ts';
import { actionType, liveAction } from '../../../../support/automation.ts';
import { propertyOf } from '../../../../support/json.ts';

const text = (value: string) => ({ default: value, i18key: '' });
const localize = (value: unknown) => {
  const label = propertyOf(value, 'default');
  return typeof label === 'string' ? label : '';
};

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
      eventTypeIds: id === 'deck' ? [ACTION_TYPE.deckScene] : [],
      tags,
    },
  };
}

const actionTypes = [
  actionType({ id: 'deck.run', tag: 'Scenes', source: { kind: 'plugin', pluginId: 'deck' } }),
  actionType({ id: 'deck.stop', tag: 'scenes', source: { kind: 'plugin', pluginId: 'deck' } }),
  actionType({ id: 'say', tag: 'audio', source: { kind: 'builtin' } }),
];

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
    const actions = [
      liveAction({ id: 'a', typeId: 'deck.run' }),
      liveAction({ id: 'b', typeId: 'deck.stop' }),
      liveAction({ id: 'c', typeId: 'say' }),
      liveAction({ id: 'd', typeId: 'missing' }),
    ];
    expect(pluginActionUsage('deck', actions, actionTypes)).toBe(2);
    expect(pluginActionUsage('tts', actions, actionTypes)).toBe(0);
  });
});

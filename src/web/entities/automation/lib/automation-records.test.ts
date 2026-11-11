import { describe, expect, it } from 'vitest';
import type { ActionTypeDefinition, BehaviorRun, LiveAction, LiveEvent, PluginStatus } from '../../../../automation/behavior/types.ts';
import {
  actionNames,
  availableActionTypeIds,
  createActionFromType,
  createEvent,
  describeAction,
  isBuiltinTrigger,
  lastRunByAction,
  matchesQuery,
  moveId,
  sortRecords,
  toggleId,
  triggerLabel,
  triggerMessageKey,
  triggerOptions,
} from './automation-records.ts';

const localize = (value: unknown) => (value as { default: string }).default;
const text = (value: string) => ({ default: value, i18key: '' });

const types = [
  { id: 'core.log', title: text('Log'), source: { kind: 'builtin' } },
  { id: 'deck.scene', title: text('Scene'), source: { kind: 'plugin', pluginId: 'deck' } },
  { id: 'tts.say', title: text('Say'), source: { kind: 'plugin', pluginId: 'tts' } },
] as unknown as ActionTypeDefinition[];

const plugins = [
  { installed: true, enabled: true, descriptor: { id: 'deck' } },
  { installed: true, enabled: false, descriptor: { id: 'tts' } },
] as unknown as PluginStatus[];

function action(id: string, name: string, enabled = true, typeId = 'core.log', config = {}): LiveAction {
  return { schemaVersion: 2, id, name, enabled, typeId, config };
}

describe('automation records', () => {
  it('sorts by name or enabled state', () => {
    const rows = [action('a', 'beta', false), action('b', 'alpha'), action('c', 'gamma')];
    expect(sortRecords(rows, 'name').map((row) => row.name)).toEqual(['alpha', 'beta', 'gamma']);
    expect(sortRecords(rows, 'name-desc').map((row) => row.name)).toEqual(['gamma', 'beta', 'alpha']);
    expect(sortRecords(rows, 'enabled').map((row) => row.name)).toEqual(['alpha', 'gamma', 'beta']);
    expect(sortRecords(rows, 'disabled').map((row) => row.name)).toEqual(['beta', 'alpha', 'gamma']);
  });

  it('matches every query word', () => {
    expect(matchesQuery('', 'anything')).toBe(true);
    expect(matchesQuery('gift WEB', 'Gift to webhook')).toBe(true);
    expect(matchesQuery('gift chat', 'Gift to webhook')).toBe(false);
  });

  it('offers built-in types and those of enabled plugins', () => {
    expect([...availableActionTypeIds(plugins, types)]).toEqual(['core.log', 'deck.scene']);
  });

  it('creates new records with defaults', () => {
    const created = createActionFromType(types[0]!, localize);
    expect(created).toMatchObject({ schemaVersion: 2, name: 'Log', typeId: 'core.log', enabled: true });
    expect(created.id).toBeTruthy();
    expect(createEvent('New event')).toMatchObject({ name: 'New event', enabled: false, trigger: 'tiktok.gift', actionIds: [] });
  });

  it('summarizes action configuration', () => {
    expect(describeAction(action('a', 'x', true, 'core.fetch', { method: 'POST', url: 'https://hooks.example.com/a/b' }))).toBe(
      'POST hooks.example.com',
    );
    expect(describeAction(action('a', 'x', true, 'core.delay', { ms: 500 }))).toBe('500 ms');
    expect(describeAction(action('a', 'x', true, 'deck.scene'))).toBe('deck.scene');
  });

  it('keeps the newest live run per action', () => {
    const runs = [
      { id: '1', actionId: 'a', test: true },
      { id: '2', actionId: 'a', test: false },
      { id: '3', actionId: 'a', test: false },
    ] as BehaviorRun[];
    expect(lastRunByAction(runs).get('a')?.id).toBe('2');
  });

  it('labels triggers from built-ins and plugin declarations', () => {
    const declared = [{ type: 'hotkey.pressed', title: text('Hotkey'), source: { kind: 'plugin', pluginId: 'hotkeys' } }];
    const builtin = (trigger: string) => `builtin:${trigger}`;
    expect(triggerMessageKey('tiktok.room_stats')).toBe('automations.triggers.tiktok-room_stats');
    expect(triggerLabel('tiktok.gift', declared as never, builtin, localize)).toBe('builtin:tiktok.gift');
    expect(triggerLabel('hotkey.pressed', declared as never, builtin, localize)).toBe('Hotkey');
    expect(triggerLabel('custom.thing', [], builtin, localize)).toBe('custom.thing');
    expect(triggerOptions(declared as never, builtin, localize).at(-1)).toEqual({ value: 'hotkey.pressed', label: 'Hotkey', plugin: 'hotkeys' });
  });

  it('recognizes host-owned triggers only', () => {
    expect(isBuiltinTrigger('tiktok.gift')).toBe(true);
    expect(isBuiltinTrigger('plugin.emit')).toBe(true);
    expect(isBuiltinTrigger('hotkey.pressed')).toBe(false);
    expect(isBuiltinTrigger('')).toBe(false);
  });

  it('resolves, toggles and reorders linked actions', () => {
    const event = { actionIds: ['b', 'missing', 'a'] } as LiveEvent;
    expect(actionNames(event, [action('a', 'Alpha'), action('b', 'Beta')])).toEqual(['Beta', 'Alpha']);
    expect(toggleId(['a'], 'b')).toEqual(['a', 'b']);
    expect(toggleId(['a', 'b'], 'a')).toEqual(['b']);
    expect(moveId(['a', 'b', 'c'], 'c', -1)).toEqual(['a', 'c', 'b']);
    expect(moveId(['a', 'b'], 'a', -1)).toEqual(['a', 'b']);
  });
});

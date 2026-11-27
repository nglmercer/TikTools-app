import { testMessage } from '@/shared/i18n/test-message.ts';
import { describe, expect, it } from 'vitest';
import type { LiveAction } from '../../../../../../src/automation/behavior/types.ts';
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
} from '../../../../../../src/web/entities/automation/lib/automation-records.ts';
import { ACTION_TYPE, EVENT_KIND } from '../../../../../shared/index.ts';
import { propertyOf } from '../../../../support/json.ts';
import { actionType, behaviorRun, liveEvent, pluginEventType, pluginStatus, text } from '../../../../support/automation.ts';

const t = (key: string): string => testMessage('en', key);

const localize = (value: unknown): string => {
  const label = propertyOf(value, 'default');
  return typeof label === 'string' ? label : '';
};

const types = [
  actionType({ id: ACTION_TYPE.coreLog, title: text('Log'), source: { kind: 'builtin' } }),
  actionType({ id: ACTION_TYPE.deckScene, title: text('Scene'), source: { kind: 'plugin', pluginId: 'deck' } }),
  actionType({ id: 'tts.say', title: text('Say'), source: { kind: 'plugin', pluginId: 'tts' } }),
];

const plugins = [pluginStatus('deck'), pluginStatus('tts', { enabled: false })];

function action(id: string, name: string, enabled = true, typeId: string = ACTION_TYPE.coreLog, config = {}): LiveAction {
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
    expect(matchesQuery('gift WEB', t('automations.builtinTemplates.giftWebhookTitle'))).toBe(true);
    expect(matchesQuery('gift chat', t('automations.builtinTemplates.giftWebhookTitle'))).toBe(false);
  });

  it('offers built-in types and those of enabled plugins', () => {
    expect([...availableActionTypeIds(plugins, types)]).toEqual([ACTION_TYPE.coreLog, ACTION_TYPE.deckScene]);
  });

  it('creates new records with defaults', () => {
    const created = createActionFromType(types[0]!, localize);
    expect(created).toMatchObject({ schemaVersion: 2, name: 'Log', typeId: ACTION_TYPE.coreLog, enabled: true });
    expect(created.id).toBeTruthy();
    expect(createEvent('New event')).toMatchObject({ name: 'New event', enabled: false, trigger: EVENT_KIND.tiktokGift, actionIds: [] });
  });

  it('summarizes action configuration', () => {
    expect(describeAction(action('a', 'x', true, ACTION_TYPE.coreFetch, { method: 'POST', url: 'https://hooks.example.com/a/b' }))).toBe(
      'POST hooks.example.com',
    );
    expect(describeAction(action('a', 'x', true, 'core.delay', { ms: 500 }))).toBe('500 ms');
    expect(describeAction(action('a', 'x', true, ACTION_TYPE.deckScene))).toBe(ACTION_TYPE.deckScene);
  });

  it('keeps the newest live run per action', () => {
    const runs = [
      behaviorRun({ id: '1', actionId: 'a', test: true }),
      behaviorRun({ id: '2', actionId: 'a', test: false }),
      behaviorRun({ id: '3', actionId: 'a', test: false }),
    ];
    expect(lastRunByAction(runs).get('a')?.id).toBe('2');
  });

  it('labels triggers from built-ins and plugin declarations', () => {
    const declared = [pluginEventType({ type: EVENT_KIND.hotkeyPressed, title: text('Hotkey'), source: { kind: 'plugin', pluginId: 'hotkeys' } })];
    const builtin = (trigger: string) => `builtin:${trigger}`;
    expect(triggerMessageKey('tiktok.room_stats')).toBe('automations.triggers.tiktok-room_stats');
    expect(triggerLabel(EVENT_KIND.tiktokGift, declared, builtin, localize)).toBe('builtin:tiktok.gift');
    expect(triggerLabel(EVENT_KIND.hotkeyPressed, declared, builtin, localize)).toBe('Hotkey');
    expect(triggerLabel('custom.thing', [], builtin, localize)).toBe('custom.thing');
    expect(triggerOptions(declared, builtin, localize).at(-1)).toEqual({ value: EVENT_KIND.hotkeyPressed, label: 'Hotkey', plugin: 'hotkeys' });
  });

  it('recognizes host-owned triggers only', () => {
    expect(isBuiltinTrigger(EVENT_KIND.tiktokGift)).toBe(true);
    expect(isBuiltinTrigger('plugin.emit')).toBe(true);
    expect(isBuiltinTrigger(EVENT_KIND.hotkeyPressed)).toBe(false);
    expect(isBuiltinTrigger('')).toBe(false);
  });

  it('resolves, toggles and reorders linked actions', () => {
    const event = liveEvent({ actionIds: ['b', 'missing', 'a'] });
    expect(actionNames(event, [action('a', 'Alpha'), action('b', 'Beta')])).toEqual(['Beta', 'Alpha']);
    expect(toggleId(['a'], 'b')).toEqual(['a', 'b']);
    expect(toggleId(['a', 'b'], 'a')).toEqual(['b']);
    expect(moveId(['a', 'b', 'c'], 'c', -1)).toEqual(['a', 'c', 'b']);
    expect(moveId(['a', 'b'], 'a', -1)).toEqual(['a', 'b']);
  });
});

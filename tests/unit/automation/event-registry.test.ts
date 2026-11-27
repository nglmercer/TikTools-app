import { describe, expect, test } from 'vitest';

import {
  EVENT_REGISTRY_VERSION,
  allRegistryFields,
  blankEventForType,
  fieldsForEventType,
  pluginEventTypes,
  registryEntryFor,
  registryEventTypes,
  registryHasPath,
  sampleEventForType,
  setPluginEventTypes,
} from '../../../src/automation/event-registry.ts';
import { fieldsForTrigger } from '../../../src/automation/behavior/fields.ts';
import { matchesFilter } from '../../../src/automation/behavior/testing/filter-matcher.ts';
import type { AutomationEventType, JsonValue } from '../../../src/automation/types.ts';
import { BUILTIN_EVENT_TYPES } from '../../../src/automation/contracts/events.ts';
import { normalizeEvent } from '../../../src/automation/behavior/schema.ts';
import type { PluginEventType } from '../../../src/automation/behavior/types.ts';
import { EVENT_KIND } from '../../shared/index.ts';
import { dataOf } from '../support/automation-event.ts';

const ALL_TYPES: AutomationEventType[] = [...BUILTIN_EVENT_TYPES];

function readPath(root: JsonValue, path: string): JsonValue | undefined {
  const parts = path.split('.').filter(Boolean);
  if (parts[0] === 'event') parts.shift();
  let current: JsonValue | undefined = root;
  for (const part of parts) {
    if (current === null || current === undefined || typeof current !== 'object') return undefined;
    if (Array.isArray(current)) {
      const index = Number(part);
      if (!Number.isInteger(index)) return undefined;
      current = current[index];
    } else {
      current = current[part];
    }
  }
  return current;
}

describe('event registry', () => {
  test('covers every automation event type', () => {
    expect(registryEventTypes()).toEqual(ALL_TYPES);
  });

  test('every registry path resolves against its own sample event (no drift)', () => {
    for (const type of ALL_TYPES) {
      const sample = sampleEventForType(type);
      for (const field of fieldsForEventType(type)) {
        expect(readPath(sample, field.path), `${type} ${field.path}`).not.toBeUndefined();
      }
    }
  });

  test('every condition-editor field resolves against the trigger sample', () => {
    for (const trigger of ALL_TYPES) {
      const sample = sampleEventForType(trigger);
      for (const field of fieldsForTrigger(trigger)) {
        expect(registryHasPath(trigger, field.path), `${trigger} ${field.path}`).toBe(true);
        expect(readPath(sample, field.path), `${trigger} ${field.path}`).not.toBeUndefined();
      }
    }
  });

  test('gift sample keeps the values condition tests rely on', () => {
    const sample = sampleEventForType(EVENT_KIND.tiktokGift);
    expect(matchesFilter({ path: 'event.data.giftName', operator: 'eq', value: 'Rose' }, sample)).toBe(true);
    expect(matchesFilter({ path: 'event.data.diamondCount', operator: 'gte', value: '1' }, sample)).toBe(true);
  });

  test('proto-derived events document their vendor message', () => {
    for (const type of [EVENT_KIND.tiktokChat, EVENT_KIND.tiktokGift, EVENT_KIND.tiktokLike, 'tiktok.join', 'tiktok.social', 'tiktok.room_stats'] as const) {
      const fields = fieldsForEventType(type);
      expect(fields.length).toBeGreaterThan(5);
      expect(fields.some((field) => field.sourceField)).toBe(true);
    }
  });

  test('data payloads are keyed by field name, not hardcoded lists', () => {
    expect(Object.keys(dataOf(sampleEventForType(EVENT_KIND.tiktokGift)))).toContain('giftName');
    expect(Object.keys(dataOf(sampleEventForType(EVENT_KIND.tiktokChat)))).toContain('comment');
    expect(allRegistryFields().some((field) => field.path === 'event.user.uniqueId')).toBe(true);
  });

  test('chat exposes stable processor fields with filterable kinds', () => {
    const fields = fieldsForEventType(EVENT_KIND.tiktokChat);
    const byPath = new Map(fields.map((field) => [field.path, field]));
    expect(byPath.get('event.intel.comment.composition.emojiOnly')?.kind).toBe('boolean');
    expect(byPath.get('event.intel.comment.spam.score')?.kind).toBe('number');
    expect(byPath.get('event.intel.comment.tts.text')?.kind).toBe('string');
    expect(byPath.get('event.intel.comment.language.top')?.kind).toBe('string');
    expect(byPath.get('event.intel.user.nickname.tts.pronunciation.ipa')?.kind).toBe('string');
    for (const field of byPath.values()) {
      if (field.path.startsWith('event.intel.')) expect(field.optional).toBe(true);
    }
    // The chat sample carries a representative optional intel object.
    const sample = sampleEventForType(EVENT_KIND.tiktokChat);
    expect(readPath(sample, 'event.intel.comment.composition.emojiOnly')).toBe(false);
    expect(readPath(sample, 'event.intel.comment.tts.text')).toBe('hello there');
  });

  test('condition editor offers intel evidence with matching operators', () => {
    const fields = fieldsForTrigger(EVENT_KIND.tiktokChat);
    const emojiOnly = fields.find((field) => field.path === 'event.intel.comment.composition.emojiOnly');
    const spamScore = fields.find((field) => field.path === 'event.intel.comment.spam.score');
    const language = fields.find((field) => field.path === 'event.intel.comment.language.top');
    expect(emojiOnly?.kind).toBe('boolean');
    expect(spamScore?.kind).toBe('number');
    expect(language?.kind).toBe('text');
    // Events without comments only expose nickname evidence, never comment paths.
    const gift = fieldsForTrigger(EVENT_KIND.tiktokGift).map((field) => field.path);
    expect(gift.some((path) => path.startsWith('event.intel.comment.'))).toBe(false);
    expect(gift).toContain('event.intel.user.nickname.tts.text');
  });

  test('registry version tracks the protobuf-provenance shape', () => {
    expect(EVENT_REGISTRY_VERSION).toBe(9);
  });

  test('chat comment traces back to the vendor content field', () => {
    const comment = fieldsForEventType(EVENT_KIND.tiktokChat).find((field) => field.path === 'event.data.comment');
    expect(comment?.sourceMethod).toBe('WebcastChatMessage');
    expect(comment?.sourcePath).toBe('content');
    expect(comment?.sourceJsonPath).toBe('content');
    expect(comment?.sourceProtoType).toBe('string');
    expect(comment?.sourceCardinality).toBe('optional');
    expect(comment?.sourceTransform).toBe('native');
  });

  test('gift id traces back to gift_id with its string transform', () => {
    const giftId = fieldsForEventType(EVENT_KIND.tiktokGift).find((field) => field.path === 'event.data.giftId');
    expect(giftId?.sourceMethod).toBe('WebcastGiftMessage');
    expect(giftId?.sourcePath).toBe('gift_id');
    expect(giftId?.sourceJsonPath).toBe('giftId');
    expect(giftId?.sourceProtoType).toBe('int64');
    expect(giftId?.sourceCardinality).toBe('optional');
    expect(giftId?.sourceTransform).toBe('u64-to-string');
    const giftName = fieldsForEventType(EVENT_KIND.tiktokGift).find((field) => field.path === 'event.data.giftName');
    expect(giftName?.sourceMethod).toBe('WebcastGiftMessage');
    expect(giftName?.sourcePath).toBe('gift.name');
    expect(giftName?.sourceJsonPath).toBe('gift.name');
    expect(giftName?.sourceProtoType).toBe('string');
  });

  test('nested gift fields trace back through the gift detail message', () => {
    const diamonds = fieldsForEventType(EVENT_KIND.tiktokGift).find((field) => field.path === 'event.data.diamondCount');
    expect(diamonds?.sourceMethod).toBe('WebcastGiftMessage');
    expect(diamonds?.sourcePath).toBe('gift.diamond_count');
    expect(diamonds?.sourceJsonPath).toBe('gift.diamondCount');
    expect(diamonds?.sourceProtoType).toBe('int32');
    expect(diamonds?.sourceTransform).toBe('normalized-unsigned');
  });

  test('protobuf-backed entries name the vendor message as their source', () => {
    const gift = registryEntryFor(EVENT_KIND.tiktokGift);
    expect(gift?.dataInterface).toBe('GiftAutomationData');
    expect(gift?.sourceInterface).toBe('WebcastGiftMessage');
    const chat = registryEntryFor(EVENT_KIND.tiktokChat);
    expect(chat?.dataInterface).toBe('ChatAutomationData');
    expect(chat?.sourceInterface).toBe('WebcastChatMessage');
    // App-only contracts keep the DTO as their source.
    const connected = registryEntryFor('tiktok.connected');
    expect(connected?.dataInterface).toBe('ConnectionAutomationData');
    expect(connected?.sourceInterface).toBe('ConnectionAutomationData');
  });

  test('TikTools-only fields claim no protobuf provenance', () => {
    const appOnly = ['event.data.method', 'event.data.msgId', 'event.data.isHistory'];
    for (const type of ALL_TYPES) {
      for (const field of fieldsForEventType(type)) {
        if (
          appOnly.includes(field.path)
          || (type === EVENT_KIND.tiktokGift && ['event.data.streakable', 'event.data.giftIconUrl'].includes(field.path))
        ) {
          expect(field.sourceMethod, `${type} ${field.path}`).toBeUndefined();
          expect(field.sourcePath, `${type} ${field.path}`).toBeUndefined();
          expect(field.sourceJsonPath, `${type} ${field.path}`).toBeUndefined();
          expect(field.sourceProtoType, `${type} ${field.path}`).toBeUndefined();
          expect(field.sourceCardinality, `${type} ${field.path}`).toBeUndefined();
          expect(field.sourceTransform, `${type} ${field.path}`).toBeUndefined();
        }
      }
    }
  });

  test('intel filters match enriched events and miss raw ones', () => {
    const enriched = sampleEventForType(EVENT_KIND.tiktokChat);
    expect(matchesFilter({ path: 'event.intel.comment.composition.emojiOnly', operator: 'is-false', value: '' }, enriched)).toBe(true);
    expect(matchesFilter({ path: 'event.intel.comment.spam.score', operator: 'lt', value: '0.70' }, enriched)).toBe(true);
    expect(matchesFilter({ path: 'event.intel.comment.language.top', operator: 'eq', value: 'en' }, enriched)).toBe(true);
    const raw = { ...enriched, intel: undefined };
    expect(matchesFilter({ path: 'event.intel.comment.composition.emojiOnly', operator: 'is-true', value: '' }, raw)).toBe(false);
  });
});


const HOTKEY: PluginEventType = {
  type: EVENT_KIND.hotkeyPressed,
  title: { default: 'Hotkey pressed', i18key: EVENT_KIND.hotkeyPressed },
  fields: [
    {
      path: 'event.data.key',
      kind: 'text',
      label: { default: 'Key', i18key: 'x' },
      options: [{ value: 'k' }, { value: 'space', label: { default: 'Space', i18key: 'y' } }],
    },
    {
      path: 'event.data.modifiers',
      kind: 'text',
      options: [{ value: '' }, { value: 'ctrl' }],
    },
  ],
  sample: { key: 'k', modifiers: 'ctrl' },
  source: { kind: 'plugin', pluginId: 'hotkeys' },
};

function liveEvent(trigger: string): Record<string, unknown> {
  return {
    id: 'evt-1',
    name: 'HK',
    enabled: true,
    trigger,
    filters: [],
    cooldownMs: 0,
    cooldownScope: 'user',
    actionIds: [],
    runMode: 'all',
  };
}

describe('plugin event-type overlay', () => {
  test('merges declared types without touching builtins', () => {
    try {
      setPluginEventTypes([HOTKEY]);
      expect(registryEventTypes()).toContain(EVENT_KIND.hotkeyPressed);
      expect(pluginEventTypes().map((entry) => entry.type)).toEqual([EVENT_KIND.hotkeyPressed]);
      expect(sampleEventForType(EVENT_KIND.hotkeyPressed).data).toMatchObject({ key: 'k', modifiers: 'ctrl' });
      const keyFields = fieldsForTrigger(EVENT_KIND.hotkeyPressed);
      expect(keyFields.map((field) => field.path)).toEqual(['event.data.key', 'event.data.modifiers']);
      expect(keyFields[0]?.options?.map((option) => option.value)).toEqual(['k', 'space']);
      expect(keyFields[0]?.options?.[1]?.label.default).toBe('Space');
      // Empty-string options ("none") survive the overlay.
      expect(keyFields[1]?.options?.map((option) => option.value)).toEqual(['', 'ctrl']);
      expect(allRegistryFields().some((field) => field.path === 'event.data.key')).toBe(true);
      expect(normalizeEvent(liveEvent(EVENT_KIND.hotkeyPressed), [EVENT_KIND.hotkeyPressed]).trigger).toBe(EVENT_KIND.hotkeyPressed);
      expect(() => normalizeEvent(liveEvent('nope.dots'), [EVENT_KIND.hotkeyPressed])).toThrow();
    } finally {
      setPluginEventTypes([]);
    }
    expect(registryEventTypes()).toEqual(ALL_TYPES);
    expect(pluginEventTypes()).toEqual([]);
  });
});

test('blankEventForType keeps the contract shape without sample sentinels', () => {
  const blank = blankEventForType(EVENT_KIND.tiktokChat);
  const data = dataOf(blank);
  expect(data).toHaveProperty('comment');
  expect(data).not.toHaveProperty('method');
  expect(data).not.toHaveProperty('msgId');
  expect(data).not.toHaveProperty('emotes');
  expect(blank.id).toBe('custom-event');
  expect(data.comment).toBe('');
  expect(blank).not.toHaveProperty('user');
  expect(blank).not.toHaveProperty('intel');
  expect(blankEventForType(EVENT_KIND.tiktokGift).data).toHaveProperty('giftName');
});

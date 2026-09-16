import { describe, expect, test } from 'bun:test';

import {
  allRegistryFields,
  fieldsForEventType,
  pluginEventTypes,
  registryEventTypes,
  registryHasPath,
  sampleDataForType,
  sampleEventForType,
  setPluginEventTypes,
} from './event-registry.ts';
import { fieldsForTrigger } from './behavior/fields.ts';
import { matchesFilter } from './behavior/filters.ts';
import { sampleEventFor } from './behavior/samples.ts';
import type { AutomationEventType, JsonValue } from './types.ts';
import { BUILTIN_EVENT_TYPES } from './contracts/events.ts';
import { normalizeEvent } from './behavior/schema.ts';
import type { PluginEventType } from './behavior/types.ts';

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
      current = (current as Record<string, JsonValue | undefined>)[part];
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
      const sample = sampleEventFor(trigger);
      for (const field of fieldsForTrigger(trigger)) {
        expect(registryHasPath(trigger, field.path), `${trigger} ${field.path}`).toBe(true);
        expect(readPath(sample, field.path), `${trigger} ${field.path}`).not.toBeUndefined();
      }
    }
  });

  test('gift sample keeps the values condition tests rely on', () => {
    const sample = sampleEventFor('tiktok.gift');
    expect(matchesFilter({ path: 'event.data.giftName', operator: 'eq', value: 'Rosa' }, sample)).toBe(true);
    expect(matchesFilter({ path: 'event.data.diamondCount', operator: 'gte', value: '1' }, sample)).toBe(true);
  });

  test('proto-derived events document their vendor message', () => {
    for (const type of ['tiktok.chat', 'tiktok.gift', 'tiktok.like', 'tiktok.join', 'tiktok.social', 'tiktok.room_stats'] as const) {
      const fields = fieldsForEventType(type);
      expect(fields.length).toBeGreaterThan(5);
      expect(fields.some((field) => field.sourceField)).toBe(true);
    }
  });

  test('data payloads are keyed by field name, not hardcoded lists', () => {
    expect(Object.keys(sampleDataForType('tiktok.gift'))).toContain('giftName');
    expect(Object.keys(sampleDataForType('tiktok.chat'))).toContain('comment');
    expect(allRegistryFields().some((field) => field.path === 'event.user.uniqueId')).toBe(true);
  });

  test('chat exposes stable processor fields with filterable kinds', () => {
    const fields = fieldsForEventType('tiktok.chat');
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
    const sample = sampleEventForType('tiktok.chat');
    expect(readPath(sample, 'event.intel.comment.composition.emojiOnly')).toBe(false);
    expect(readPath(sample, 'event.intel.comment.tts.text')).toBe('hello there');
  });

  test('condition editor offers intel evidence with matching operators', () => {
    const fields = fieldsForTrigger('tiktok.chat');
    const emojiOnly = fields.find((field) => field.path === 'event.intel.comment.composition.emojiOnly');
    const spamScore = fields.find((field) => field.path === 'event.intel.comment.spam.score');
    const language = fields.find((field) => field.path === 'event.intel.comment.language.top');
    expect(emojiOnly?.kind).toBe('boolean');
    expect(spamScore?.kind).toBe('number');
    expect(language?.kind).toBe('text');
    // Events without comments only expose nickname evidence, never comment paths.
    const gift = fieldsForTrigger('tiktok.gift').map((field) => field.path);
    expect(gift.some((path) => path.startsWith('event.intel.comment.'))).toBe(false);
    expect(gift).toContain('event.intel.user.nickname.tts.text');
  });

  test('intel filters match enriched events and miss raw ones', () => {
    const enriched = sampleEventFor('tiktok.chat');
    expect(matchesFilter({ path: 'event.intel.comment.composition.emojiOnly', operator: 'is-false', value: '' }, enriched)).toBe(true);
    expect(matchesFilter({ path: 'event.intel.comment.spam.score', operator: 'lt', value: '0.70' }, enriched)).toBe(true);
    expect(matchesFilter({ path: 'event.intel.comment.language.top', operator: 'eq', value: 'en' }, enriched)).toBe(true);
    const raw = { ...enriched, intel: undefined };
    expect(matchesFilter({ path: 'event.intel.comment.composition.emojiOnly', operator: 'is-true', value: '' }, raw)).toBe(false);
  });
});


const HOTKEY: PluginEventType = {
  type: 'hotkey.pressed',
  title: { default: 'Hotkey pressed', i18key: 'hotkey.pressed' },
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
  sample: { key: 'ctrl+k' },
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
      expect(registryEventTypes()).toContain('hotkey.pressed');
      expect(pluginEventTypes().map((entry) => entry.type)).toEqual(['hotkey.pressed']);
      expect(sampleDataForType('hotkey.pressed')).toMatchObject({ key: 'ctrl+k' });
      const keyFields = fieldsForTrigger('hotkey.pressed');
      expect(keyFields.map((field) => field.path)).toEqual(['event.data.key', 'event.data.modifiers']);
      expect(keyFields[0]?.options?.map((option) => option.value)).toEqual(['k', 'space']);
      expect(keyFields[0]?.options?.[1]?.label.default).toBe('Space');
      // Empty-string options ("none") survive the overlay.
      expect(keyFields[1]?.options?.map((option) => option.value)).toEqual(['', 'ctrl']);
      expect(allRegistryFields().some((field) => field.path === 'event.data.key')).toBe(true);
      expect(normalizeEvent(liveEvent('hotkey.pressed'), ['hotkey.pressed']).trigger).toBe('hotkey.pressed');
      expect(() => normalizeEvent(liveEvent('nope.dots'), ['hotkey.pressed'])).toThrow();
    } finally {
      setPluginEventTypes([]);
    }
    expect(registryEventTypes()).toEqual(ALL_TYPES);
    expect(pluginEventTypes()).toEqual([]);
  });
});

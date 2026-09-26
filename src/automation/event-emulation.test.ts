import { describe, expect, test } from 'bun:test';

import {
  emulateEventForType,
  makeGiftComboEvents,
  matchingLastEvent,
  refreshEmulatedEvent,
} from './event-emulation.ts';
import { sampleEventForType } from './event-registry.ts';

describe('matchingLastEvent', () => {
  test('accepts only the trigger being emulated', () => {
    const gift = sampleEventForType('tiktok.gift');
    expect(matchingLastEvent('tiktok.gift', gift)).toBe(gift);
    expect(matchingLastEvent('tiktok.chat', gift)).toBeUndefined();
    expect(matchingLastEvent('tiktok.gift', null)).toBeUndefined();
    expect(matchingLastEvent('tiktok.gift', undefined)).toBeUndefined();
  });
});

describe('emulateEventForType', () => {
  test('replays matching live data with a fresh timestamp', () => {
    const live = sampleEventForType('tiktok.gift');
    live.timestamp = 1;
    (live.data as Record<string, unknown>)['giftName'] = 'Galaxy';
    const before = Date.now();
    const { event, source } = emulateEventForType('tiktok.gift', live);
    expect(source).toBe('live');
    expect((event.data as Record<string, unknown>)['giftName']).toBe('Galaxy');
    expect(event.timestamp).toBeGreaterThanOrEqual(before);
    // The stored live envelope is untouched.
    expect(live.timestamp).toBe(1);
  });

  test('falls back to the registry sample on type mismatch', () => {
    const chat = sampleEventForType('tiktok.chat');
    const { event, source } = emulateEventForType('tiktok.gift', chat);
    expect(source).toBe('sample');
    expect(event.type).toBe('tiktok.gift');
    expect((event.data as Record<string, unknown>)['giftName']).toBe('Rosa');
  });
});

describe('refreshEmulatedEvent', () => {
  test('renews identity without touching the payload', () => {
    const live = sampleEventForType('tiktok.chat');
    live.id = 'live-1';
    live.timestamp = 1;
    const replay = refreshEmulatedEvent(live, 'replay-1');
    expect(replay.id).toBe('replay-1');
    expect(replay.timestamp).toBeGreaterThan(1);
    expect(replay.data).toEqual(live.data);
    expect(live.id).toBe('live-1');
  });
});

describe('makeGiftComboEvents', () => {
  test('builds an x1..xn streak sharing one group id', () => {
    const combo = makeGiftComboEvents(3, { giftName: 'Rose', diamondCount: 1 });
    expect(combo).toHaveLength(3);
    const groups = new Set(combo.map((event) => (event.data as Record<string, unknown>)['groupId']));
    expect(groups.size).toBe(1);
    expect(combo.map((event) => (event.data as Record<string, unknown>)['repeatCount'])).toEqual([1, 2, 3]);
    expect(combo.map((event) => (event.data as Record<string, unknown>)['streakable'])).toEqual([true, true, true]);
    expect(combo.map((event) => (event.data as Record<string, unknown>)['repeatEnd'])).toEqual([false, false, true]);
    // Distinct ids so replays never look like duplicates.
    expect(new Set(combo.map((event) => event.id)).size).toBe(3);
  });
});

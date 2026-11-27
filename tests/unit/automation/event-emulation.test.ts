import { describe, expect, test } from 'vitest';

import {
  emulateEventForType,
  makeGiftComboEvents,
  matchingLastEvent,
  refreshEmulatedEvent,
} from '../../../src/automation/event-emulation.ts';
import { sampleEventForType } from '../../../src/automation/event-registry.ts';
import { EVENT_KIND } from '../../shared/index.ts';
import { dataOf } from '../support/automation-event.ts';

describe('matchingLastEvent', () => {
  test('accepts only the trigger being emulated', () => {
    const gift = sampleEventForType(EVENT_KIND.tiktokGift);
    expect(matchingLastEvent(EVENT_KIND.tiktokGift, gift)).toBe(gift);
    expect(matchingLastEvent(EVENT_KIND.tiktokChat, gift)).toBeUndefined();
    expect(matchingLastEvent(EVENT_KIND.tiktokGift, null)).toBeUndefined();
    expect(matchingLastEvent(EVENT_KIND.tiktokGift, undefined)).toBeUndefined();
  });
});

describe('emulateEventForType', () => {
  test('replays matching live data with a fresh timestamp', () => {
    const live = sampleEventForType(EVENT_KIND.tiktokGift);
    live.timestamp = 1;
    dataOf(live)['giftName'] = 'Galaxy';
    const before = Date.now();
    const { event, source } = emulateEventForType(EVENT_KIND.tiktokGift, live);
    expect(source).toBe('live');
    expect(dataOf(event)['giftName']).toBe('Galaxy');
    expect(event.timestamp).toBeGreaterThanOrEqual(before);
    // The stored live envelope is untouched.
    expect(live.timestamp).toBe(1);
  });

  test('falls back to the registry sample on type mismatch', () => {
    const chat = sampleEventForType(EVENT_KIND.tiktokChat);
    const { event, source } = emulateEventForType(EVENT_KIND.tiktokGift, chat);
    expect(source).toBe('sample');
    expect(event.type).toBe(EVENT_KIND.tiktokGift);
    expect(dataOf(event)['giftName']).toBe('Rose');
  });
});

describe('refreshEmulatedEvent', () => {
  test('renews identity without touching the payload', () => {
    const live = sampleEventForType(EVENT_KIND.tiktokChat);
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
    const groups = new Set(combo.map((event) => dataOf(event)['groupId']));
    expect(groups.size).toBe(1);
    expect(combo.map((event) => dataOf(event)['repeatCount'])).toEqual([1, 2, 3]);
    expect(combo.map((event) => dataOf(event)['streakable'])).toEqual([true, true, true]);
    expect(combo.map((event) => dataOf(event)['repeatEnd'])).toEqual([false, false, true]);
    // Distinct ids so replays never look like duplicates.
    expect(new Set(combo.map((event) => event.id)).size).toBe(3);
  });
});

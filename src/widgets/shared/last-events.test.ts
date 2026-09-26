import { describe, expect, test } from 'bun:test';

import { parseLiveEvent } from './event-parser.ts';
import { LastEventCache } from './last-events.ts';
import { makeTestEnvelopeFor, makeTestGapEnvelope, makeTestGiftEnvelope } from './test-events.ts';

describe('LastEventCache', () => {
  test('records one envelope per type and ignores non-live frames', () => {
    const cache = new LastEventCache();
    cache.record(makeTestGapEnvelope());
    cache.record(null);
    cache.record({ topic: 'live.event', data: { nope: true } });
    expect(cache.size).toBe(0);

    cache.record(makeTestEnvelopeFor('tiktok.gift', { data: { giftName: 'Galaxy' } }));
    cache.record(makeTestEnvelopeFor('tiktok.chat'));
    cache.record(makeTestEnvelopeFor('tiktok.gift', { data: { giftName: 'Rose' } }));
    expect(cache.types).toEqual(['tiktok.gift', 'tiktok.chat']);
  });

  test('replays the latest payload with fresh identity', () => {
    const cache = new LastEventCache();
    expect(cache.replay('tiktok.gift')).toBeNull();
    cache.record(makeTestGiftEnvelope({ giftName: 'Galaxy', diamondCount: 1000 }));
    const replay = cache.replay('tiktok.gift');
    expect(replay).not.toBeNull();
    const payload = parseLiveEvent(replay!);
    const data = (payload?.event.data ?? {}) as Record<string, unknown>;
    expect(data['giftName']).toBe('Galaxy');
    expect(data['diamondCount']).toBe(1000);
    expect(payload?.event.id).toContain('-replay-');
    // Replaying never mutates the stored envelope.
    const again = cache.replay('tiktok.gift');
    expect(again).not.toBeNull();
    expect(parseLiveEvent(again!)?.event.id).toContain('-replay-');
  });
});

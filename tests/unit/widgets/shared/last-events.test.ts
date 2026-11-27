import { describe, expect, test } from 'vitest';

import { parseLiveEvent } from '../../../../src/widgets/shared/event-parser.ts';
import { LastEventCache } from '../../../../src/widgets/shared/last-events.ts';
import { makeTestEnvelopeFor, makeTestGapEnvelope, makeTestGiftEnvelope } from '../../../../src/widgets/shared/test-events.ts';
import { EVENT_KIND, TOPIC } from '../../../shared/index.ts';

describe('LastEventCache', () => {
  test('records one envelope per type and ignores non-live frames', () => {
    const cache = new LastEventCache();
    cache.record(makeTestGapEnvelope());
    cache.record(null);
    cache.record({ topic: TOPIC.liveEvent, data: { nope: true } });
    expect(cache.size).toBe(0);

    cache.record(makeTestEnvelopeFor(EVENT_KIND.tiktokGift, { data: { giftName: 'Galaxy' } }));
    cache.record(makeTestEnvelopeFor(EVENT_KIND.tiktokChat));
    cache.record(makeTestEnvelopeFor(EVENT_KIND.tiktokGift, { data: { giftName: 'Rose' } }));
    expect(cache.types).toEqual([EVENT_KIND.tiktokGift, EVENT_KIND.tiktokChat]);
  });

  test('replays the latest payload with fresh identity', () => {
    const cache = new LastEventCache();
    expect(cache.replay(EVENT_KIND.tiktokGift)).toBeNull();
    cache.record(makeTestGiftEnvelope({ giftName: 'Galaxy', diamondCount: 1000 }));
    const replay = cache.replay(EVENT_KIND.tiktokGift);
    expect(replay).not.toBeNull();
    const payload = parseLiveEvent(replay!);
    const data = (payload?.event.data ?? {}) as Record<string, unknown>;
    expect(data['giftName']).toBe('Galaxy');
    expect(data['diamondCount']).toBe(1000);
    expect(payload?.event.id).toContain('-replay-');
    // Replaying never mutates the stored envelope.
    const again = cache.replay(EVENT_KIND.tiktokGift);
    expect(again).not.toBeNull();
    expect(parseLiveEvent(again!)?.event.id).toContain('-replay-');
  });
});

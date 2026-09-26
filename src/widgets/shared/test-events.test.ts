import { describe, expect, test } from 'bun:test';

import {
  isConnectionEvent,
  isGiftEvent,
  isJoinEvent,
  isLikeEvent,
  isPluginEmitEvent,
  isPointsAwardedEvent,
  isRoomStatsEvent,
  isSocialEvent,
  parseLiveEvent,
} from './event-parser.ts';
import {
  makeTestConnectionEnvelope,
  makeTestEnvelopeFor,
  makeTestJoinEnvelope,
  makeTestLikeEnvelope,
  makeTestPluginEmitEnvelope,
  makeTestPointsAwardedEnvelope,
  makeTestRoomStatsEnvelope,
  makeTestSocialEnvelope,
  makeTestSubscribeEnvelope,
  TEST_EMULATABLE_TYPES,
} from './test-events.ts';

describe('extended test-event factories', () => {
  test('like envelopes validate as like events', () => {
    const payload = parseLiveEvent(makeTestLikeEnvelope({ count: 7, total: 42 }));
    expect(payload?.eventType).toBe('tiktok.like');
    expect(payload && isLikeEvent(payload.event)).toBe(true);
  });

  test('social envelopes use the generic action, not follow/share', () => {
    const payload = parseLiveEvent(makeTestSocialEnvelope());
    expect(payload && isSocialEvent(payload.event)).toBe(true);
    const data = (payload?.event.data ?? {}) as Record<string, unknown>;
    expect(data['action']).toBe(0);
  });

  test('plain joins and subscribes share the type but not the action', () => {
    const join = parseLiveEvent(makeTestJoinEnvelope());
    expect(join && isJoinEvent(join.event)).toBe(true);
    expect(((join?.event.data ?? {}) as Record<string, unknown>)['action']).toBe(0);
    const subscribe = parseLiveEvent(makeTestSubscribeEnvelope());
    expect(subscribe && isJoinEvent(subscribe.event)).toBe(true);
    expect(((subscribe?.event.data ?? {}) as Record<string, unknown>)['action']).toBe(3);
  });

  test('room stats envelopes validate without a user', () => {
    const payload = parseLiveEvent(makeTestRoomStatsEnvelope({ viewers: 10 }));
    expect(payload?.eventType).toBe('tiktok.room_stats');
    expect(payload && isRoomStatsEvent(payload.event)).toBe(true);
    expect(payload?.event.user).toBeUndefined();
  });

  test('connection envelopes validate for both directions', () => {
    for (const type of ['tiktok.connected', 'tiktok.disconnected'] as const) {
      const payload = parseLiveEvent(makeTestConnectionEnvelope(type));
      expect(payload?.eventType).toBe(type);
      expect(payload && isConnectionEvent(payload.event)).toBe(true);
    }
  });

  test('points and internal emits validate', () => {
    const points = parseLiveEvent(makeTestPointsAwardedEnvelope({ delta: 5 }));
    expect(points && isPointsAwardedEvent(points.event)).toBe(true);
    const emit = parseLiveEvent(makeTestPluginEmitEnvelope({ emitType: 'demo.ping' }));
    expect(emit && isPluginEmitEvent(emit.event)).toBe(true);
  });
});

describe('makeTestEnvelopeFor', () => {
  test('covers every emulatable trigger', () => {
    expect(TEST_EMULATABLE_TYPES).toHaveLength(12);
    for (const type of TEST_EMULATABLE_TYPES) {
      const payload = parseLiveEvent(makeTestEnvelopeFor(type));
      expect(payload?.eventType, type).toBe(type);
    }
  });

  test('merges data overrides and honors identity overrides', () => {
    const envelope = makeTestEnvelopeFor('tiktok.gift', {
      id: 'custom-1',
      user: { uniqueId: 'luna_dev' },
      data: { giftName: 'Galaxy', diamondCount: 1000 },
    });
    const payload = parseLiveEvent(envelope);
    expect(payload && isGiftEvent(payload.event)).toBe(true);
    expect(payload?.event.id).toBe('custom-1');
    expect(payload?.event.user?.uniqueId).toBe('luna_dev');
    const data = (payload?.event.data ?? {}) as Record<string, unknown>;
    expect(data['giftName']).toBe('Galaxy');
    expect(data['diamondCount']).toBe(1000);
  });

  test('unknown types degrade to a minimal envelope', () => {
    const payload = parseLiveEvent(makeTestEnvelopeFor('plugin.unknown'));
    expect(payload?.eventType).toBe('plugin.unknown');
    expect(payload?.event.data).toEqual({});
  });
});

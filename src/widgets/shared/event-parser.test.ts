import { describe, expect, test } from 'bun:test';

import { FOLLOW_EVENT_TYPE, GIFT_EVENT_TYPE } from './config.ts';
import {
  isDomainEventEnvelope,
  isFollowEvent,
  isGatewayControlMessage,
  isGiftEvent,
  isHistoryEvent,
  parseEventGap,
  parseLiveEvent,
} from './event-parser.ts';
import { displayNameFor, handleFor } from './event-types.ts';
import {
  makeTestFollowEnvelope,
  makeTestGapEnvelope,
  makeTestGiftEnvelope,
  makeTestUser,
} from './test-events.ts';

describe('event parser', () => {
  test('accepts well-formed envelopes and rejects malformed frames', () => {
    expect(isDomainEventEnvelope({ topic: 'live.event', data: {} })).toBe(true);
    expect(isDomainEventEnvelope({ topic: 'live.event' })).toBe(false);
    expect(isDomainEventEnvelope({ data: {} })).toBe(false);
    expect(isDomainEventEnvelope(null)).toBe(false);
    expect(isDomainEventEnvelope('live.event')).toBe(false);
    expect(isDomainEventEnvelope([])).toBe(false);
  });

  test('recognizes gateway control messages', () => {
    expect(isGatewayControlMessage({ type: 'authenticated' })).toBe(true);
    expect(isGatewayControlMessage({ type: 'pong' })).toBe(true);
    expect(isGatewayControlMessage({ type: 'subscribed', topics: ['live.event'] })).toBe(true);
    expect(isGatewayControlMessage({ type: 'error', error: 'nope' })).toBe(true);
    expect(isGatewayControlMessage({ type: 'subscribed', topics: 'live.event' })).toBe(false);
    expect(isGatewayControlMessage({ type: 'bogus' })).toBe(false);
    expect(isGatewayControlMessage({ topic: 'live.event', data: {} })).toBe(false);
  });

  test('parses follow envelopes into typed events', () => {
    const payload = parseLiveEvent(makeTestFollowEnvelope());
    expect(payload?.eventType).toBe(FOLLOW_EVENT_TYPE);
    expect(payload && isFollowEvent(payload.event)).toBe(true);
    expect(payload && isGiftEvent(payload.event)).toBe(false);
  });

  test('parses gift envelopes into typed events', () => {
    const payload = parseLiveEvent(makeTestGiftEnvelope());
    expect(payload?.eventType).toBe(GIFT_EVENT_TYPE);
    expect(payload && isGiftEvent(payload.event)).toBe(true);
    expect(payload && isFollowEvent(payload.event)).toBe(false);
  });

  test('rejects non-live topics and malformed payloads without throwing', () => {
    expect(parseLiveEvent({ topic: 'room.stats', data: {} })).toBeNull();
    expect(parseLiveEvent({ topic: 'live.event', data: null })).toBeNull();
    expect(parseLiveEvent({ topic: 'live.event', data: { eventType: 'x' } })).toBeNull();
    expect(
      parseLiveEvent({ topic: 'live.event', data: { eventType: 'x', event: { id: '', type: 'x' } } }),
    ).toBeNull();
  });

  test('rejects mistyped gift and follow payloads', () => {
    const gift = makeTestGiftEnvelope({ diamondCount: Number.NaN });
    const payload = parseLiveEvent(gift);
    expect(payload && isGiftEvent(payload.event)).toBe(false);

    const follow = makeTestFollowEnvelope();
    const followEvent = (follow.data as Record<string, unknown>)['event'] as Record<string, unknown>;
    const tampered = {
      topic: 'live.event',
      data: { eventType: FOLLOW_EVENT_TYPE, event: { ...followEvent, data: { action: 'one' } } },
    };
    const parsed = parseLiveEvent(tampered);
    expect(parsed && isFollowEvent(parsed.event)).toBe(false);
  });

  test('detects historical events', () => {
    const live = parseLiveEvent(makeTestFollowEnvelope());
    const history = parseLiveEvent(makeTestFollowEnvelope({ isHistory: true }));
    expect(live && isHistoryEvent(live.event)).toBe(false);
    expect(history && isHistoryEvent(history.event)).toBe(true);
  });

  test('parses event.gap frames', () => {
    expect(parseEventGap(makeTestGapEnvelope(4))).toEqual({ lost: 4, resync: true });
    expect(parseEventGap(makeTestFollowEnvelope())).toBeNull();
    expect(parseEventGap({ topic: 'event.gap', data: { lost: 'many' } })).toBeNull();
  });

  test('display name falls back nickname -> uniqueId -> Viewer', () => {
    expect(displayNameFor(makeTestUser({ nickname: 'Nick', uniqueId: 'uid' }))).toBe('Nick');
    expect(displayNameFor(makeTestUser({ nickname: '  ', uniqueId: 'uid' }))).toBe('uid');
    expect(displayNameFor(null)).toBe('Viewer');
    expect(displayNameFor(undefined)).toBe('Viewer');
    expect(handleFor(makeTestUser({ uniqueId: 'uid' }))).toBe('@uid');
    expect(handleFor(null)).toBe('');
  });
});

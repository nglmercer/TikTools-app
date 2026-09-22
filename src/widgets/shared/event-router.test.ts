import { describe, expect, test } from 'bun:test';

import { routeLiveEvent } from './event-router.ts';
import {
  makeTestChatEnvelope,
  makeTestFollowEnvelope,
  makeTestGapEnvelope,
  makeTestGiftEnvelope,
  makeTestShareEnvelope,
  makeTestSubscribeEnvelope,
} from './test-events.ts';

describe('event router', () => {
  test('routes follow and gift envelopes to typed events', () => {
    const follow = routeLiveEvent(makeTestFollowEnvelope());
    expect(follow?.kind).toBe('follow');
    if (follow?.kind === 'follow') {
      expect(follow.event.type).toBe('tiktok.follow');
      expect(follow.event.data.action).toBe(1);
    }
    const gift = routeLiveEvent(makeTestGiftEnvelope());
    expect(gift?.kind).toBe('gift');
    if (gift?.kind === 'gift') {
      expect(gift.event.type).toBe('tiktok.gift');
      expect(gift.event.data.streakable).toBe(false);
    }
  });

  test('routes chat, share, and subscribe envelopes to typed events', () => {
    const chat = routeLiveEvent(makeTestChatEnvelope({ comment: 'hello' }));
    expect(chat?.kind).toBe('chat');
    if (chat?.kind === 'chat') {
      expect(chat.event.type).toBe('tiktok.chat');
      expect(chat.event.data.comment).toBe('hello');
    }
    const share = routeLiveEvent(makeTestShareEnvelope());
    expect(share?.kind).toBe('share');
    if (share?.kind === 'share') {
      expect(share.event.type).toBe('tiktok.share');
    }
    const subscribe = routeLiveEvent(makeTestSubscribeEnvelope());
    expect(subscribe?.kind).toBe('subscribe');
    if (subscribe?.kind === 'subscribe') {
      expect(subscribe.event.type).toBe('tiktok.join');
      expect(subscribe.event.data.action).toBe(3);
    }
  });

  test('plain joins stay unrouted', () => {
    expect(routeLiveEvent(makeTestSubscribeEnvelope({ action: 1 }))).toBeNull();
    expect(routeLiveEvent(makeTestSubscribeEnvelope({ action: 0 }))).toBeNull();
  });

  test('ignores gaps, unknown types, and malformed frames', () => {
    expect(routeLiveEvent(makeTestGapEnvelope())).toBeNull();
    expect(routeLiveEvent({ topic: 'live.event', data: { eventType: 'tiktok.chat', event: null } })).toBeNull();
    expect(routeLiveEvent({ topic: 'live.event', data: {} })).toBeNull();
    expect(routeLiveEvent(null)).toBeNull();
    expect(routeLiveEvent({ type: 'pong' })).toBeNull();
  });
});

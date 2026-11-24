import { describe, expect, test } from 'vitest';

import { eventIdentity, type AutomationEventLike } from '../../../../src/widgets/shared/event-types.ts';

function event(overrides: Partial<AutomationEventLike> & { msgId?: string } = {}): AutomationEventLike {
  const { msgId, ...rest } = overrides;
  return {
    id: rest.id ?? 'envelope-1',
    type: rest.type ?? 'tiktok.share',
    timestamp: rest.timestamp ?? 1,
    creator: rest.creator,
    user: rest.user ?? null,
    data: msgId === undefined ? (rest.data ?? {}) : { ...(rest.data as object), msgId },
  };
}

describe('eventIdentity', () => {
  test('keys on the upstream message id, not the minted envelope id', () => {
    expect(eventIdentity(event({ msgId: '9007199254740993' }))).toBe(JSON.stringify(['', '', 'tiktok.share', '9007199254740993']));
  });

  test('two emissions of the same message share one identity', () => {
    const first = event({ id: 'envelope-1', msgId: 'msg-9' });
    const replay = event({ id: 'envelope-2', msgId: 'msg-9' });
    expect(eventIdentity(first)).toBe(eventIdentity(replay));
  });

  test('distinct messages stay distinct', () => {
    expect(eventIdentity(event({ msgId: 'msg-1' }))).not.toBe(
      eventIdentity(event({ msgId: 'msg-2' })),
    );
  });

  test('the room scopes the identity so two rooms cannot collide', () => {
    const roomA = event({ msgId: 'msg-5', creator: { uniqueId: 'c', roomId: 'room-a' } });
    const roomB = event({ msgId: 'msg-5', creator: { uniqueId: 'c', roomId: 'room-b' } });
    expect(eventIdentity(roomA)).toBe(JSON.stringify(['c', 'room-a', 'tiktok.share', 'msg-5']));
    expect(eventIdentity(roomB)).toBe(JSON.stringify(['c', 'room-b', 'tiktok.share', 'msg-5']));
    expect(eventIdentity(roomA)).not.toBe(eventIdentity(roomB));
  });

  test('the same id on different event types stays distinct', () => {
    expect(eventIdentity(event({ msgId: '42', type: 'tiktok.chat' }))).not.toBe(eventIdentity(event({ msgId: '42', type: 'tiktok.emote_chat' })));
  });

  test('falls back to the envelope id when there is no usable upstream id', () => {
    const absent = event({ id: 'envelope-x' });
    expect(eventIdentity(absent)).toBe('envelope-x');
    // `0` is the proto default, not a real message id.
    const zero = event({ id: 'envelope-y', msgId: '0' });
    expect(eventIdentity(zero)).toBe('envelope-y');
    const empty = event({ id: 'envelope-z', msgId: '' });
    expect(eventIdentity(empty)).toBe('envelope-z');
  });
});

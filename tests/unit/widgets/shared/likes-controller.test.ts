import { describe, expect, test } from 'vitest';

import { LikesController, type LikeBoardEntry } from '../../../../src/widgets/shared/likes-controller.ts';
import { makeTestFollowEnvelope, makeTestLikeEvent, toLiveEnvelope } from '../../../../src/widgets/shared/test-events.ts';
import type { DomainEventEnvelope } from '../../../../src/widgets/shared/event-types.ts';

function setup(options: { maxEntries?: number; trackedCapacity?: number } = {}) {
  const boards: LikeBoardEntry[][] = [];
  const controller = new LikesController({ onBoard: (entries) => void boards.push(entries) }, options);
  return { boards, controller };
}

function likeEnvelope(
  userId: string,
  uniqueId: string,
  count: number,
  aggregationTotal?: number,
  msgId?: string,
): DomainEventEnvelope {
  const event = makeTestLikeEvent({ user: { userId, uniqueId, nickname: uniqueId }, count, msgId });
  if (aggregationTotal !== undefined) {
    event.data.aggregation = { global: null, user: { previous: 0, total: aggregationTotal } };
  }
  return toLiveEnvelope(event);
}

describe('likes controller', () => {
  test('accumulates counts per sender into a ranked board', () => {
    const { boards, controller } = setup();
    expect(controller.handleEnvelope(likeEnvelope('u1', 'ana', 5))).toBe('updated');
    expect(controller.handleEnvelope(likeEnvelope('u2', 'ben', 3))).toBe('updated');
    expect(controller.handleEnvelope(likeEnvelope('u1', 'ana', 7))).toBe('updated');
    const board = boards.at(-1) ?? [];
    expect(board.map((entry) => [entry.displayName, entry.total])).toEqual([
      ['ana', 12],
      ['ben', 3],
    ]);
    expect(board[0]?.uniqueId).toBe('@ana');
  });

  test('prefers the backend cumulative total when present', () => {
    const { boards, controller } = setup();
    expect(controller.handleEnvelope(likeEnvelope('u1', 'ana', 5, 100))).toBe('updated');
    expect(controller.handleEnvelope(likeEnvelope('u1', 'ana', 5, 90))).toBe('updated');
    expect(boards.at(-1)?.[0]?.total).toBe(100);
  });

  test('caps the board and the tracked set', () => {
    const { boards, controller } = setup({ maxEntries: 2, trackedCapacity: 3 });
    for (const [userId, uniqueId, count] of [['u1', 'ana', 1], ['u2', 'ben', 2], ['u3', 'cid', 3], ['u4', 'dan', 4]] as const) {
      expect(controller.handleEnvelope(likeEnvelope(userId, uniqueId, count))).toBe('updated');
    }
    expect(controller.trackedCount).toBe(3);
    expect(boards.at(-1)?.map((entry) => entry.displayName)).toEqual(['dan', 'cid']);
  });

  test('ignores history, duplicates, and non-like events', () => {
    const { boards, controller } = setup();
    const envelope = likeEnvelope('u1', 'ana', 5);
    expect(controller.handleEnvelope(envelope)).toBe('updated');
    expect(controller.handleEnvelope(envelope)).toBe('duplicate');
    const historic = toLiveEnvelope(makeTestLikeEvent({ user: { userId: 'u9', uniqueId: 'zed' }, isHistory: true }));
    expect(controller.handleEnvelope(historic)).toBe('history');
    expect(controller.handleEnvelope(makeTestFollowEnvelope())).toBe('ignored');
    expect(boards).toHaveLength(1);
  });

  test('ignores likes without a usable sender', () => {
    const { controller } = setup();
    const event = makeTestLikeEvent({ user: null, count: 5 });
    expect(controller.handleEnvelope(toLiveEnvelope(event))).toBe('ignored');
    const anonymous = makeTestLikeEvent({ user: { userId: '', uniqueId: '' }, count: 5 });
    expect(controller.handleEnvelope(toLiveEnvelope(anonymous))).toBe('ignored');
  });
});

import { callbacks, createControl, describe, expect, it, createLiveFeedState } from '../../../../../../src/web/entities/live-event/model/testing/live-feed-fixture.ts';
import { FIXTURE_ID, TOPIC } from '../../../../../shared/index.ts';

describe('authoritative core likes ranking', () => {
  it('keeps the likes ranking independent of feed history, and rejects another room', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    try {
      // Ranking may arrive on the lossy lane before the reliable
      // live.connected notification.
      const preconnectLeader = { rank: 1, score: 5, uniqueId: 'early', nickname: 'Early', userId: '9' };
      harness.emit(TOPIC.liveLikesRanking, {
        creatorId: 'creator', roomId: FIXTURE_ID.roomA, leaders: [preconnectLeader],
      } as never);
      harness.emit(TOPIC.liveConnected, { uniqueId: 'creator', roomId: FIXTURE_ID.roomA } as never);
      // Assert: the pre-connection ranking is already shown, so the board does not wait for live.connected
      expect(live.likeLeaders.value).toEqual([preconnectLeader]);
      const alice = { rank: 1, score: 150, uniqueId: 'alice', nickname: 'Alice', userId: '1' };
      const bob = { rank: 2, score: 90, uniqueId: 'bob', nickname: 'Bob', userId: '2' };
      harness.emit(TOPIC.liveLikesRanking, {
        creatorId: 'creator',
        roomId: FIXTURE_ID.roomA,
        leaders: [alice, bob],
      } as never);
      expect(live.likeLeaders.value).toEqual([alice, bob]);
      // Assert: ranking updates never add feed events, so the board is independent of the feed history
      expect(live.events.value).toHaveLength(0);

      // Act: a ranking for another room, with a far higher score, must be rejected rather than merged
      harness.emit(TOPIC.liveLikesRanking, {
        creatorId: 'creator',
        roomId: 'other-room',
        leaders: [{ ...alice, score: 9000 }],
      } as never);
      expect(live.likeLeaders.value).toEqual([alice, bob]);

      // Act: a fresh ranking for the joined room reorders the board, so leaders follow the new scores
      harness.emit(TOPIC.liveLikesRanking, {
        creatorId: 'creator',
        roomId: FIXTURE_ID.roomA,
        leaders: [{ ...bob, rank: 1, score: 200 }, { ...alice, rank: 2, score: 150 }],
      } as never);
      expect(live.likeLeaders.value.map((leader) => leader.uniqueId)).toEqual(['bob', 'alice']);

      // Rejoining the same room does not clear authoritative session totals.
      harness.emit(TOPIC.liveConnected, { uniqueId: 'creator', roomId: FIXTURE_ID.roomA } as never);
      expect(live.likeLeaders.value).toHaveLength(2);
      // Act: joining a different room starts a new session, which resets the authoritative totals
      harness.emit(TOPIC.liveConnected, { uniqueId: 'creator', roomId: 'room-b' } as never);
      expect(live.likeLeaders.value).toHaveLength(0);
    } finally {
      live.dispose();
    }
  });

  it('bounds ranking snapshots to ten valid positive-scoring users', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    try {
      harness.emit(TOPIC.liveLikesRanking, {
        creatorId: 'creator',
        roomId: 'room',
        leaders: [
          { rank: 1, score: 0, uniqueId: 'zero', nickname: '', userId: '0' },
          ...Array.from({ length: 30 }, (_, index) => ({
            rank: index + 2, score: 50 - index, uniqueId: `viewer-${index}`,
            nickname: '', userId: String(index + 1),
          })),
        ],
      } as never);
      expect(live.likeLeaders.value).toHaveLength(10);
      expect(live.likeLeaders.value[0]?.score).toBe(50);
      expect(live.likeLeaders.value.every((leader) => leader.score > 0)).toBe(true);
    } finally {
      live.dispose();
    }
  });
});

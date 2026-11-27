import { callbacks, createControl, describe, expect, it, createLiveFeedState } from '../../../../../../src/web/entities/live-event/model/testing/live-feed-fixture.ts';
import { TOPIC } from '../../../../../shared/index.ts';

describe('authoritative core likes ranking', () => {
  it('keeps the likes ranking independent of feed history, and rejects another room', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    try {
      // Ranking may arrive on the lossy lane before the reliable
      // live.connected notification.
      const preconnectLeader = { rank: 1, score: 5, uniqueId: 'early', nickname: 'Early', userId: '9' };
      harness.emit('live.likes-ranking', {
        creatorId: 'creator', roomId: 'room-a', leaders: [preconnectLeader],
      } as never);
      harness.emit(TOPIC.liveConnected, { uniqueId: 'creator', roomId: 'room-a' } as never);
      expect(live.likeLeaders.value).toEqual([preconnectLeader]);
      const alice = { rank: 1, score: 150, uniqueId: 'alice', nickname: 'Alice', userId: '1' };
      const bob = { rank: 2, score: 90, uniqueId: 'bob', nickname: 'Bob', userId: '2' };
      harness.emit('live.likes-ranking', {
        creatorId: 'creator',
        roomId: 'room-a',
        leaders: [alice, bob],
      } as never);
      expect(live.likeLeaders.value).toEqual([alice, bob]);
      expect(live.events.value).toHaveLength(0);

      harness.emit('live.likes-ranking', {
        creatorId: 'creator',
        roomId: 'other-room',
        leaders: [{ ...alice, score: 9000 }],
      } as never);
      expect(live.likeLeaders.value).toEqual([alice, bob]);

      harness.emit('live.likes-ranking', {
        creatorId: 'creator',
        roomId: 'room-a',
        leaders: [{ ...bob, rank: 1, score: 200 }, { ...alice, rank: 2, score: 150 }],
      } as never);
      expect(live.likeLeaders.value.map((leader) => leader.uniqueId)).toEqual(['bob', 'alice']);

      // Rejoining the same room does not clear authoritative session totals.
      harness.emit(TOPIC.liveConnected, { uniqueId: 'creator', roomId: 'room-a' } as never);
      expect(live.likeLeaders.value).toHaveLength(2);
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
      harness.emit('live.likes-ranking', {
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

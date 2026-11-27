import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { ControlClient, GapHandler, RpcParams } from '@/shared/api/control-client.ts';
import type { RankingDefinition } from '../../../../../../src/web/entities/ranking/types.ts';
import { createFeedRankings, feedRankingDefinition, FEED_RANKING_IDS } from '../../../../../../src/web/entities/ranking/model/feed-rankings.ts';
import { EVENT_KIND, FIXTURE_ID, RPC, TOPIC, defined } from '../../../../../shared/index.ts';
import { answer } from '../../../../support/answer.ts';
import { isRecord, propertyOf } from '../../../../support/json.ts';

function isRankingDefinition(value: unknown): value is RankingDefinition {
  return isRecord(value)
    && value['schemaVersion'] === 1
    && typeof value['id'] === 'string'
    && typeof value['creatorId'] === 'string'
    && typeof value['roomId'] === 'string'
    && Array.isArray(value['sources']);
}

export function rankingSnapshot(rankingId: string, creatorId: string, roomId: string, scores: number[], revision = 1) {
  return {
    rankingId, schemaVersion: 1, creatorId, roomId, revision, integrityStatus: 'unknown', paused: false,
    rejected: 0, dropped: 0, limited: 0, cooldownRejected: 0,
    entries: scores.map((score, index) => ({
      key: `u${index}`, rank: index + 1, score, label: `User ${index}`, uniqueId: `user${index}`, nickname: `User ${index}`,
    })),
    totalScore: scores.reduce((sum, score) => sum + score, 0),
    totalParticipants: scores.length,
  };
}

function fakeHost(options: { fail?: boolean } = {}) {
  const definitions = new Map<string, RankingDefinition>();
  const calls: Array<{
    method: string;
    params: RpcParams;
  }> = [];
  const topics = new Map<string, Set<(data: unknown) => void>>();
  const gaps = new Set<GapHandler>();
  const scores: Record<string, number[]> = {};
  const reply = async (method: string, params: RpcParams): Promise<unknown> => {
    if (options.fail) throw new Error('rankings unavailable');
    if (method === RPC.rankingsList) return { rankings: [...definitions.values()] };
    if (method === RPC.rankingsCreate) {
      if (!isRankingDefinition(params)) throw new Error('rankings.create expects a ranking definition');
      definitions.set(params.id, params);
      return { ok: true };
    }
    if (method === 'rankings.update') {
      const definition = params['definition'];
      if (!isRankingDefinition(definition)) throw new Error('rankings.update expects a ranking definition');
      definitions.set(definition.id, definition);
      return { ok: true };
    }
    if (method === RPC.rankingsSnapshot) {
      const definition = defined(definitions.get(String(params['id'])));
      return rankingSnapshot(definition.id, definition.creatorId, definition.roomId, scores[definition.id] ?? []);
    }
    throw new Error(`unexpected ${method}`);
  };
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: async <T>(method: string, params: RpcParams = {}): Promise<T> => {
      calls.push({ method, params });
      return answer<T>(await reply(method, params));
    },
    onTopic: <T>(topic: string, handler: (data: T) => void) => {
      const set = topics.get(topic) ?? new Set();
      const listener = (data: unknown) => Reflect.apply(handler, undefined, [data]);
      set.add(listener);
      topics.set(topic, set);
      return () => set.delete(listener);
    },
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: (handler: GapHandler) => {
      gaps.add(handler);
      return () => gaps.delete(handler);
    },
  };
  const emit = (topic: string, data: unknown) => topics.get(topic)?.forEach((handler) => handler(data));
  return { client, calls, definitions, scores, emit, gap: () => gaps.forEach((handler) => handler(0)) };
}

describe('feed rankings', () => {
  it('creates both session rankings for the connected room and loads their snapshots', async () => {
    const host = fakeHost();
    host.scores[FEED_RANKING_IDS.top] = [500, 20];
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', FIXTURE_ID.room1);
    await flushPromises();

    expect(host.definitions.get(FEED_RANKING_IDS.top)).toEqual(feedRankingDefinition('top', 'luna', FIXTURE_ID.room1));
    expect(host.definitions.get(FEED_RANKING_IDS.likes)?.sources).toEqual([
      { source: EVENT_KIND.tiktokLike, metric: 'like.count', weight: 1, enabled: true },
    ]);
    expect(rankings.rankingsAvailable.value).toBe(true);
    expect(rankings.topRanking.value?.entries.map((entry) => entry.score)).toEqual([500, 20]);
    expect(rankings.likesRanking.value?.entries).toEqual([]);
    rankings.dispose();
  });

  it('applies pushes for the current room only', async () => {
    const host = fakeHost();
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', FIXTURE_ID.room1);
    host.emit(TOPIC.rankingUpdated, { snapshot: rankingSnapshot(FEED_RANKING_IDS.likes, 'luna', FIXTURE_ID.room1, [9], 2) });
    expect(rankings.likesRanking.value?.entries[0]?.score).toBe(9);
    host.emit(TOPIC.rankingUpdated, { snapshot: rankingSnapshot(FEED_RANKING_IDS.likes, 'other', 'room-9', [99], 3) });
    expect(rankings.likesRanking.value).toBeNull();
    rankings.dispose();
  });

  it('retargets the same ids with a reset on room change and skips unchanged reconnects', async () => {
    const host = fakeHost();
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', FIXTURE_ID.room1);
    host.calls.length = 0;

    await rankings.connect('luna', FIXTURE_ID.room1);
    expect(host.calls.map((call) => call.method)).toEqual([RPC.rankingsList, RPC.rankingsSnapshot, RPC.rankingsSnapshot]);

    host.calls.length = 0;
    await rankings.connect('luna', 'room-2');
    const updates = host.calls.filter((call) => call.method === 'rankings.update');
    expect(updates).toHaveLength(2);
    expect(updates.every((call) => propertyOf(call.params, 'reset') === true)).toBe(true);
    expect(host.definitions.size).toBe(2);
    expect(host.definitions.get(FEED_RANKING_IDS.top)?.roomId).toBe('room-2');
    rankings.dispose();
  });

  it('reports unavailable when the host refuses, so the feed can fall back', async () => {
    const host = fakeHost({ fail: true });
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', FIXTURE_ID.room1);
    expect(rankings.rankingsAvailable.value).toBe(false);
    expect(rankings.topRanking.value).toBeNull();
    expect(rankings.likesRanking.value).toBeNull();
    rankings.dispose();
  });

  it('reloads snapshots after a delivery gap', async () => {
    const host = fakeHost();
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', FIXTURE_ID.room1);
    host.scores[FEED_RANKING_IDS.top] = [42];
    host.gap();
    await flushPromises();
    expect(rankings.topRanking.value?.entries[0]?.score).toBe(42);
    rankings.dispose();
  });
});

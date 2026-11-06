import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { ControlClient, GapHandler, TopicHandler } from '@/shared/api/control-client.ts';
import type { RankingDefinition } from '../types.ts';
import { createFeedRankings, feedRankingDefinition, FEED_RANKING_IDS } from './feed-rankings.ts';

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
  const calls: Array<{ method: string; params: unknown }> = [];
  const topics = new Map<string, Set<TopicHandler>>();
  const gaps = new Set<GapHandler>();
  const scores: Record<string, number[]> = {};
  const client = {
    call: async (method: string, params: Record<string, unknown> = {}) => {
      calls.push({ method, params });
      if (options.fail) throw new Error('rankings unavailable');
      if (method === 'rankings.list') return { rankings: [...definitions.values()] };
      if (method === 'rankings.create') { definitions.set(String(params['id']), params as unknown as RankingDefinition); return { ok: true }; }
      if (method === 'rankings.update') {
        const definition = params['definition'] as RankingDefinition;
        definitions.set(definition.id, definition);
        return { ok: true };
      }
      if (method === 'rankings.snapshot') {
        const definition = definitions.get(String(params['id']))!;
        return rankingSnapshot(definition.id, definition.creatorId, definition.roomId, scores[definition.id] ?? []);
      }
      throw new Error(`unexpected ${method}`);
    },
    onTopic: (topic: string, handler: TopicHandler) => {
      const set = topics.get(topic) ?? new Set();
      set.add(handler);
      topics.set(topic, set);
      return () => set.delete(handler);
    },
    onGap: (handler: GapHandler) => { gaps.add(handler); return () => gaps.delete(handler); },
  } as unknown as ControlClient;
  const emit = (topic: string, data: unknown) => topics.get(topic)?.forEach((handler) => handler(data));
  return { client, calls, definitions, scores, emit, gap: () => gaps.forEach((handler) => handler({} as never)) };
}

describe('feed rankings', () => {
  it('creates both session rankings for the connected room and loads their snapshots', async () => {
    const host = fakeHost();
    host.scores[FEED_RANKING_IDS.top] = [500, 20];
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', 'room-1');
    await flushPromises();

    expect(host.definitions.get(FEED_RANKING_IDS.top)).toEqual(feedRankingDefinition('top', 'luna', 'room-1'));
    expect(host.definitions.get(FEED_RANKING_IDS.likes)?.sources).toEqual([
      { source: 'tiktok.like', metric: 'like.count', weight: 1, enabled: true },
    ]);
    expect(rankings.rankingsAvailable.value).toBe(true);
    expect(rankings.topRanking.value?.entries.map((entry) => entry.score)).toEqual([500, 20]);
    expect(rankings.likesRanking.value?.entries).toEqual([]);
    rankings.dispose();
  });

  it('applies pushes for the current room only', async () => {
    const host = fakeHost();
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', 'room-1');
    host.emit('ranking.updated', { snapshot: rankingSnapshot(FEED_RANKING_IDS.likes, 'luna', 'room-1', [9], 2) });
    expect(rankings.likesRanking.value?.entries[0]?.score).toBe(9);
    host.emit('ranking.updated', { snapshot: rankingSnapshot(FEED_RANKING_IDS.likes, 'other', 'room-9', [99], 3) });
    expect(rankings.likesRanking.value).toBeNull();
    rankings.dispose();
  });

  it('retargets the same ids with a reset on room change and skips unchanged reconnects', async () => {
    const host = fakeHost();
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', 'room-1');
    host.calls.length = 0;

    await rankings.connect('luna', 'room-1');
    expect(host.calls.map((call) => call.method)).toEqual(['rankings.list', 'rankings.snapshot', 'rankings.snapshot']);

    host.calls.length = 0;
    await rankings.connect('luna', 'room-2');
    const updates = host.calls.filter((call) => call.method === 'rankings.update');
    expect(updates).toHaveLength(2);
    expect(updates.every((call) => (call.params as { reset: boolean }).reset)).toBe(true);
    expect(host.definitions.size).toBe(2);
    expect(host.definitions.get(FEED_RANKING_IDS.top)?.roomId).toBe('room-2');
    rankings.dispose();
  });

  it('reports unavailable when the host refuses, so the feed can fall back', async () => {
    const host = fakeHost({ fail: true });
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', 'room-1');
    expect(rankings.rankingsAvailable.value).toBe(false);
    expect(rankings.topRanking.value).toBeNull();
    expect(rankings.likesRanking.value).toBeNull();
    rankings.dispose();
  });

  it('reloads snapshots after a delivery gap', async () => {
    const host = fakeHost();
    const rankings = createFeedRankings(host.client);
    await rankings.connect('luna', 'room-1');
    host.scores[FEED_RANKING_IDS.top] = [42];
    host.gap();
    await flushPromises();
    expect(rankings.topRanking.value?.entries[0]?.score).toBe(42);
    rankings.dispose();
  });
});

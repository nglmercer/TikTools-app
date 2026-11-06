/**
 * Built-in rankings that back the Live feed inspector ("General" and
 * "Likes"). They are ordinary host rankings, so scores come from the
 * Rust ranking engine and never from UI re-aggregation.
 *
 * The feed owns two fixed ids. On each `live.connected` they are created,
 * or retargeted (with a reset) to the new creator/room, so they never pile
 * up one pair per room.
 */
import { shallowRef } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { RankingController, type RankingSnapshot } from '../../../../widgets/shared/ranking-controller.ts';
import type { RankingDefinition } from '../types.ts';

export const FEED_RANKING_IDS = {
  top: 'feed-top-gifters',
  likes: 'feed-top-likes',
} as const;

export type FeedRankingKind = keyof typeof FEED_RANKING_IDS;

const SOURCES: Record<FeedRankingKind, { source: string; metric: string }> = {
  top: { source: 'tiktok.gift', metric: 'gift.diamonds' },
  likes: { source: 'tiktok.like', metric: 'like.count' },
};

/** Entries shown in the inspector; the engine tracks up to `capacity` users. */
export const FEED_RANKING_LIMIT = 10;
const FEED_RANKING_CAPACITY = 5_000;

export function feedRankingDefinition(kind: FeedRankingKind, creatorId: string, roomId: string): RankingDefinition {
  return {
    schemaVersion: 1,
    id: FEED_RANKING_IDS[kind],
    creatorId,
    roomId,
    groupBy: 'user',
    window: { type: 'session' },
    sources: [{ ...SOURCES[kind], weight: 1, enabled: true }],
    limit: FEED_RANKING_LIMIT,
    capacity: FEED_RANKING_CAPACITY,
  };
}

function sameSources(current: RankingDefinition, wanted: RankingDefinition): boolean {
  return current.groupBy === wanted.groupBy
    && current.window.type === wanted.window.type
    && current.limit === wanted.limit
    && current.sources.length === wanted.sources.length
    && current.sources.every((source, index) => {
      const other = wanted.sources[index]!;
      return source.source === other.source && source.metric === other.metric
        && source.weight === other.weight && source.enabled === other.enabled;
    });
}

export function createFeedRankings(control: ControlClient) {
  const kinds = Object.keys(FEED_RANKING_IDS) as FeedRankingKind[];
  const snapshots = {
    top: shallowRef<RankingSnapshot | null>(null),
    likes: shallowRef<RankingSnapshot | null>(null),
  };
  /** False when the host refused to provision; the feed then falls back. */
  const available = shallowRef(false);
  const controllers = Object.fromEntries(
    kinds.map((kind) => [kind, new RankingController(FEED_RANKING_IDS[kind], (snapshot) => {
      // Never show a snapshot from another room while switching.
      snapshots[kind].value = snapshot && snapshot.creatorId === scope.creatorId && snapshot.roomId === scope.roomId
        ? snapshot
        : null;
    })]),
  ) as Record<FeedRankingKind, RankingController>;
  let scope = { creatorId: '', roomId: '' };
  let generation = 0;
  let disposed = false;

  async function load(kind: FeedRankingKind, token: number): Promise<void> {
    const result = await control.call<unknown>('rankings.snapshot', { id: FEED_RANKING_IDS[kind] });
    if (disposed || token !== generation) return;
    controllers[kind].handleEnvelope({ topic: 'ranking.updated', data: { snapshot: result } });
  }

  async function provision(token: number): Promise<void> {
    const { rankings } = await control.call<{ rankings: RankingDefinition[] }>('rankings.list', {});
    for (const kind of kinds) {
      if (disposed || token !== generation) return;
      const wanted = feedRankingDefinition(kind, scope.creatorId, scope.roomId);
      const current = rankings.find((ranking) => ranking.id === wanted.id);
      if (!current) {
        await control.call('rankings.create', { ...wanted });
      } else if (current.creatorId !== wanted.creatorId || current.roomId !== wanted.roomId) {
        await control.call('rankings.update', { definition: wanted, reset: true });
      } else if (!sameSources(current, wanted)) {
        await control.call('rankings.update', { definition: wanted, reset: false });
      }
    }
  }

  /** Points both rankings at the connected room and loads their snapshots. */
  async function connect(creatorId: string, roomId: string): Promise<void> {
    if (!creatorId || !roomId) return;
    const changed = creatorId !== scope.creatorId || roomId !== scope.roomId;
    scope = { creatorId, roomId };
    const token = ++generation;
    if (changed) for (const kind of kinds) controllers[kind].clear();
    try {
      await provision(token);
      if (disposed || token !== generation) return;
      available.value = true;
      await Promise.all(kinds.map((kind) => load(kind, token)));
    } catch {
      if (!disposed && token === generation) available.value = false;
    }
  }

  const unsubscribers = [
    control.onTopic('ranking.updated', (data) => {
      for (const kind of kinds) controllers[kind].handleEnvelope({ topic: 'ranking.updated', data });
    }),
    control.onGap(() => {
      if (!scope.roomId || !available.value) return;
      const token = generation;
      // Revisions may not advance across a gap; accept the reloaded state.
      for (const kind of kinds) {
        controllers[kind].clear();
        void load(kind, token).catch(() => undefined);
      }
    }),
  ];

  function dispose(): void {
    disposed = true;
    for (const off of unsubscribers) off();
    for (const kind of kinds) controllers[kind].dispose();
  }

  return { topRanking: snapshots.top, likesRanking: snapshots.likes, rankingsAvailable: available, connect, dispose };
}

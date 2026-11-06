import type { LikeLeaderPayload, TopViewerPayload, ViewerRecord } from '@/types.ts';
import type { RankingEntry, RankingSnapshot } from '../../../../widgets/shared/ranking-controller.ts';

export interface RankedViewerEntry {
  key: string;
  rank: number;
  uniqueId: string;
  nickname?: string;
  avatarUrl?: string;
  score: number;
  provenance: 'ranking' | 'tiktok_room' | 'loyalty_points';
}

/** Host ranking entries grouped by user, in the shape of the like list. */
export function rankingLeaders(snapshot: RankingSnapshot | null): LikeLeaderPayload[] {
  return (snapshot?.entries ?? []).filter((entry) => entry.score > 0).map(rankingLeader);
}

function rankingLeader(entry: RankingEntry): LikeLeaderPayload {
  return {
    rank: entry.rank,
    score: entry.score,
    uniqueId: entry.uniqueId ?? entry.label,
    nickname: entry.nickname ?? entry.label,
    avatarUrl: entry.avatarUrl,
    userId: entry.userId ?? entry.key,
  };
}

/**
 * The feed ranking is authoritative once it has entries. Until then (no
 * gifts yet, or not connected) show TikTok's room list, then loyalty points.
 */
export function rankedViewers(
  topViewers: readonly TopViewerPayload[],
  leaderboard: readonly ViewerRecord[],
  ranking: RankingSnapshot | null = null,
): RankedViewerEntry[] {
  const ranked = rankingLeaders(ranking);
  if (ranked.length) {
    return ranked.map((leader) => ({
      key: leader.userId,
      rank: leader.rank,
      uniqueId: leader.uniqueId,
      nickname: leader.nickname,
      avatarUrl: leader.avatarUrl,
      score: leader.score,
      provenance: 'ranking',
    }));
  }
  if (topViewers.length) {
    return topViewers.slice(0, 6).map((viewer, index) => ({
      key: viewer.uniqueId,
      rank: viewer.rank || index + 1,
      uniqueId: viewer.uniqueId,
      nickname: viewer.nickname,
      avatarUrl: viewer.avatarUrl,
      score: viewer.score,
      provenance: 'tiktok_room',
    }));
  }
  return leaderboard.slice(0, 3).map((viewer, index) => ({
    key: viewer.uniqueId,
    rank: index + 1,
    uniqueId: viewer.uniqueId,
    nickname: viewer.nickname,
    avatarUrl: viewer.avatarUrl,
    score: viewer.points,
    provenance: 'loyalty_points',
  }));
}

import { describe, expect, it } from 'vitest';
import type { ViewerRecord } from '../../../../../../src/shared/messages.ts';
import { filterViewers, rankViewers, sortViewers } from '../../../../../../src/web/entities/points/lib/leaderboard.ts';

function viewer(uniqueId: string, points: number, level: number, nickname?: string): ViewerRecord {
  return {
    uniqueId,
    nickname,
    points,
    level,
    isSubscriber: false,
    totalChats: 0,
    totalCoins: 0,
    totalLikes: 0,
    totalShares: 0,
    firstSeen: 0,
    lastSeen: 0,
  };
}

const ranked = rankViewers([viewer('luna', 300, 3, 'Luna'), viewer('mango', 200, 4), viewer('kiwi', 100, 1, 'Mango fan')]);

describe('leaderboard', () => {
  it('ranks in board order', () => {
    expect(ranked.map((row) => row.rank)).toEqual([1, 2, 3]);
  });

  it('matches handles and nicknames', () => {
    expect(filterViewers(ranked, '@MANGO').map((row) => row.uniqueId)).toEqual(['mango', 'kiwi']);
    expect(filterViewers(ranked, '')).toHaveLength(3);
  });

  it('sorts by level, points and handle', () => {
    expect(sortViewers(ranked, 'level', 'desc').map((row) => row.uniqueId)).toEqual(['mango', 'luna', 'kiwi']);
    expect(sortViewers(ranked, 'points', 'asc').map((row) => row.uniqueId)).toEqual(['kiwi', 'mango', 'luna']);
    expect(sortViewers(ranked, 'uniqueId', 'asc').map((row) => row.uniqueId)).toEqual(['kiwi', 'luna', 'mango']);
  });
});

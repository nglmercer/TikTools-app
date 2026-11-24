import { describe, expect, it } from 'vitest';
import type { AnalyticsTopViewer } from '../../../../../../src/shared/messages.ts';
import { filterContributors, rankContributors, sortContributors } from '../../../../../../src/web/entities/analytics/lib/contributors.ts';

function viewer(uniqueId: string, chats: number): AnalyticsTopViewer {
  return { uniqueId, chats, gifts: 0, diamonds: 0, likes: 0, shares: 0, interactions: chats, lastSeen: 0 };
}

const ranked = rankContributors([viewer('luna', 5), viewer('Mango', 9), viewer('zeta', 5)]);

describe('contributors', () => {
  it('ranks in host order', () => {
    expect(ranked.map((row) => [row.uniqueId, row.rank])).toEqual([
      ['luna', 1],
      ['Mango', 2],
      ['zeta', 3],
    ]);
  });

  it('filters case-insensitively and ignores a leading @', () => {
    expect(filterContributors(ranked, '@man').map((row) => row.uniqueId)).toEqual(['Mango']);
    expect(filterContributors(ranked, '  ')).toHaveLength(3);
  });

  it('sorts by numeric and text columns with rank as tie-breaker', () => {
    expect(sortContributors(ranked, 'chats', 'desc').map((row) => row.uniqueId)).toEqual(['Mango', 'luna', 'zeta']);
    expect(sortContributors(ranked, 'uniqueId', 'asc').map((row) => row.uniqueId)).toEqual(['luna', 'Mango', 'zeta']);
  });
});

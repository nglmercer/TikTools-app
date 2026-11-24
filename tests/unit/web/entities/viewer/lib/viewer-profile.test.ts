import { describe, expect, it } from 'vitest';
import type { TopViewerPayload, ViewerRecord } from '@/types.ts';
import { avatarColor, avatarInitials } from '../../../../../../src/web/entities/viewer/lib/avatar-color.ts';
import { rankedViewers } from '../../../../../../src/web/entities/viewer/lib/top-viewers.ts';
import { enrichViewer, sameHandle } from '../../../../../../src/web/entities/viewer/lib/viewer-profile.ts';

const record: ViewerRecord = {
  uniqueId: 'luna',
  nickname: 'Luna',
  avatarUrl: 'https://cdn/luna.webp',
  points: 420,
  level: 5,
  isSubscriber: true,
  totalChats: 12,
  totalCoins: 30,
  totalLikes: 900,
  totalShares: 2,
  firstSeen: 1,
  lastSeen: 2,
};

describe('viewer profile', () => {
  it('fills tracked stats from the leaderboard and keeps event identity first', () => {
    expect(enrichViewer({ uniqueId: 'luna', nickname: 'Luna ✨' }, [record])).toMatchObject({
      nickname: 'Luna ✨',
      avatarUrl: 'https://cdn/luna.webp',
      points: 420,
      totalChats: 12,
    });
  });

  it('leaves untracked viewers without fabricated stats', () => {
    expect(enrichViewer({ uniqueId: 'ghost' }, [record]).points).toBeUndefined();
  });

  it('compares handles ignoring @ and case', () => {
    expect(sameHandle('@Luna', 'luna ')).toBe(true);
  });

  it('derives stable initials and colors', () => {
    expect(avatarInitials('@luna')).toBe('LU');
    expect(avatarColor('luna')).toBe(avatarColor('luna'));
  });
});

describe('rankedViewers', () => {
  it('prefers native TikTok ranking and falls back to the points top 3', () => {
    const native: TopViewerPayload[] = [{ rank: 1, score: 50, delta: 0, uniqueId: 'a', nickname: 'A', userId: '1' }];
    expect(rankedViewers(native, [record])).toEqual([expect.objectContaining({ uniqueId: 'a', score: 50, provenance: 'tiktok_room' })]);
    expect(rankedViewers([], [record])).toEqual([
      expect.objectContaining({ uniqueId: 'luna', rank: 1, score: 420, provenance: 'loyalty_points' }),
    ]);
  });
});

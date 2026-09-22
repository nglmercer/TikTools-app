<script lang="tsx">
import { t, type Locale } from '../i18n.ts';
import type { TopViewerPayload, ViewerRecord } from '../types.ts';
import { UserAvatar } from './user-avatar.vue';

type Props = {
  locale: Locale;
  // Native TikTok ranking (Contributor 0-5) — preferred
  topViewers?: TopViewerPayload[];
  // Fallback local points leaderboard
  leaderboard?: ViewerRecord[];
  // Live viewer count from WebcastRoomUserSeqMessage
  liveViewers?: number;
};

/**
 * Minimalist contributor list: plain rows with a hairline separator, no
 * pill backgrounds or gold/silver/bronze fills. Only the rank numeral keeps
 * a subtle tint for the top 3. The collapsible panel header (title, count,
 * toggle) lives in FeedView so this component stays a pure list that can
 * also render inline without chrome.
 */
export function TopViewersRibbon({ locale, topViewers = [], leaderboard = [], liveViewers = 0 }: Props) {
  const hasNative = topViewers.length > 0;
  // Native top 0-5, fallback to points top 3
  const ribbonItems: Array<{
    key: string;
    rank: number;
    uniqueId: string;
    nickname?: string;
    avatarUrl?: string;
    score: number;
  }> = hasNative
    ? topViewers.slice(0, 6).map((v) => ({
        key: v.uniqueId,
        rank: v.rank || 1,
        uniqueId: v.uniqueId,
        nickname: v.nickname,
        avatarUrl: v.avatarUrl,
        score: v.score,
      }))
    : leaderboard.slice(0, 3).map((v, idx) => ({
        key: v.uniqueId,
        rank: idx + 1,
        uniqueId: v.uniqueId,
        nickname: v.nickname,
        avatarUrl: v.avatarUrl,
        score: v.points,
      }));

  void liveViewers;

  if (ribbonItems.length === 0) {
    return <p class="tt-viewers-empty">{t(locale, 'noContributors')}</p>;
  }

  return (
    <div class="tt-viewers-list" role="list">
      {ribbonItems.map((viewer, idx) => {
        const rankNum = hasNative ? viewer.rank || idx + 1 : idx + 1;
        const displayRank = rankNum === 0 ? '–' : String(rankNum);
        const scoreLabel = viewer.score > 0 ? String(viewer.score) : hasNative ? '' : String(viewer.score);
        const rankTone = rankNum <= 3 ? ` rank-${rankNum}` : '';
        return (
          <div key={viewer.key} role="listitem" class={`tt-viewer-row${rankTone}`}>
            <span class="tt-rank-num">{displayRank}</span>
            <UserAvatar
              uniqueId={viewer.uniqueId}
              nickname={viewer.nickname}
              avatarUrl={viewer.avatarUrl}
              imgClass="tt-chip-avatar"
              fallbackClass="tt-chip-avatar fallback"
            />
            <span class="tt-rank-name" title={viewer.nickname || viewer.uniqueId}>
              {viewer.nickname && viewer.nickname !== viewer.uniqueId ? viewer.nickname : `@${viewer.uniqueId}`}
            </span>
            {scoreLabel ? <span class="tt-rank-pts">{scoreLabel}</span> : null}
          </div>
        );
      })}
    </div>
  );
}

export default TopViewersRibbon;
</script>

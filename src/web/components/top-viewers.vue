<script lang="tsx">
import { t, type Locale } from '../i18n.ts';
import type { TopViewerPayload, ViewerRecord } from '../types.ts';
import { Tooltip } from './ui/Tooltip.vue';

type Props = {
  locale: Locale;
  // Native TikTok ranking (Contributor 0-5) — preferred
  topViewers?: TopViewerPayload[];
  // Fallback local points leaderboard
  leaderboard?: ViewerRecord[];
  // Live viewer count from WebcastRoomUserSeqMessage
  liveViewers?: number;
};

function getInitials(name: string): string {
  return name.slice(0, 2).toUpperCase();
}

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

  if (ribbonItems.length === 0) return null;

  const totalLabel = liveViewers > 0 ? liveViewers : hasNative ? topViewers.length : leaderboard.length;

  return (
    <div class="tt-viewers-ribbon">
      <Tooltip text={t(locale, 'topContributors')} position="bottom">
        <div class="tt-ribbon-title">
          <span>
            {t(locale, 'viewersCount')} · {totalLabel}
          </span>
          <span style={{ opacity: 0.6, fontSize: '11px' }} title="TikTok native ranking">ⓘ</span>
        </div>
      </Tooltip>
      <div class="tt-top-contributors">
        {ribbonItems.map((viewer, idx) => {
          const rankNum = hasNative ? viewer.rank || idx + 1 : idx + 1;
          const displayRank = rankNum === 0 ? '–' : String(rankNum);
          const scoreLabel = viewer.score > 0 ? String(viewer.score) : hasNative ? '' : String(viewer.score);
          return (
            <div key={viewer.key} class={`tt-contributor-chip rank-${rankNum <= 3 ? rankNum : 'other'}`}>
              <span class="tt-rank-num">{displayRank}</span>
              {viewer.avatarUrl ? (
                <img
                  src={viewer.avatarUrl}
                  alt={viewer.uniqueId}
                  class="tt-chip-avatar"
                  loading="lazy"
                  onError={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')}
                />
              ) : (
                <span class="tt-chip-avatar fallback" aria-hidden>
                  {getInitials(viewer.uniqueId)}
                </span>
              )}
              <span class="tt-rank-name" title={viewer.nickname || viewer.uniqueId}>
                {viewer.nickname && viewer.nickname !== viewer.uniqueId ? viewer.nickname : `@${viewer.uniqueId}`}
              </span>
              {scoreLabel ? <span class="tt-rank-pts">{scoreLabel}</span> : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default TopViewersRibbon;
</script>

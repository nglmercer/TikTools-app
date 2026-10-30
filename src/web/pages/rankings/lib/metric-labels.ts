import type { RankingDefinition } from '@/entities/ranking/types.ts';

/** i18n keys for native metrics; plugin and custom metrics keep their technical name. */
const metricNames: Record<string, string> = {
  'tiktok.like|like.count': 'likes', 'tiktok.like|event.count': 'likePackets',
  'tiktok.gift|gift.diamonds': 'giftDiamonds', 'tiktok.gift|gift.count': 'giftCount', 'tiktok.gift|event.count': 'giftPackets',
  'tiktok.share|share.count': 'shares', 'tiktok.share|event.count': 'shareEvents',
  'tiktok.follow|event.count': 'follows', 'tiktok.social|event.count': 'social', 'tiktok.chat|event.count': 'chat',
  'tiktok.join|event.count': 'joins', 'points.awarded|points.delta': 'points', 'competition.vote|event.count': 'votes',
  'plugin.event|event.count': 'pluginEvents', 'plugin.event|value': 'pluginValue',
};

/** Quick-add sources offered first in the ranking editor. */
export const presetMetrics = ['tiktok.gift|gift.diamonds', 'tiktok.like|like.count', 'tiktok.share|share.count', 'tiktok.follow|event.count', 'tiktok.chat|event.count', 'points.awarded|points.delta'];

type Translate = (key: string, values?: Record<string, unknown>) => string;

export function metricLabel(t: Translate, source: string, metric: string): string {
  const name = metricNames[`${source}|${metric}`];
  return name ? t(`rankingsPage.metrics.${name}`) : `${source} · ${metric}`;
}

export function rankingSummary(t: Translate, definition: RankingDefinition): string {
  const window = definition.window;
  const period = window.type === 'rolling' ? t('rankingsPage.windows.rolling', { seconds: Math.round(window.durationMs / 1000) }) : t(`rankingsPage.windows.${window.type}`);
  const sources = definition.sources.map((source) => metricLabel(t, source.source, source.metric)).join(' + ');
  return `${sources} · ${t(`rankingsPage.groups.${definition.groupBy}`)} · ${period}`;
}

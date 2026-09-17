<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import { IconBarChart, IconChat, IconCoins, IconGift, IconHeart, IconUsers } from '../icons.vue';
import { StatCard, StatGrid } from '../ui/Page.vue';
import type { AnalyticsTotals } from '../../../shared/messages.ts';
import { formatCount } from './analytics-range.ts';

type AnalyticsStatsGridProps = {
  locale: Locale;
  totals: AnalyticsTotals;
  sessions: number;
};

/** Six KPI tiles. Thin wrapper over the shared StatGrid/StatCard primitives. */
export const AnalyticsStatsGrid = defineVueFunctional<AnalyticsStatsGridProps>((props) => {
  const { locale, totals, sessions } = props;
  return (
    <StatGrid>
      <StatCard icon={<IconChat />} value={formatCount(totals.chats, locale)} label={t(locale, 'analyticsChats')} tone="cyan" />
      <StatCard icon={<IconGift />} value={formatCount(totals.gifts, locale)} label={t(locale, 'analyticsGifts')} tone="pink" />
      <StatCard icon={<IconHeart />} value={formatCount(totals.likes, locale)} label={t(locale, 'analyticsLikes')} tone="yellow" />
      <StatCard icon={<IconUsers />} value={formatCount(totals.peakViewers, locale)} label={t(locale, 'analyticsPeakViewers')} tone="green" />
      <StatCard icon={<IconCoins />} value={formatCount(totals.diamonds, locale)} label={t(locale, 'analyticsDiamonds')} tone="cyan" />
      <StatCard icon={<IconBarChart />} value={formatCount(sessions, locale)} label={t(locale, 'analyticsSessions')} tone="pink" />
    </StatGrid>
  );
});

export default AnalyticsStatsGrid;
</script>

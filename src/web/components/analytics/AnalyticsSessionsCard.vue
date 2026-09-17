<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import type { AnalyticsSummaryData } from '../../../shared/messages.ts';
import { IconUsers } from '../icons.vue';
import { Card } from '../ui/Card.vue';
import { formatCount } from './analytics-range.ts';

type AnalyticsSessionsCardProps = {
  locale: Locale;
  summary: AnalyticsSummaryData;
};

/** Sessions KPI list. Reuses the shared Card primitive. */
export const AnalyticsSessionsCard = defineVueFunctional<AnalyticsSessionsCardProps>((props) => {
  const { locale, summary } = props;
  const totals = summary.totals;
  const facts = [
    { label: t(locale, 'analyticsSessions'), value: formatCount(summary.sessions, locale) },
    { label: t(locale, 'analyticsPeakViewers'), value: formatCount(totals.peakViewers, locale) },
    { label: t(locale, 'analyticsGiftEvents'), value: formatCount(totals.giftEvents, locale) },
    { label: t(locale, 'analyticsLikeEvents'), value: formatCount(totals.likeEvents, locale) },
    { label: t(locale, 'analyticsFollows'), value: formatCount(totals.follows, locale) },
    { label: t(locale, 'analyticsShares'), value: formatCount(totals.shares, locale) },
    { label: t(locale, 'analyticsJoins'), value: formatCount(totals.joins, locale) },
  ];

  return (
    <Card title={t(locale, 'analyticsSessions')} icon={<IconUsers />}>
      <dl class="analytics-facts">
        {facts.map((fact) => (
          <div key={fact.label} class="analytics-facts__row">
            <dt>{fact.label}</dt>
            <dd>{fact.value}</dd>
          </div>
        ))}
      </dl>
    </Card>
  );
});

export default AnalyticsSessionsCard;
</script>

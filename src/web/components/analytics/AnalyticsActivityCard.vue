<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import type { AnalyticsSummaryData } from '../../../shared/messages.ts';
import { IconBarChart } from '../icons.vue';
import { Card, Chip, ChipGroup } from '../ui/Card.vue';
import { AnalyticsChart } from './AnalyticsChart.vue';
import { AnalyticsDayBreakdown } from './AnalyticsDayBreakdown.vue';
import { ANALYTICS_METRICS, isSingleDaySpan, type AnalyticsMetric } from './analytics-chart.ts';

type AnalyticsActivityCardProps = {
  locale: Locale;
  summary: AnalyticsSummaryData;
  metric: AnalyticsMetric;
  onMetricChange: (metric: AnalyticsMetric) => void;
};

const METRIC_LABELS: Record<AnalyticsMetric, 'analyticsChats' | 'analyticsGifts' | 'analyticsLikes' | 'analyticsDiamonds' | 'analyticsViewers'> = {
  chats: 'analyticsChats',
  gifts: 'analyticsGifts',
  likes: 'analyticsLikes',
  diamonds: 'analyticsDiamonds',
  peakViewers: 'analyticsViewers',
};

/**
 * Activity card that works for every range:
 * - multi-day spans render the trend line (AnalyticsChart),
 * - single-day spans (Today) render a per-metric breakdown
 *   (AnalyticsDayBreakdown) since a one-point line is meaningless.
 * Metric switching reuses the shared Chip/ChipGroup primitives.
 */
export const AnalyticsActivityCard = defineVueFunctional<AnalyticsActivityCardProps>((props) => {
  const { locale, summary, metric, onMetricChange } = props;
  const singleDay = isSingleDaySpan(summary.startDay, summary.endDay);

  return (
    <Card title={t(locale, 'analyticsActivity')} icon={<IconBarChart />}>
      <ChipGroup>
        {ANALYTICS_METRICS.map((key) => (
          <Chip key={key} active={metric === key} onClick={() => onMetricChange(key)}>
            {t(locale, METRIC_LABELS[key])}
          </Chip>
        ))}
      </ChipGroup>
      {singleDay ? (
        <AnalyticsDayBreakdown
          locale={locale}
          day={summary.startDay}
          totals={summary.totals}
          metric={metric}
          metricLabel={(key) => t(locale, METRIC_LABELS[key])}
          onMetricChange={onMetricChange}
        />
      ) : (
        <AnalyticsChart
          locale={locale}
          days={summary.days}
          startDay={summary.startDay}
          endDay={summary.endDay}
          metric={metric}
          label={t(locale, METRIC_LABELS[metric])}
        />
      )}
    </Card>
  );
});

export default AnalyticsActivityCard;
</script>
